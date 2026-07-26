//! 定参 Lambda 调用指令,对应 Java `com.alibaba.qlexpress4.runtime.instruction.CallInstruction`。
//! 职责:以固定参数个数调用 Lambda。
//! 本文件由 `call.rs` 拆分而来(SPEC §5.5 一类一文件),仅移动代码与补充中文注释,行为完全一致。

use crate::exception::error_codes;
use crate::exception::error_reporter::ErrorReporter;
use crate::exception::QLException;
use crate::ql_options::QLOptions;
use crate::runtime::instruction::QLInstruction;
use crate::runtime::q_result::QResult;
use crate::runtime::qcontext::QContext;
use crate::runtime::util::throw_utils::{report_user_defined_exception, wrap_throwable};
use crate::runtime::value::{DataValue, QValue};
use crate::utils::println_utils::PrintlnUtils;
use std::rc::Rc;

/// 定参 Lambda 调用指令。对应 Java: com.alibaba.qlexpress4.runtime.instruction.CallInstruction(职责:以固定参数个数调用 Lambda)
/// Operation: call a lambda with fixed number of arguments
/// Input: ${argNum} + 1
/// Output: 1, lambda return result
///
/// Mirrors Java `CallInstruction`.
pub struct CallInstruction {
    error_reporter: Rc<dyn ErrorReporter>,
    arg_num: usize,
}

impl CallInstruction {
    /// 构造指令,对应 Java 构造器 `CallInstruction`。
    pub fn new(error_reporter: Rc<dyn ErrorReporter>, arg_num: usize) -> Self {
        CallInstruction {
            error_reporter,
            arg_num,
        }
    }

    /// 对应 Java 方法 `argNum`。
    pub fn arg_num(&self) -> usize {
        self.arg_num
    }
}

impl QLInstruction for CallInstruction {
    /// 向下转型支持(供 api/parsecache Exporter 的 Java `instanceof` 分派)。
    fn as_any(&self) -> Option<&dyn std::any::Any> {
        Some(self)
    }

    fn execute(
        &self,
        q_context: &mut dyn QContext,
        ql_options: &QLOptions,
    ) -> Result<QResult, QLException> {
        let parameters = q_context.pop_n(self.arg_num + 1);
        let bean = parameters.get(0).expect("lambda slot popped").get();
        if bean.is_null() {
            if ql_options.is_avoid_null_pointer() {
                q_context.push(QValue::Data(DataValue::NULL_VALUE));
                return Ok(QResult::NEXT_INSTRUCTION);
            }
            return Err(self.error_reporter.report(
                error_codes::NULL_CALL,
                error_codes::error_msg(error_codes::NULL_CALL),
            ));
        }
        let DataValue::Lambda(q_lambda) = &bean else {
            return Err(self.error_reporter.report_format(
                error_codes::OBJECT_NOT_CALLABLE,
                error_codes::error_msg(error_codes::OBJECT_NOT_CALLABLE),
                &[bean.data_type_name().to_string()],
            ));
        };
        let params: Vec<DataValue> = (0..self.arg_num)
            .map(|i| parameters.get_value(i + 1))
            .collect();
        match q_lambda.call(&params) {
            Ok(result) => {
                q_context.push(QValue::Data(result.value()).to_immutable());
                Ok(QResult::NEXT_INSTRUCTION)
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

    fn stack_input(&self) -> i32 {
        self.arg_num as i32 + 1
    }

    fn stack_output(&self) -> i32 {
        1
    }

    fn println(&self, index: usize, depth: usize, debug: &mut dyn FnMut(String)) {
        PrintlnUtils::println_by_cur_depth(
            depth as i32,
            &format!("{}: Call with argNum {}", index, self.arg_num),
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
