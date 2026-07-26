//! 超时检查指令,对应 Java `com.alibaba.qlexpress4.runtime.instruction.CheckTimeOutInstruction`。
//! 职责:检查脚本执行是否超时。
//! 本文件由 `flow.rs` 拆分而来(SPEC §5.5 一类一文件),仅移动代码与补充中文注释,行为完全一致。

use std::rc::Rc;
use crate::exception::error_codes;
use crate::exception::error_reporter::ErrorReporter;
use crate::exception::QLException;
use crate::ql_options::QLOptions;
use crate::runtime::q_result::QResult;
use crate::runtime::instruction::QLInstruction;
use crate::runtime::qcontext::QContext;
use crate::runtime::qvm_runtime::current_time_millis;
use crate::utils::println_utils::PrintlnUtils;

/// 超时检查指令。对应 Java: com.alibaba.qlexpress4.runtime.instruction.CheckTimeOutInstruction(职责:检查脚本执行是否超时)
/// Operation: check if program timeout
/// Input: 0
/// Output: 0
///
/// Mirrors Java `CheckTimeOutInstruction`.
pub struct CheckTimeOutInstruction {
    error_reporter: Rc<dyn ErrorReporter>,
}

impl CheckTimeOutInstruction {
    /// 构造指令,对应 Java 构造器 `CheckTimeOutInstruction`。
    pub fn new(error_reporter: Rc<dyn ErrorReporter>) -> Self {
        CheckTimeOutInstruction { error_reporter }
    }
}

impl QLInstruction for CheckTimeOutInstruction {
    /// 向下转型支持(供 api/parsecache Exporter 的 Java `instanceof` 分派)。
    fn as_any(&self) -> Option<&dyn std::any::Any> {
        Some(self)
    }

    fn execute(
        &self,
        q_context: &mut dyn QContext,
        ql_options: &QLOptions,
    ) -> Result<QResult, QLException> {
        if ql_options.timeout_millis() <= 0 {
            return Ok(QResult::NEXT_INSTRUCTION);
        }
        if current_time_millis() - q_context.script_start_time_stamp() > ql_options.timeout_millis()
        {
            // timeout
            return Err(self.error_reporter.report_format(
                error_codes::SCRIPT_TIME_OUT,
                error_codes::error_msg(error_codes::SCRIPT_TIME_OUT),
                &[ql_options.timeout_millis().to_string()],
            ));
        }
        Ok(QResult::NEXT_INSTRUCTION)
    }

    fn stack_input(&self) -> i32 {
        0
    }

    fn stack_output(&self) -> i32 {
        0
    }

    fn println(&self, index: usize, depth: usize, debug: &mut dyn FnMut(String)) {
        PrintlnUtils::println_by_cur_depth(depth as i32, &format!("{index}: CheckTimeout"), debug);
    }

    fn error_reporter(&self) -> &Rc<dyn ErrorReporter> {
        &self.error_reporter
    }
}

