//! try-catch 指令,对应 Java `com.alibaba.qlexpress4.runtime.instruction.TryCatchInstruction`。
//! 职责:异常捕获与 finally 处理。
//! 本文件由 `flow.rs` 拆分而来(SPEC §5.5 一类一文件),仅移动代码与补充中文注释,行为完全一致。

use std::rc::Rc;
use crate::exception::error_codes;
use crate::exception::error_reporter::ErrorReporter;
use crate::exception::QLException;
use crate::ql_options::QLOptions;
use crate::runtime::q_result::QResult;
use crate::runtime::instruction::QLInstruction;
use crate::runtime::member::ClassRef;
use crate::runtime::qcontext::QContext;
use crate::runtime::qlambda_definition::QLambdaDefinition;
use crate::runtime::util::throw_utils::wrap_throwable;
use crate::runtime::value::{DataValue, QValue};
use crate::utils::println_utils::PrintlnUtils;

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
    fn should_exit_try_catch(result: &QResult) -> bool {
        matches!(
            result,
            QResult::Return(_) | QResult::Break | QResult::Continue(_)
        )
    }

    /// Java `getExceptionHandler(Class)`:
    /// `entry.getKey().isAssignableFrom(catchObjClass)`.
    fn get_exception_handler(&self, catch_obj: Option<&DataValue>) -> Option<&Rc<dyn QLambdaDefinition>> {
        let catch_type = match catch_obj {
            // Java substitutes `new Object()` for a null catch object.
            None => "java.lang.Object",
            Some(value) => value.data_type_name(),
        };
        self.exception_table
            .iter()
            .find(|(clz, _)| class_assignable_from(clz, catch_type))
            .map(|(_, handler)| handler)
    }

    /// Java `callExceptionHandler`.
    fn call_exception_handler(
        &self,
        catch_obj: Option<&DataValue>,
        q_context: &mut dyn QContext,
        ql_options: &QLOptions,
    ) -> Result<Option<QResult>, QLException> {
        let exception_handler = self.get_exception_handler(catch_obj);
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
                let handled =
                    self.call_exception_handler(err.catch_obj(), q_context, ql_options)?;
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

        // v1 保守行为:is_expression_form=true 时,Continue(value) 作为
        // 块表达式结果压栈;Break/Return 作为控制信号透传。
        // 这确保 `1 + try{100+1/0}catch{11}` 正确返回 12。
        // 已知 v1 限制:while 循环内 try 的 continue/break 信号被
        // is_expression_form=true 吞掉(因为 visit_try_catch_expr
        // 始终设 true)。需要更精细的传播路径分析来同时满足两种场景。
        let signal_to_propagate: Option<QResult> = if self.is_expression_form {
            if Self::should_exit_try_catch(&try_catch_result)
                && !matches!(&try_catch_result, QResult::Continue(_))
            {
                Some(try_catch_result.clone())
            } else {
                None
            }
        } else if Self::should_exit_try_catch(&try_catch_result) {
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

    fn println(&self, index: usize, depth: usize, debug: &mut dyn FnMut(String)) {
        PrintlnUtils::println_by_cur_depth(depth as i32, &format!("{index}: TryCatch"), debug);
        PrintlnUtils::println_by_cur_depth(depth as i32 + 1, "Body", debug);
        self.body.println(depth + 2, debug);
        for (clz, handler) in &self.exception_table {
            PrintlnUtils::println_by_cur_depth(depth as i32 + 1, clz.simple_name(), debug);
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

/// Java `Class.isAssignableFrom` for the built-in type/exception hierarchy
/// used by catch matching. `java.lang.Object` is assignable from anything;
/// the exception supertypes match their subclasses; otherwise exact name.
fn class_assignable_from(entry: &ClassRef, catch_type: &str) -> bool {
    let entry_name = entry.java_name();
    if entry_name == catch_type || entry_name == "java.lang.Object" {
        return true;
    }
    let catch_is_exception = catch_type.contains("Exception") || catch_type.contains("Throwable");
    match entry_name {
        "java.lang.Throwable" => catch_is_exception || catch_type.contains("Error"),
        "java.lang.Exception" => catch_is_exception,
        "java.lang.RuntimeException" => {
            catch_type.contains("RuntimeException")
                || catch_type == "com.alibaba.qlexpress4.exception.QLRuntimeException"
        }
        _ => false,
    }
}

