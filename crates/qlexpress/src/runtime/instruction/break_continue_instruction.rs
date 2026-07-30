//! break/continue 指令,对应 Java `com.alibaba.qlexpress4.runtime.instruction.BreakContinueInstruction`。
//! 职责:循环中的 break/continue 跳转。
//! 本文件由 `flow.rs` 拆分而来(SPEC §5.5 一类一文件),仅移动代码与补充中文注释,行为完全一致。

use crate::exception::error_reporter::ErrorReporter;
use crate::exception::QLException;
use crate::ql_options::QLOptions;
use crate::runtime::instruction::QLInstruction;
use crate::runtime::q_result::QResult;
use crate::runtime::qcontext::QContext;
use crate::utils::println_utils::PrintlnUtils;
use std::rc::Rc;

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
    /// 创建对象实例。
    /// 参数：`error_reporter`、`is_break`；返回：`Self`。
    /// 对应或承接 Java 源文件：`com/alibaba/qlexpress4/runtime/instruction/BreakContinueInstruction.java`，构造器 `<init>`；Rust 侧按所有权与 `Result` 语义适配。
    /// Java `new BreakContinueInstruction(errorReporter, result)` where
    /// `result` is `LOOP_BREAK_RESULT` or `LOOP_CONTINUE_RESULT`.
    /// 对应 Java: com.alibaba.qlexpress4.runtime.instruction.BreakContinueInstruction#new。
    pub fn new(error_reporter: Rc<dyn ErrorReporter>, is_break: bool) -> Self {
        BreakContinueInstruction {
            error_reporter,
            is_break,
        }
    }

    /// 处理 result 对应的领域职责。
    /// 无显式参数；返回：`QResult`。
    /// 对应或承接 Java 源文件：`com/alibaba/qlexpress4/runtime/instruction/BreakContinueInstruction.java`，方法 `result`；Rust 侧按所有权与 `Result` 语义适配。
    /// Java `getResult()`.
    /// 对应 Java: com.alibaba.qlexpress4.runtime.instruction.BreakContinueInstruction#result。
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
