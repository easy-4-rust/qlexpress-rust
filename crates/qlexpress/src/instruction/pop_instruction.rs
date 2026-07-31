//! 弹栈指令,对应 Java `com.alibaba.qlexpress4.runtime.instruction.PopInstruction`。
//! 职责:弹出并丢弃栈顶元素。
//! 本文件由 `flow.rs` 拆分而来(SPEC §5.5 一类一文件),仅移动代码与补充中文注释,行为完全一致。

use crate::exception::error_reporter::ErrorReporter;
use crate::exception::QLException;
use crate::ql_options::QLOptions;
use crate::runtime::instruction::QLInstruction;
use crate::runtime::q_result::QResult;
use crate::runtime::qcontext::QContext;
use crate::utils::println_utils::PrintlnUtils;
use std::rc::Rc;

/// 弹栈指令。对应 Java: com.alibaba.qlexpress4.runtime.instruction.PopInstruction(职责:弹出并丢弃栈顶元素)
/// Operation: pop top element
/// Input: 1
/// Output: 0
///
/// Mirrors Java `PopInstruction`.
pub struct PopInstruction {
    error_reporter: Rc<dyn ErrorReporter>,
}

impl PopInstruction {
    /// 构造指令,对应 Java 构造器 `PopInstruction`。
    pub fn new(error_reporter: Rc<dyn ErrorReporter>) -> Self {
        PopInstruction { error_reporter }
    }
}

impl QLInstruction for PopInstruction {
    /// 向下转型支持(供 api/parsecache Exporter 的 Java `instanceof` 分派)。
    fn as_any(&self) -> Option<&dyn std::any::Any> {
        Some(self)
    }

    fn execute(
        &self,
        q_context: &mut dyn QContext,
        _ql_options: &QLOptions,
    ) -> Result<QResult, QLException> {
        q_context.pop();
        Ok(QResult::NEXT_INSTRUCTION)
    }

    fn stack_input(&self) -> i32 {
        1
    }

    fn stack_output(&self) -> i32 {
        0
    }

    fn println(&self, index: usize, depth: usize, debug: &mut dyn FnMut(String)) {
        PrintlnUtils::println_by_cur_depth(depth as i32, &format!("{index}: Pop"), debug);
    }

    fn error_reporter(&self) -> &Rc<dyn ErrorReporter> {
        &self.error_reporter
    }
}
