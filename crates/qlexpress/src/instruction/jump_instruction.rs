//! 无条件跳转指令,对应 Java `com.alibaba.qlexpress4.runtime.instruction.JumpInstruction`。
//! 职责:按相对偏移无条件跳转。
//! 本文件由 `flow.rs` 拆分而来(SPEC §5.5 一类一文件),仅移动代码与补充中文注释,行为完全一致。

use crate::exception::QLException;
use crate::exception::error_reporter::ErrorReporter;
use crate::ql_options::QLOptions;
use crate::runtime::instruction::QLInstruction;
use crate::runtime::q_result::QResult;
use crate::runtime::qcontext::QContext;
use crate::utils::println_utils::PrintlnUtils;
use std::cell::Cell;
use std::rc::Rc;

/// 无条件跳转指令。对应 Java: com.alibaba.qlexpress4.runtime.instruction.JumpInstruction(职责:按相对偏移无条件跳转)
/// Operation: jump to a position
/// Input: 0
/// Output: 0
///
/// Mirrors Java `JumpInstruction` (position is a relative offset, applied
/// by the VM loop with `i += position`).
pub struct JumpInstruction {
    error_reporter: Rc<dyn ErrorReporter>,
    position: Cell<i32>,
}

impl JumpInstruction {
    /// 构造指令,对应 Java 构造器 `JumpInstruction`。
    pub fn new(error_reporter: Rc<dyn ErrorReporter>, position: i32) -> Self {
        JumpInstruction {
            error_reporter,
            position: Cell::new(position),
        }
    }

    /// 对应 Java 方法 `setPosition`。
    pub fn set_position(&self, position: i32) {
        self.position.set(position);
    }

    /// 对应 Java 方法 `position`。
    pub fn position(&self) -> i32 {
        self.position.get()
    }
}

impl QLInstruction for JumpInstruction {
    /// 向下转型支持(供 api/parsecache Exporter 的 Java `instanceof` 分派)。
    fn as_any(&self) -> Option<&dyn std::any::Any> {
        Some(self)
    }

    fn execute(
        &self,
        _q_context: &mut dyn QContext,
        _ql_options: &QLOptions,
    ) -> Result<QResult, QLException> {
        Ok(QResult::Jump(self.position.get()))
    }

    fn stack_input(&self) -> i32 {
        0
    }

    fn stack_output(&self) -> i32 {
        0
    }

    fn println(&self, index: usize, depth: usize, debug: &mut dyn FnMut(String)) {
        PrintlnUtils::println_by_cur_depth(
            depth as i32,
            &format!("{}: Jump {}", index, self.position.get()),
            debug,
        );
    }

    fn static_jump(&self) -> Option<i32> {
        Some(self.position.get())
    }

    fn error_reporter(&self) -> &Rc<dyn ErrorReporter> {
        &self.error_reporter
    }
}
