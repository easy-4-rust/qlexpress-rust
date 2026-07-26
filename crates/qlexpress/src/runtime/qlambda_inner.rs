//! 指令序列 Lambda,对应 Java `com.alibaba.qlexpress4.runtime.QLambdaInner`。
//! 职责:以一段编译后的指令序列为体的 Lambda,调用时绑定参数并执行取指-执行循环。
//! 本文件由 `qlambda.rs` 拆分而来(SPEC §5.5 一类一文件),仅移动代码与补充中文注释,行为完全一致。

use std::collections::HashMap;
use std::rc::Rc;

use crate::exception::error_codes;
use crate::exception::error_reporter::ErrorReporter;
use crate::exception::pure_err_reporter::PureErrReporter;
use crate::exception::QLException;
use crate::ql_options::QLOptions;
use crate::runtime::data::convert::obj_type_convertor::{ObjTypeConvertor, TargetType};
use crate::runtime::data::AssignableDataValue;
use crate::runtime::delegate_qcontext::DelegateQContext;
use crate::runtime::function::CustomFunction;
use crate::runtime::left_value::LeftValue;
use crate::runtime::q_result::QResult;
use crate::runtime::qcontext::QContext;
use crate::runtime::qlambda_definition_inner::QLambdaDefinitionInner;
use crate::runtime::qvm_runtime::run_instructions;
use crate::runtime::scope::{QScope, SymbolTable};
use crate::runtime::value::DataValue;

/// 指令序列 Lambda。对应 Java: com.alibaba.qlexpress4.runtime.QLambdaInner
///
/// Instruction-sequence lambda, mirroring Java `QLambdaInner`.
pub struct QLambdaInner {
    // pub(crate): [`crate::runtime::qlambda::QLambda`] 的 Debug 实现需要读取
    pub(crate) lambda_definition: Rc<QLambdaDefinitionInner>,
    q_context: DelegateQContext,
    ql_options: QLOptions,
    // pub(crate): [`crate::runtime::qlambda::QLambda`] 的 Debug 实现需要读取
    pub(crate) new_env: bool,
}

impl QLambdaInner {
    /// 构造指令序列 Lambda。对应 Java 构造器
    /// `QLambdaInner(lambdaDefinition, qContext, qlOptions, newEnv)`。
    pub fn new(
        lambda_definition: Rc<QLambdaDefinitionInner>,
        q_context: DelegateQContext,
        ql_options: QLOptions,
        new_env: bool,
    ) -> Self {
        QLambdaInner {
            lambda_definition,
            q_context,
            ql_options,
            new_env,
        }
    }

    /// 获取 Lambda 定义。对应 Java 字段 `lambdaDefinition` 的访问。
    pub fn definition(&self) -> &Rc<QLambdaDefinitionInner> {
        &self.lambda_definition
    }

    /// 调用 Lambda。对应 Java 方法 `call(Object... params)`。
    /// Java `call(Object... params)`.
    pub fn call(&self, params: &[DataValue]) -> Result<QResult, QLException> {
        let mut runtime = if self.new_env {
            self.inherit_scope(params)?
        } else {
            // DelegateQContext is a pair of Rc handles; cloning shares the
            // same runtime and scope chain (Java mutates its own wrapper).
            // DelegateQContext 是一对 Rc 句柄;克隆后共享同一 runtime 与作用域链
            // (Java 直接复用并修改自身包装器,语义一致)。
            DelegateQContext::new(
                Rc::clone(self.q_context.q_runtime()),
                self.q_context.current_scope(),
            )
        };
        self.call_inner(&mut runtime)
    }

    /// 调用并收集 Lambda 内定义的函数表。对应 Java 方法
    /// `getFunctionDefined(Object... params)`。
    /// Java `getFunctionDefined(Object... params)`.
    pub fn function_defined(
        &self,
        params: &[DataValue],
    ) -> Result<HashMap<String, Rc<dyn CustomFunction>>, QLException> {
        let mut new_runtime = if self.new_env {
            self.inherit_scope(params)?
        } else {
            DelegateQContext::new(
                Rc::clone(self.q_context.q_runtime()),
                self.q_context.current_scope(),
            )
        };
        self.call_inner(&mut new_runtime)?;
        Ok(new_runtime.function_table())
    }

    /// 取指-执行循环。对应 Java 方法 `callInner`(见 [`run_instructions`])。
    /// Java `callInner`: the fetch-execute loop (see [`run_instructions`]).
    fn call_inner(&self, runtime: &mut dyn QContext) -> Result<QResult, QLException> {
        run_instructions(
            runtime,
            self.lambda_definition.instructions(),
            &self.ql_options,
        )
    }

    /// 绑定参数并构造继承作用域。对应 Java 方法 `inheritScope`:
    /// 参数转换为声明类型(缺失的为 `null`),绑定在以捕获的定义作用域为父的
    /// 全新块作用域中。
    /// Java `inheritScope`: bind parameters (converted to their declared
    /// types, `null` for missing ones) in a fresh block scope whose parent
    /// is the captured definition scope.
    fn inherit_scope(&self, params: &[DataValue]) -> Result<DelegateQContext, QLException> {
        let params_definition = self.lambda_definition.params_type();
        let mut init_symbol_table: SymbolTable = HashMap::with_capacity(params.len());
        for (i, param_definition) in params_definition.iter().enumerate().take(params.len()) {
            let origin_param_i = &params[i];
            let target_cls = param_definition.clazz();
            let ql_convert_result = ObjTypeConvertor::cast_opt(origin_param_i, target_cls);
            if !ql_convert_result.is_convertible() {
                // Java: UserDefineException(INVALID_ARGUMENT, ...)
                let message = format!(
                    "invalid argument at index {} (start from 0), required type {}, but {} provided",
                    i,
                    target_cls
                        .map(TargetType::java_name)
                        .unwrap_or("java.lang.Object"),
                    if origin_param_i.is_null() {
                        "null".to_string()
                    } else {
                        origin_param_i.data_type_name().to_string()
                    }
                );
                return Err(
                    PureErrReporter::INSTANCE.report(error_codes::INVALID_ARGUMENT, &message)
                );
            }
            let slot: Rc<std::cell::RefCell<dyn LeftValue>> = match target_cls {
                Some(clz) => Rc::new(std::cell::RefCell::new(AssignableDataValue::with_type(
                    param_definition.name(),
                    ql_convert_result.into_converted(),
                    clz,
                ))),
                None => Rc::new(std::cell::RefCell::new(AssignableDataValue::new(
                    param_definition.name(),
                    ql_convert_result.into_converted(),
                ))),
            };
            init_symbol_table.insert(param_definition.name().to_string(), slot);
        }
        // null for rest params
        // 其余未传参数绑定 null(Java 语义)
        for param_definition in params_definition.iter().skip(params.len()) {
            let slot: Rc<std::cell::RefCell<dyn LeftValue>> = match param_definition.clazz() {
                Some(clz) => Rc::new(std::cell::RefCell::new(AssignableDataValue::with_type(
                    param_definition.name(),
                    DataValue::Null,
                    clz,
                ))),
                None => Rc::new(std::cell::RefCell::new(AssignableDataValue::new(
                    param_definition.name(),
                    DataValue::Null,
                ))),
            };
            init_symbol_table.insert(param_definition.name().to_string(), slot);
        }
        let new_scope =
            QScope::block_fresh_stack(&self.q_context.current_scope(), init_symbol_table);
        Ok(DelegateQContext::new(
            Rc::clone(self.q_context.q_runtime()),
            new_scope,
        ))
    }
}
