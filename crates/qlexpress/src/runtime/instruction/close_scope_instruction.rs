//! 关闭作用域指令,对应 Java `com.alibaba.qlexpress4.runtime.instruction.CloseScopeInstruction`。
//! 职责:关闭当前局部作用域。
//! 本文件由 `scope.rs` 拆分而来(SPEC §5.5 一类一文件),仅移动代码与补充中文注释,行为完全一致。

use crate::exception::error_reporter::ErrorReporter;
use crate::exception::QLException;
use crate::ql_options::QLOptions;
use crate::runtime::instruction::QLInstruction;
use crate::runtime::q_result::QResult;
use crate::runtime::qcontext::QContext;
use crate::utils::println_utils::PrintlnUtils;
use std::rc::Rc;

/// 关闭作用域指令。对应 Java: com.alibaba.qlexpress4.runtime.instruction.CloseScopeInstruction(职责:关闭当前局部作用域)
/// Operation: close scope
/// Input: 0
/// Output: 0
///
/// Mirrors Java `CloseScopeInstruction`.
pub struct CloseScopeInstruction {
    error_reporter: Rc<dyn ErrorReporter>,
    scope_name: String,
}

impl CloseScopeInstruction {
    /// 构造指令,对应 Java 构造器 `CloseScopeInstruction`。
    pub fn new(error_reporter: Rc<dyn ErrorReporter>, scope_name: impl Into<String>) -> Self {
        CloseScopeInstruction {
            error_reporter,
            scope_name: scope_name.into(),
        }
    }

    /// 对应 Java 方法 `scopeName`。
    pub fn scope_name(&self) -> &str {
        &self.scope_name
    }
}

impl QLInstruction for CloseScopeInstruction {
    /// 向下转型支持(供 api/parsecache Exporter 的 Java `instanceof` 分派)。
    fn as_any(&self) -> Option<&dyn std::any::Any> {
        Some(self)
    }

    fn execute(
        &self,
        q_context: &mut dyn QContext,
        _ql_options: &QLOptions,
    ) -> Result<QResult, QLException> {
        q_context.close_scope();
        Ok(QResult::NEXT_INSTRUCTION)
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
            &format!("{}: CloseScope {}", index, self.scope_name),
            debug,
        );
    }

    fn error_reporter(&self) -> &Rc<dyn ErrorReporter> {
        &self.error_reporter
    }
}
