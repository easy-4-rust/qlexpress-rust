//! try-catch 指令,对应 Java `com.alibaba.qlexpress4.runtime.instruction.TryCatchInstruction`。
//! 职责:异常捕获与 finally 处理。
//! 本文件由 `flow.rs` 拆分而来(SPEC §5.5 一类一文件),仅移动代码与补充中文注释,行为完全一致。

use crate::exception::error_codes;
use crate::exception::error_reporter::ErrorReporter;
use crate::exception::QLException;
use crate::ql_options::QLOptions;
use crate::runtime::instruction::QLInstruction;
use crate::runtime::member::ClassRef;
use crate::runtime::member::NativeRegistry;
use crate::runtime::opaque_native_object::OpaqueNativeObject;
use crate::runtime::q_result::QResult;
use crate::runtime::qcontext::QContext;
use crate::runtime::qlambda_definition::QLambdaDefinition;
use crate::runtime::util::throw_utils::wrap_throwable;
use crate::runtime::value::{DataValue, QValue};
use crate::utils::println_utils::PrintlnUtils;
use std::collections::HashSet;
use std::rc::Rc;

/// try-catch 指令。对应 Java: com.alibaba.qlexpress4.runtime.instruction.TryCatchInstruction(职责:异常捕获与 finally 处理)
/// Operation: try and catch throw element
/// Input: 0
/// Output: 1
///
/// Mirrors Java `TryCatchInstruction`. Catch entries are keyed by
/// [`ClassRef`] (Java `Class<?>`); matching follows Java
/// `Class.isAssignableFrom` for the built-in exception hierarchy plus
/// `java.lang.Object` (matches everything) — see Stage-3a notes.
pub struct TryCatchInstruction {
    error_reporter: Rc<dyn ErrorReporter>,
    body: Rc<dyn QLambdaDefinition>,
    exception_table: Vec<(ClassRef, Rc<dyn QLambdaDefinition>)>,
    /// nullable
    final_body: Option<Rc<dyn QLambdaDefinition>>,
    /// v1 新增:区分 block-expression `Continue(value)`(应传播值)
    /// 与 loop control `Continue/Break/Return`(应透传信号)。
    /// 当 `try/catch` 作为表达式使用时为 `true`,由
    /// [`crate::aparser::qvm_instruction_visitor`] 在 `parse_exception_table`
    /// 时根据 catch body 形态设置。
    is_expression_form: bool,
}

impl TryCatchInstruction {
    /// 为 QL 内核直接产生、尚未带 Java Throwable 的错误补齐可 catch 的
    /// 运行时异常对象。Java 版会保留原始 ArithmeticException/NPE；Rust 没有
    /// JVM throwable，因此在异常边界恢复其可观察的类型语义。
    fn inferred_catch_obj(error: &QLException) -> Option<DataValue> {
        let class_name = match error.error_code() {
            error_codes::INVALID_ARITHMETIC => "java.lang.ArithmeticException",
            error_codes::NULL_FIELD_ACCESS
            | error_codes::NULL_METHOD_ACCESS
            | error_codes::NULL_CALL => "java.lang.NullPointerException",
            _ => return None,
        };
        Some(OpaqueNativeObject::new(class_name).into_data_value())
    }

    /// 构造指令,对应 Java 构造器 `TryCatchInstruction`。
    pub fn new(
        error_reporter: Rc<dyn ErrorReporter>,
        body: Rc<dyn QLambdaDefinition>,
        exception_table: Vec<(ClassRef, Rc<dyn QLambdaDefinition>)>,
        final_body: Option<Rc<dyn QLambdaDefinition>>,
    ) -> Self {
        TryCatchInstruction {
            error_reporter,
            body,
            exception_table,
            final_body,
            is_expression_form: false,
        }
    }

    /// 标记 try/catch 用作表达式(`1 + try{...}catch{...}` 形式)。
    /// 此时 catch body 的 `Continue(value)` 是表达式结果值,不是循环控制信号。
    /// 对应 Java: com.alibaba.qlexpress4.runtime.instruction.TryCatchInstruction#withExpressionForm。
    pub fn with_expression_form(mut self, is_expr: bool) -> Self {
        self.is_expression_form = is_expr;
        self
    }

    /// 对应 Java 方法 `body`。
    pub fn body(&self) -> &Rc<dyn QLambdaDefinition> {
        &self.body
    }

    /// 对应 Java 方法 `exceptionTable`。
    pub fn exception_table(&self) -> &[(ClassRef, Rc<dyn QLambdaDefinition>)] {
        &self.exception_table
    }

    /// 对应 Java 方法 `finalBody`。
    pub fn final_body(&self) -> Option<&Rc<dyn QLambdaDefinition>> {
        self.final_body.as_ref()
    }

    /// Java `shouldExitTryCatch`.
    /// Java `shouldExitTryCatch`:仅传播循环控制信号(Return/Break/
    /// Continue(null))。Continue(non-null)是块表达式结果,
    /// 不应被当作循环控制信号透传(对齐 Java `LOOP_CONTINUE_RESULT`
    /// 哨兵引用相等语义)。
    fn should_exit_try_catch(result: &QResult) -> bool {
        match result {
            QResult::Return(_) | QResult::Break => true,
            QResult::Continue(v) => v.is_null(),
            _ => false,
        }
    }

    /// Java `getExceptionHandler(Class)`:
    /// `entry.getKey().isAssignableFrom(catchObjClass)`.
    fn get_exception_handler(
        &self,
        catch_obj: Option<&DataValue>,
        registry: &NativeRegistry,
    ) -> Option<&Rc<dyn QLambdaDefinition>> {
        let catch_type = match catch_obj {
            // Java substitutes `new Object()` for a null catch object.
            None => "java.lang.Object",
            // Java catch 按 Throwable 的实际运行时类匹配；宿主异常在 Rust
            // 中统一装入 `DataValue::Object`，其静态标签是 NativeObject，
            // 必须取显式注册的实际类名而不能用 data_type_name。
            Some(value) => &value.runtime_type_name(),
        };
        self.exception_table
            .iter()
            .find(|(clz, _)| class_assignable_from(clz, catch_type, registry))
            .map(|(_, handler)| handler)
    }

    /// Java `callExceptionHandler`.
    fn call_exception_handler(
        &self,
        catch_obj: Option<&DataValue>,
        q_context: &mut dyn QContext,
        ql_options: &QLOptions,
    ) -> Result<Option<QResult>, QLException> {
        let exception_handler = self.get_exception_handler(catch_obj, q_context.registry());
        let Some(handler) = exception_handler else {
            return Ok(None);
        };
        let catch_handler_lambda = Rc::clone(handler).to_lambda(q_context, ql_options, true);
        let arg = catch_obj.cloned().unwrap_or(DataValue::Null);
        match catch_handler_lambda.call(std::slice::from_ref(&arg)) {
            Ok(result) => Ok(Some(result)),
            Err(err) => Err(wrap_throwable(
                err,
                &*self.error_reporter,
                error_codes::EXECUTE_CATCH_HANDLER_ERROR,
                error_codes::error_msg(error_codes::EXECUTE_CATCH_HANDLER_ERROR),
                &[],
            )),
        }
    }

    /// Java `tryCatchResult`.
    fn try_catch_result(
        &self,
        q_context: &mut dyn QContext,
        ql_options: &QLOptions,
    ) -> Result<QResult, QLException> {
        let body_lambda = Rc::clone(&self.body).to_lambda(q_context, ql_options, true);
        match body_lambda.call(&[]) {
            Ok(result) => Ok(result),
            Err(err) => {
                let inferred_catch_obj = err
                    .catch_obj()
                    .cloned()
                    .or_else(|| Self::inferred_catch_obj(&err));
                let handled = self.call_exception_handler(
                    inferred_catch_obj.as_ref(),
                    q_context,
                    ql_options,
                )?;
                match handled {
                    Some(result) => Ok(result),
                    // Java: QLRuntimeException with no matching handler →
                    // rethrow as-is; other Throwables → EXECUTE_TRY_BLOCK_ERROR.
                    None => Err(err),
                }
            }
        }
    }

    /// Java `callFinal`.
    fn call_final(
        &self,
        final_body: &Rc<dyn QLambdaDefinition>,
        q_context: &mut dyn QContext,
        ql_options: &QLOptions,
    ) -> Result<(), QLException> {
        let final_lambda = Rc::clone(final_body).to_lambda(q_context, ql_options, true);
        final_lambda.call(&[]).map(|_| ()).map_err(|err| {
            wrap_throwable(
                err,
                &*self.error_reporter,
                error_codes::EXECUTE_FINAL_BLOCK_ERROR,
                error_codes::error_msg(error_codes::EXECUTE_FINAL_BLOCK_ERROR),
                &[],
            )
        })
    }
}

impl QLInstruction for TryCatchInstruction {
    /// 向下转型支持(供 api/parsecache Exporter 的 Java `instanceof` 分派)。
    fn as_any(&self) -> Option<&dyn std::any::Any> {
        Some(self)
    }

    fn execute(
        &self,
        q_context: &mut dyn QContext,
        ql_options: &QLOptions,
    ) -> Result<QResult, QLException> {
        let try_catch_result = self.try_catch_result(q_context, ql_options)?;

        // Java TryCatchInstruction.execute:始终压栈结果值,然后
        // 用 should_exit_try_catch 判断是否传播控制信号。
        // should_exit_try_catch 仅传播 Return / Break /
        // Continue(null)(循环控制哨兵)。Continue(non-null)
        // 是块表达式结果,不传播。
        let signal_to_propagate: Option<QResult> = if Self::should_exit_try_catch(&try_catch_result)
        {
            Some(try_catch_result.clone())
        } else {
            None
        };

        let result_value = try_catch_result.value();
        q_context.push(QValue::Data(result_value).to_immutable());

        if let Some(final_body) = &self.final_body {
            self.call_final(final_body, q_context, ql_options)?;
        }
        if let Some(sig) = signal_to_propagate {
            return Ok(sig);
        }
        Ok(QResult::NEXT_INSTRUCTION)
    }

    fn stack_input(&self) -> i32 {
        0
    }

    fn stack_output(&self) -> i32 {
        1
    }

    fn compiled_instruction_count(&self) -> usize {
        let handlers = self
            .exception_table
            .iter()
            .fold(0usize, |total, (_, definition)| {
                total.saturating_add(definition.compiled_instruction_count())
            });
        1usize
            .saturating_add(self.body.compiled_instruction_count())
            .saturating_add(handlers)
            .saturating_add(
                self.final_body
                    .as_ref()
                    .map_or(0, |definition| definition.compiled_instruction_count()),
            )
    }

    fn println(&self, index: usize, depth: usize, debug: &mut dyn FnMut(String)) {
        PrintlnUtils::println_by_cur_depth(depth as i32, &format!("{index}: TryCatch"), debug);
        PrintlnUtils::println_by_cur_depth(depth as i32 + 1, "Body", debug);
        self.body.println(depth + 2, debug);
        for (clz, handler) in &self.exception_table {
            PrintlnUtils::println_by_cur_depth(depth as i32 + 1, &clz.simple_name(), debug);
            handler.println(depth + 2, debug);
        }
        if let Some(final_body) = &self.final_body {
            PrintlnUtils::println_by_cur_depth(depth as i32 + 1, "Finally", debug);
            final_body.println(depth + 2, debug);
        }
    }

    fn error_reporter(&self) -> &Rc<dyn ErrorReporter> {
        &self.error_reporter
    }
}

/// Java `Class.isAssignableFrom` for the exception hierarchy used by catch
/// matching.
///
/// Walks [`NativeType::supertypes`] from `catch_type` upward through the
/// registry to check if it reaches `entry`. Falls back to the well-known
/// Java exception hierarchy for types not yet registered.
///
/// `java.lang.Object` is assignable from anything (universal catch-all).
fn class_assignable_from(entry: &ClassRef, catch_type: &str, registry: &NativeRegistry) -> bool {
    let entry_name = entry.java_name();
    if entry_name == catch_type || entry_name == "java.lang.Object" {
        return true;
    }
    // 优先走注册表 supertypes 链（真实类层次）。
    if is_assignable_via_registry(entry_name, catch_type, registry) {
        return true;
    }
    // 兜底:对注册表未覆盖的已知 Java 异常层次做精确枚举（非字串包含）。
    is_assignable_via_known_hierarchy(entry_name, catch_type)
}

/// 从 `from_type` 沿 supertypes 链向上遍历，检查是否能到达 `to_type`。
/// 使用 visited 集合防环。
fn is_assignable_via_registry(to_type: &str, from_type: &str, registry: &NativeRegistry) -> bool {
    let mut visited = HashSet::new();
    is_assignable_recursive(to_type, from_type, registry, &mut visited)
}

fn is_assignable_recursive(
    to_type: &str,
    current: &str,
    registry: &NativeRegistry,
    visited: &mut HashSet<String>,
) -> bool {
    if current == to_type {
        return true;
    }
    if !visited.insert(current.to_string()) {
        return false;
    }
    let Some(native_type) = registry.get_type(current) else {
        return false;
    };
    native_type
        .supertypes
        .iter()
        .any(|super_name| is_assignable_recursive(to_type, super_name, registry, visited))
}

/// 已知 Java 异常层次的精确枚举兜底（非字串包含匹配）。
/// 仅在注册表未覆盖时使用。
fn is_assignable_via_known_hierarchy(entry_name: &str, catch_type: &str) -> bool {
    // QLExpress 私有异常族。
    let ql_family = matches!(
        catch_type,
        "com.alibaba.qlexpress4.exception.QLException"
            | "com.alibaba.qlexpress4.exception.QLRuntimeException"
            | "com.alibaba.qlexpress4.exception.QLSyntaxException"
            | "com.alibaba.qlexpress4.exception.QLTimeoutException"
    );
    // Java RuntimeException 子族（精确枚举，不含字串匹配）。
    let java_runtime_sub = matches!(
        catch_type,
        "java.lang.RuntimeException"
            | "java.lang.ArithmeticException"
            | "java.lang.NullPointerException"
            | "java.lang.ClassCastException"
            | "java.lang.IllegalArgumentException"
            | "java.lang.IllegalStateException"
            | "java.lang.IndexOutOfBoundsException"
            | "java.lang.ArrayIndexOutOfBoundsException"
            | "java.lang.StringIndexOutOfBoundsException"
            | "java.lang.NumberFormatException"
            | "java.lang.SecurityException"
            | "java.lang.UnsupportedOperationException"
    ) || ql_family;
    // Java Exception 子族（含 RuntimeException 子族）。
    let java_exception_sub = java_runtime_sub
        || matches!(
            catch_type,
            "java.lang.Exception" | "java.lang.ReflectiveOperationException"
        );
    // Java Error 子族。
    let java_error_sub = matches!(
        catch_type,
        "java.lang.Error"
            | "java.lang.AssertionError"
            | "java.lang.LinkageError"
            | "java.lang.BootstrapMethodError"
            | "java.lang.ClassCircularityError"
            | "java.lang.ClassFormatError"
            | "java.lang.ExceptionInInitializerError"
            | "java.lang.IncompatibleClassChangeError"
            | "java.lang.AbstractMethodError"
            | "java.lang.IllegalAccessError"
            | "java.lang.InstantiationError"
            | "java.lang.NoClassDefFoundError"
            | "java.lang.NoSuchFieldError"
            | "java.lang.NoSuchMethodError"
            | "java.lang.UnsatisfiedLinkError"
            | "java.lang.VerifyError"
            | "java.lang.ThreadDeath"
            | "java.lang.VirtualMachineError"
            | "java.lang.InternalError"
            | "java.lang.OutOfMemoryError"
            | "java.lang.StackOverflowError"
            | "java.lang.UnknownError"
    );
    match entry_name {
        "java.lang.Throwable" => java_exception_sub || java_error_sub,
        "java.lang.Exception" => java_exception_sub,
        "java.lang.RuntimeException" => java_runtime_sub,
        "java.lang.Error" => java_error_sub,
        "com.alibaba.qlexpress4.exception.QLException" => ql_family,
        "com.alibaba.qlexpress4.exception.QLRuntimeException" => matches!(
            catch_type,
            "com.alibaba.qlexpress4.exception.QLRuntimeException"
                | "com.alibaba.qlexpress4.exception.QLTimeoutException"
        ),
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 辅助:构造一个带 supertypes 的 NativeRegistry 用于测试。
    fn test_registry() -> NativeRegistry {
        use crate::runtime::native_type::NativeType;
        let registry = NativeRegistry::new();
        // java.lang.Throwable -> Object
        let mut throwable = NativeType::named("java.lang.Throwable");
        throwable.supertypes = vec!["java.lang.Object".to_string()];
        registry.register_type(throwable);
        // java.lang.Exception -> Throwable
        let mut exception = NativeType::named("java.lang.Exception");
        exception.supertypes = vec!["java.lang.Throwable".to_string()];
        registry.register_type(exception);
        // java.lang.RuntimeException -> Exception
        let mut runtime_ex = NativeType::named("java.lang.RuntimeException");
        runtime_ex.supertypes = vec!["java.lang.Exception".to_string()];
        registry.register_type(runtime_ex);
        // java.lang.NullPointerException -> RuntimeException
        let mut npe = NativeType::named("java.lang.NullPointerException");
        npe.supertypes = vec!["java.lang.RuntimeException".to_string()];
        registry.register_type(npe);
        // java.lang.ArithmeticException -> RuntimeException
        let mut arithmetic = NativeType::named("java.lang.ArithmeticException");
        arithmetic.supertypes = vec!["java.lang.RuntimeException".to_string()];
        registry.register_type(arithmetic);
        // QLRuntimeException -> RuntimeException
        let mut ql_rte = NativeType::named("com.alibaba.qlexpress4.exception.QLRuntimeException");
        ql_rte.supertypes = vec!["java.lang.RuntimeException".to_string()];
        registry.register_type(ql_rte);
        // java.lang.Error -> Throwable
        let mut error = NativeType::named("java.lang.Error");
        error.supertypes = vec!["java.lang.Throwable".to_string()];
        registry.register_type(error);
        // java.lang.OutOfMemoryError -> Error
        let mut oome = NativeType::named("java.lang.OutOfMemoryError");
        oome.supertypes = vec!["java.lang.Error".to_string()];
        registry.register_type(oome);
        // 用户自定义类:非异常，但类名含 "Exception" 字串
        let mut handler = NativeType::named("com.example.MyExceptionHandler");
        handler.supertypes = vec!["java.lang.Object".to_string()];
        registry.register_type(handler);
        registry
    }

    #[test]
    fn exact_name_match() {
        let registry = test_registry();
        let entry = ClassRef::from_name("java.lang.NullPointerException");
        assert!(class_assignable_from(
            &entry,
            "java.lang.NullPointerException",
            &registry,
        ));
    }

    #[test]
    fn object_matches_everything() {
        let registry = test_registry();
        let entry = ClassRef::from_name("java.lang.Object");
        assert!(class_assignable_from(
            &entry,
            "com.example.AnyClass",
            &registry,
        ));
        assert!(class_assignable_from(
            &entry,
            "java.lang.Throwable",
            &registry,
        ));
    }

    #[test]
    fn npe_assignable_to_runtime_exception_via_registry() {
        let registry = test_registry();
        let entry = ClassRef::from_name("java.lang.RuntimeException");
        assert!(class_assignable_from(
            &entry,
            "java.lang.NullPointerException",
            &registry,
        ));
    }

    #[test]
    fn npe_assignable_to_exception_via_registry() {
        let registry = test_registry();
        let entry = ClassRef::from_name("java.lang.Exception");
        assert!(class_assignable_from(
            &entry,
            "java.lang.NullPointerException",
            &registry,
        ));
    }

    #[test]
    fn npe_assignable_to_throwable_via_registry() {
        let registry = test_registry();
        let entry = ClassRef::from_name("java.lang.Throwable");
        assert!(class_assignable_from(
            &entry,
            "java.lang.NullPointerException",
            &registry,
        ));
    }

    #[test]
    fn ql_rte_assignable_to_runtime_exception() {
        let registry = test_registry();
        let entry = ClassRef::from_name("java.lang.RuntimeException");
        assert!(class_assignable_from(
            &entry,
            "com.alibaba.qlexpress4.exception.QLRuntimeException",
            &registry,
        ));
    }

    #[test]
    fn oome_assignable_to_throwable() {
        let registry = test_registry();
        let entry = ClassRef::from_name("java.lang.Throwable");
        assert!(class_assignable_from(
            &entry,
            "java.lang.OutOfMemoryError",
            &registry,
        ));
    }

    /// 核心回归:类名含 "Exception" 但不是 Exception 子类的类型不应被
    /// `catch (Exception e)` 捕获。这是旧版字串包含匹配的典型误判场景。
    #[test]
    fn exception_handler_class_not_caught_by_catch_exception() {
        let registry = test_registry();
        let entry = ClassRef::from_name("java.lang.Exception");
        assert!(!class_assignable_from(
            &entry,
            "com.example.MyExceptionHandler",
            &registry,
        ));
    }

    #[test]
    fn throwable_handler_class_not_caught_by_catch_throwable() {
        let registry = test_registry();
        let entry = ClassRef::from_name("java.lang.Throwable");
        assert!(!class_assignable_from(
            &entry,
            "com.example.MyThrowableHandler",
            &registry,
        ));
    }

    #[test]
    fn fallback_known_hierarchy_without_registry_entry() {
        let registry = NativeRegistry::new();
        let entry = ClassRef::from_name("java.lang.RuntimeException");
        assert!(class_assignable_from(
            &entry,
            "java.lang.ArithmeticException",
            &registry,
        ));
        let entry_err = ClassRef::from_name("java.lang.Error");
        assert!(!class_assignable_from(
            &entry_err,
            "java.lang.ArithmeticException",
            &registry,
        ));
    }

    #[test]
    fn fallback_error_hierarchy() {
        let registry = NativeRegistry::new();
        let throwable_entry = ClassRef::from_name("java.lang.Throwable");
        assert!(class_assignable_from(
            &throwable_entry,
            "java.lang.OutOfMemoryError",
            &registry,
        ));
        let exception_entry = ClassRef::from_name("java.lang.Exception");
        assert!(!class_assignable_from(
            &exception_entry,
            "java.lang.OutOfMemoryError",
            &registry,
        ));
    }

    #[test]
    fn fallback_user_class_not_matched() {
        let registry = NativeRegistry::new();
        for catch_type in &[
            "java.lang.Exception",
            "java.lang.RuntimeException",
            "java.lang.Throwable",
            "java.lang.Error",
        ] {
            let entry = ClassRef::from_name(catch_type);
            assert!(
                !class_assignable_from(&entry, "com.example.MyService", &registry),
                "com.example.MyService should not be caught by catch ({catch_type})",
            );
        }
    }
}
