//! 列表字面量指令,对应 Java `com.alibaba.qlexpress4.runtime.instruction.NewListInstruction`。
//! 职责:以元素列表创建 List。
//! 本文件由 `new_instance.rs` 拆分而来(SPEC §5.5 一类一文件),仅移动代码与补充中文注释,行为完全一致。

use crate::exception::error_reporter::ErrorReporter;
use crate::exception::QLException;
use crate::ql_options::QLOptions;
use crate::runtime::instruction::QLInstruction;
use crate::runtime::q_result::QResult;
use crate::runtime::qcontext::QContext;
use crate::runtime::value::{DataValue, QValue};
use crate::utils::println_utils::PrintlnUtils;
use std::rc::Rc;

/// 列表字面量指令。对应 Java: com.alibaba.qlexpress4.runtime.instruction.NewListInstruction(职责:以元素列表创建 List)
/// Operation: new a List with top ${initLength} stack element
/// Input: ${initLength}
/// Output: 1
///
/// Mirrors Java `NewListInstruction`.
pub struct NewListInstruction {
    error_reporter: Rc<dyn ErrorReporter>,
    init_length: usize,
}

impl NewListInstruction {
    /// 构造指令,对应 Java 构造器 `NewListInstruction`。
    pub fn new(error_reporter: Rc<dyn ErrorReporter>, init_length: usize) -> Self {
        NewListInstruction {
            error_reporter,
            init_length,
        }
    }

    /// 对应 Java 方法 `initLength`。
    pub fn init_length(&self) -> usize {
        self.init_length
    }
}

impl QLInstruction for NewListInstruction {
    /// 向下转型支持(供 api/parsecache Exporter 的 Java `instanceof` 分派)。
    fn as_any(&self) -> Option<&dyn std::any::Any> {
        Some(self)
    }

    fn execute(
        &self,
        q_context: &mut dyn QContext,
        _ql_options: &QLOptions,
    ) -> Result<QResult, QLException> {
        if let Some(budget) = q_context.q_runtime().execution_budget() {
            budget.charge_collection_items(self.init_length)?;
        }
        let init_items = q_context.pop_n(self.init_length);
        let list = init_items.values();
        q_context.push(QValue::Data(DataValue::list(list)));
        Ok(QResult::NEXT_INSTRUCTION)
    }

    fn stack_input(&self) -> i32 {
        self.init_length as i32
    }

    fn stack_output(&self) -> i32 {
        1
    }

    fn println(&self, index: usize, depth: usize, debug: &mut dyn FnMut(String)) {
        PrintlnUtils::println_by_cur_depth(
            depth as i32,
            &format!("{}: NewList {}", index, self.init_length),
            debug,
        );
    }

    fn error_reporter(&self) -> &Rc<dyn ErrorReporter> {
        &self.error_reporter
    }
}
