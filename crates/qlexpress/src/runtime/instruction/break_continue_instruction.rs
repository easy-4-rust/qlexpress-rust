//! break/continue 指令,对应 Java `com.alibaba.qlexpress4.runtime.instruction.BreakContinueInstruction`。
//! 职责:循环中的 break/continue 跳转。
//! 本文件由 `flow.rs` 拆分而来(SPEC §5.5 一类一文件),仅移动代码与补充中文注释,行为完全一致。

use std::rc::Rc;
use crate::exception::error_reporter::ErrorReporter;
use crate::exception::QLException;
use crate::ql_options::QLOptions;
use crate::runtime::q_result::QResult;
use crate::runtime::instruction::QLInstruction;
use crate::runtime::qcontext::QContext;
use crate::utils::println_utils::PrintlnUtils;

/// break/continue 指令。对应 Java: com.alibaba.qlexpress4.runtime.instruction.BreakContinueInstruction(职责:循环中的 break/continue 跳转)
/// Operation: return break object and exit lambda
/// Input: 0
/// Output: 0
///
/// Mirrors Java `BreakContinueInstruction`.
pub struct BreakContinueInstruction {
    error_reporter: Rc<dyn ErrorReporter>,
    /// `true` = break (Java `QResult.LOOP_BREAK_RESULT`), `false` =
    /// continue (`QResult.LOOP_CONTINUE_RESULT`).
    is_break: bool,
}

impl BreakContinueInstruction {
    /// Java `new BreakContinueInstruction(errorReporter, result)` where
    /// `result` is `LOOP_BREAK_RESULT` or `LOOP_CONTINUE_RESULT`.
    pub fn new(error_reporter: Rc<dyn ErrorReporter>, is_break: bool) -> Self {
        BreakContinueInstruction {
            error_reporter,
            is_break,
        }
    }

    /// Java `getResult()`.
    pub fn result(&self) -> QResult {
        if self.is_break {
            QResult::LOOP_BREAK_RESULT
        } else {
            QResult::LOOP_CONTINUE_RESULT
        }
    }

    /// 对应 Java 方法 `isBreak`。
    pub fn is_break(&self) -> bool {
        self.is_break
    }
}

impl QLInstruction for BreakContinueInstruction {
    /// 向下转型支持(供 api/parsecache Exporter 的 Java `instanceof` 分派)。
    fn as_any(&self) -> Option<&dyn std::any::Any> {
        Some(self)
    }

    fn execute(
        &self,
        _q_context: &mut dyn QContext,
        _ql_options: &QLOptions,
    ) -> Result<QResult, QLException> {
        Ok(self.result())
    }

    fn stack_input(&self) -> i32 {
        0
    }

    fn stack_output(&self) -> i32 {
        0
    }

    fn println(&self, index: usize, depth: usize, debug: &mut dyn FnMut(String)) {
        let break_continue = if self.is_break { "Break" } else { "Continue" };
        PrintlnUtils::println_by_cur_depth(
            depth as i32,
            &format!("{index}: {break_continue}"),
            debug,
        );
    }

    fn is_terminal(&self) -> bool {
        true
    }

    fn error_reporter(&self) -> &Rc<dyn ErrorReporter> {
        &self.error_reporter
    }
}

