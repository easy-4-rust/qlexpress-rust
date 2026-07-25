//! Constant instructions, mirroring Java `ConstInstruction` and
//! `CallConstInstruction`.

use std::rc::Rc;

use crate::exception::error_codes;
use crate::exception::error_reporter::ErrorReporter;
use crate::exception::QLException;
use crate::ql_options::QLOptions;
use crate::ql_result::QResult;
use crate::runtime::instruction::{with_trace, QLInstruction};
use crate::runtime::qcontext::QContext;
use crate::runtime::qlambda::QLambda;
use crate::runtime::util::throw_utils::{report_user_defined_exception, wrap_throwable};
use crate::runtime::value::{DataValue, QValue};
use crate::utils::println_utils::PrintlnUtils;

/// Operation: push constObj to stack
/// Input: 0
/// Output: 1
///
/// Mirrors Java `ConstInstruction`.
pub struct ConstInstruction {
    error_reporter: Rc<dyn ErrorReporter>,
    const_obj: DataValue,
    trace_key: Option<i32>,
}

impl ConstInstruction {
    pub fn new(
        error_reporter: Rc<dyn ErrorReporter>,
        const_obj: DataValue,
        trace_key: Option<i32>,
    ) -> Self {
        ConstInstruction {
            error_reporter,
            const_obj,
            trace_key,
        }
    }

    pub fn const_obj(&self) -> &DataValue {
        &self.const_obj
    }

    pub fn trace_key(&self) -> Option<i32> {
        self.trace_key
    }
}

impl QLInstruction for ConstInstruction {
    fn execute(
        &self,
        q_context: &mut dyn QContext,
        _ql_options: &QLOptions,
    ) -> Result<QResult, QLException> {
        q_context.push(QValue::Data(self.const_obj.clone()));

        // trace
        with_trace(q_context, self.trace_key, |trace| {
            trace.value_evaluated(self.const_obj.clone());
        });

        Ok(QResult::NEXT_INSTRUCTION)
    }

    fn stack_input(&self) -> i32 {
        0
    }

    fn stack_output(&self) -> i32 {
        1
    }

    fn println(&self, index: usize, depth: usize, debug: &mut dyn FnMut(String)) {
        PrintlnUtils::println_by_cur_depth(
            depth as i32,
            &format!("{}: LoadConst {}", index, self.const_obj.string_value_of()),
            debug,
        );
    }

    fn error_reporter(&self) -> &Rc<dyn ErrorReporter> {
        &self.error_reporter
    }
}

/// Operation: call const lambda
/// Input: ${argNum}
/// Output: 1 const lambda result
///
/// Mirrors Java `CallConstInstruction`.
pub struct CallConstInstruction {
    error_reporter: Rc<dyn ErrorReporter>,
    const_lambda: Rc<QLambda>,
    arg_num: usize,
    lambda_name: String,
}

impl CallConstInstruction {
    pub fn new(
        error_reporter: Rc<dyn ErrorReporter>,
        const_lambda: Rc<QLambda>,
        arg_num: usize,
        lambda_name: impl Into<String>,
    ) -> Self {
        CallConstInstruction {
            error_reporter,
            const_lambda,
            arg_num,
            lambda_name: lambda_name.into(),
        }
    }

    pub fn const_lambda(&self) -> &Rc<QLambda> {
        &self.const_lambda
    }

    pub fn arg_num(&self) -> usize {
        self.arg_num
    }

    pub fn lambda_name(&self) -> &str {
        &self.lambda_name
    }
}

impl QLInstruction for CallConstInstruction {
    fn execute(
        &self,
        q_context: &mut dyn QContext,
        _ql_options: &QLOptions,
    ) -> Result<QResult, QLException> {
        let args = q_context.pop_n(self.arg_num);
        let arg_arr = args.values();

        match self.const_lambda.call(&arg_arr) {
            Ok(result) => {
                q_context.push(QValue::Data(result.value()).to_immutable());
                Ok(QResult::NEXT_INSTRUCTION)
            }
            Err(err) => {
                // Java: UserDefineException → reportUserDefinedException;
                // other Throwables → wrapThrowable(EXECUTE_BLOCK_ERROR).
                if err.error_code() == error_codes::INVALID_ARGUMENT
                    || err.error_code() == error_codes::BIZ_EXCEPTION
                {
                    Err(report_user_defined_exception(&*self.error_reporter, &err))
                } else {
                    Err(wrap_throwable(
                        err,
                        &*self.error_reporter,
                        error_codes::EXECUTE_BLOCK_ERROR,
                        error_codes::error_msg(error_codes::EXECUTE_BLOCK_ERROR),
                        &[],
                    ))
                }
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
            &format!("{}: CallConstLambda {}", index, self.lambda_name),
            debug,
        );
    }

    fn error_reporter(&self) -> &Rc<dyn ErrorReporter> {
        &self.error_reporter
    }
}
