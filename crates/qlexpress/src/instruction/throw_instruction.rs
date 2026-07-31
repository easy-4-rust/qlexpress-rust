//! 抛出异常指令,对应 Java `com.alibaba.qlexpress4.runtime.instruction.ThrowInstruction`。
//! 职责:弹出栈顶异常并抛出。
//! 本文件由 `flow.rs` 拆分而来(SPEC §5.5 一类一文件),仅移动代码与补充中文注释,行为完全一致。

use crate::exception::QLException;
use crate::exception::error_codes;
use crate::exception::error_reporter::ErrorReporter;
use crate::ql_options::QLOptions;
use crate::runtime::instruction::QLInstruction;
use crate::runtime::q_result::QResult;
use crate::runtime::qcontext::QContext;
use crate::utils::println_utils::PrintlnUtils;
use std::rc::Rc;

/// 抛出异常指令。对应 Java: com.alibaba.qlexpress4.runtime.instruction.ThrowInstruction(职责:弹出栈顶异常并抛出)
/// Operation: throw top element on the stack
/// Input: 1
/// Output: 0
///
/// Mirrors Java `ThrowInstruction`.
pub struct ThrowInstruction {
    error_reporter: Rc<dyn ErrorReporter>,
}

impl ThrowInstruction {
    /// 构造指令,对应 Java 构造器 `ThrowInstruction`。
    pub fn new(error_reporter: Rc<dyn ErrorReporter>) -> Self {
        ThrowInstruction { error_reporter }
    }
}

impl QLInstruction for ThrowInstruction {
    /// 向下转型支持(供 api/parsecache Exporter 的 Java `instanceof` 分派)。
    fn as_any(&self) -> Option<&dyn std::any::Any> {
        Some(self)
    }

    fn execute(
        &self,
        q_context: &mut dyn QContext,
        _ql_options: &QLOptions,
    ) -> Result<QResult, QLException> {
        let throw_obj = q_context.pop().get();
        Err(self.error_reporter.report_with_catch(
            Some(throw_obj),
            error_codes::QL_THROW,
            error_codes::error_msg(error_codes::QL_THROW),
        ))
    }

    fn stack_input(&self) -> i32 {
        1
    }

    fn stack_output(&self) -> i32 {
        0
    }

    fn println(&self, index: usize, depth: usize, debug: &mut dyn FnMut(String)) {
        PrintlnUtils::println_by_cur_depth(depth as i32, &format!("{index}: Throw"), debug);
    }

    fn is_terminal(&self) -> bool {
        true
    }

    fn error_reporter(&self) -> &Rc<dyn ErrorReporter> {
        &self.error_reporter
    }
}
