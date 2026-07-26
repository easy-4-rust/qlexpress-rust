//! 函数调用指令,对应 Java `com.alibaba.qlexpress4.runtime.instruction.CallFunctionInstruction`。
//! 职责:按函数名调用函数/Lambda。
//! 本文件由 `call.rs` 拆分而来(SPEC §5.5 一类一文件),仅移动代码与补充中文注释,行为完全一致。

use crate::exception::error_codes;
use crate::exception::error_reporter::ErrorReporter;
use crate::exception::QLException;
use crate::ql_options::QLOptions;
use crate::runtime::instruction::{with_trace, QLInstruction};
use crate::runtime::q_result::QResult;
use crate::runtime::qcontext::QContext;
use crate::runtime::util::throw_utils::{report_user_defined_exception, wrap_throwable};
use crate::runtime::value::{DataValue, QValue};
use crate::utils::println_utils::PrintlnUtils;
use std::rc::Rc;

/// 函数调用指令。对应 Java: com.alibaba.qlexpress4.runtime.instruction.CallFunctionInstruction(职责:按函数名调用函数/Lambda)
/// Operation: call ql function from function table
/// Input: ${argNum}
/// Output: 1 function result
///
/// Mirrors Java `CallFunctionInstruction`.
pub struct CallFunctionInstruction {
    error_reporter: Rc<dyn ErrorReporter>,
    function_name: String,
    arg_num: usize,
    trace_key: Option<i32>,
}

impl CallFunctionInstruction {
    /// 构造指令,对应 Java 构造器 `CallFunctionInstruction`。
    pub fn new(
        error_reporter: Rc<dyn ErrorReporter>,
        function_name: impl Into<String>,
        arg_num: usize,
        trace_key: Option<i32>,
    ) -> Self {
        CallFunctionInstruction {
            error_reporter,
            function_name: function_name.into(),
            arg_num,
            trace_key,
        }
    }

    /// 对应 Java 方法 `functionName`。
    pub fn function_name(&self) -> &str {
        &self.function_name
    }

    /// 对应 Java 方法 `argNum`。
    pub fn arg_num(&self) -> usize {
        self.arg_num
    }

    /// 对应 Java 方法 `traceKey`。
    pub fn trace_key(&self) -> Option<i32> {
        self.trace_key
    }

    /// Java `callLambda`: fall back to a lambda-valued variable of the same
    /// name.
    fn call_lambda(
        &self,
        q_context: &mut dyn QContext,
        ql_options: &QLOptions,
    ) -> Result<(), QLException> {
        let lambda_symbol = q_context.get_symbol_value(&self.function_name)?;
        // 对齐 Java:全局作用域对缺失变量返回 null 值(Java `null`),
        // 与「符号不存在」同等处理 —— avoidNullPointer 下短路为 null,
        // 否则报 FUNCTION_NOT_FOUND(而非 FUNCTION_TYPE_MISMATCH)。
        // (对齐测试 avoidnullpointer/can_not_find_function.ql、
        // avoid_null_pointer.ql 发现。)
        let lambda_symbol = lambda_symbol.filter(|v| !v.is_null());
        let Some(lambda_symbol) = lambda_symbol else {
            if ql_options.is_avoid_null_pointer() {
                q_context.pop_n(self.arg_num);
                q_context.push(QValue::Data(DataValue::NULL_VALUE));
                return Ok(());
            }
            return Err(self.error_reporter.report_format(
                error_codes::FUNCTION_NOT_FOUND,
                error_codes::error_msg(error_codes::FUNCTION_NOT_FOUND),
                std::slice::from_ref(&self.function_name),
            ));
        };
        let DataValue::Lambda(q_lambda) = &lambda_symbol else {
            return Err(self.error_reporter.report_format(
                error_codes::FUNCTION_TYPE_MISMATCH,
                error_codes::error_msg(error_codes::FUNCTION_TYPE_MISMATCH),
                std::slice::from_ref(&self.function_name),
            ));
        };
        let parameters = q_context.pop_n(self.arg_num);
        let parameters_arr = parameters.values();
        match q_lambda.call(&parameters_arr) {
            Ok(result) => {
                q_context.push(QValue::Data(result.value()).to_immutable());
                Ok(())
            }
            Err(err) => Err(rethrow_call_error(
                &self.error_reporter,
                err,
                error_codes::INVOKE_LAMBDA_ERROR,
                error_codes::error_msg(error_codes::INVOKE_LAMBDA_ERROR),
                &[],
            )),
        }
    }
}

impl QLInstruction for CallFunctionInstruction {
    /// 向下转型支持(供 api/parsecache Exporter 的 Java `instanceof` 分派)。
    fn as_any(&self) -> Option<&dyn std::any::Any> {
        Some(self)
    }

    fn execute(
        &self,
        q_context: &mut dyn QContext,
        ql_options: &QLOptions,
    ) -> Result<QResult, QLException> {
        let function = q_context.get_function(&self.function_name);
        let Some(function) = function else {
            self.call_lambda(q_context, ql_options)?;
            return Ok(QResult::NEXT_INSTRUCTION);
        };
        let parameters = q_context.pop_n(self.arg_num);
        match function.call(q_context, &parameters) {
            Ok(function_result_obj) => {
                q_context.push(QValue::Data(function_result_obj.clone()));

                // trace
                with_trace(q_context, self.trace_key, |trace| {
                    trace.value_evaluated(function_result_obj);
                });

                Ok(QResult::NEXT_INSTRUCTION)
            }
            Err(err) => {
                let reason = err.reason().to_string();
                Err(rethrow_call_error(
                    &self.error_reporter,
                    err,
                    error_codes::INVOKE_FUNCTION_INNER_ERROR,
                    error_codes::error_msg(error_codes::INVOKE_FUNCTION_INNER_ERROR),
                    &[self.function_name.clone(), reason],
                ))
            }
        }
    }

    fn stack_input(&self) -> i32 {
        self.arg_num as i32
    }

    fn stack_output(&self) -> i32 {
        1
    }

    fn println(&self, index: usize, depth: usize, debug: &mut dyn FnMut(String)) {
        PrintlnUtils::println_by_cur_depth(
            depth as i32,
            &format!(
                "{}: CallFunction {} {}",
                index, self.function_name, self.arg_num
            ),
            debug,
        );
    }

    fn error_reporter(&self) -> &Rc<dyn ErrorReporter> {
        &self.error_reporter
    }
}

/// Whether a lambda-call error came from a user-defined exception
/// (Java `catch (UserDefineException e)`).
fn is_user_defined(err: &QLException) -> bool {
    err.error_code() == error_codes::INVALID_ARGUMENT
        || err.error_code() == error_codes::BIZ_EXCEPTION
}

/// Java `ThrowUtils.reportUserDefinedException` / `wrapThrowable` dispatch
/// used by the call instructions.
fn rethrow_call_error(
    error_reporter: &Rc<dyn ErrorReporter>,
    err: QLException,
    wrap_code: &str,
    wrap_msg: &str,
    wrap_args: &[String],
) -> QLException {
    if is_user_defined(&err) {
        report_user_defined_exception(&**error_reporter, &err)
    } else {
        wrap_throwable(err, &**error_reporter, wrap_code, wrap_msg, wrap_args)
    }
}
