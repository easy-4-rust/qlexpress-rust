//! 常量 Lambda 调用指令,对应 Java `com.alibaba.qlexpress4.runtime.instruction.CallConstInstruction`。
//! 职责:调用常量 Lambda 并将其结果压栈。
//! 本文件由 `const_inst.rs` 拆分而来(SPEC §5.5 一类一文件),仅移动代码与补充中文注释,行为完全一致。

use crate::exception::QLException;
use crate::exception::error_codes;
use crate::exception::error_reporter::ErrorReporter;
use crate::ql_options::QLOptions;
use crate::runtime::instruction::QLInstruction;
use crate::runtime::q_result::QResult;
use crate::runtime::qcontext::QContext;
use crate::runtime::qlambda::QLambda;
use crate::runtime::util::throw_utils::{report_user_defined_exception, wrap_throwable};
use crate::runtime::value::QValue;
use crate::utils::println_utils::PrintlnUtils;
use std::rc::Rc;

/// 常量 Lambda 调用指令。对应 Java: com.alibaba.qlexpress4.runtime.instruction.CallConstInstruction(职责:调用常量 Lambda 并将其结果压栈)
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
    /// 构造指令,对应 Java 构造器 `CallConstInstruction`。
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

    /// 对应 Java 方法 `constLambda`。
    pub fn const_lambda(&self) -> &Rc<QLambda> {
        &self.const_lambda
    }

    /// 对应 Java 方法 `argNum`。
    pub fn arg_num(&self) -> usize {
        self.arg_num
    }

    /// 对应 Java 方法 `lambdaName`。
    pub fn lambda_name(&self) -> &str {
        &self.lambda_name
    }
}

impl QLInstruction for CallConstInstruction {
    /// 向下转型支持(供 api/parsecache Exporter 的 Java `instanceof` 分派)。
    fn as_any(&self) -> Option<&dyn std::any::Any> {
        Some(self)
    }

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
