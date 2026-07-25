//! 条件跳转指令,对应 Java `com.alibaba.qlexpress4.runtime.instruction.JumpIfInstruction`。
//! 职责:栈顶布尔值满足期望时跳转,不弹栈。
//! 本文件由 `flow.rs` 拆分而来(SPEC §5.5 一类一文件),仅移动代码与补充中文注释,行为完全一致。

use std::cell::Cell;
use std::rc::Rc;
use crate::exception::error_codes;
use crate::exception::error_reporter::ErrorReporter;
use crate::exception::QLException;
use crate::ql_options::QLOptions;
use crate::runtime::q_result::QResult;
use crate::runtime::instruction::{with_trace, QLInstruction};
use crate::runtime::qcontext::QContext;
use crate::runtime::value::DataValue;
use crate::utils::println_utils::PrintlnUtils;

/// 条件跳转指令。对应 Java: com.alibaba.qlexpress4.runtime.instruction.JumpIfInstruction(职责:栈顶布尔值满足期望时跳转,不弹栈)
/// Operation: if the element is ${expect}, jump to position, else execute
/// next instruction as normal. not jump if null
/// Input: 0
/// Output: 0
///
/// Mirrors Java `JumpIfInstruction`.
pub struct JumpIfInstruction {
    error_reporter: Rc<dyn ErrorReporter>,
    expect: bool,
    position: Cell<i32>,
    trace_key: Option<i32>,
}

impl JumpIfInstruction {
    /// 构造指令,对应 Java 构造器 `JumpIfInstruction`。
    pub fn new(
        error_reporter: Rc<dyn ErrorReporter>,
        expect: bool,
        position: i32,
        trace_key: Option<i32>,
    ) -> Self {
        JumpIfInstruction {
            error_reporter,
            expect,
            position: Cell::new(position),
            trace_key,
        }
    }

    /// 对应 Java 方法 `setPosition`。
    pub fn set_position(&self, position: i32) {
        self.position.set(position);
    }

    /// 对应 Java 方法 `isExpect`。
    pub fn is_expect(&self) -> bool {
        self.expect
    }

    /// 对应 Java 方法 `position`。
    pub fn position(&self) -> i32 {
        self.position.get()
    }

    /// 对应 Java 方法 `traceKey`。
    pub fn trace_key(&self) -> Option<i32> {
        self.trace_key
    }

    /// Java `conditionToBool`: `null` becomes `!expect`, non-Boolean is an
    /// error.
    fn condition_to_bool(&self, condition: &DataValue) -> Result<bool, QLException> {
        if condition.is_null() {
            return Ok(!self.expect);
        }
        match condition {
            DataValue::Bool(b) => Ok(*b),
            _ => Err(self.error_reporter.report(
                error_codes::CONDITION_BOOL_REQUIRED,
                error_codes::error_msg(error_codes::CONDITION_BOOL_REQUIRED),
            )),
        }
    }
}

impl QLInstruction for JumpIfInstruction {
    /// 向下转型支持(供 api/parsecache Exporter 的 Java `instanceof` 分派)。
    fn as_any(&self) -> Option<&dyn std::any::Any> {
        Some(self)
    }

    fn execute(
        &self,
        q_context: &mut dyn QContext,
        ql_options: &QLOptions,
    ) -> Result<QResult, QLException> {
        let condition_bool = self.condition_to_bool(&q_context.peek().get())?;
        if condition_bool == self.expect && !ql_options.is_short_circuit_disable() {
            // short circuit; trace
            with_trace(q_context, self.trace_key, |trace| {
                trace.value_evaluated(DataValue::Bool(condition_bool));
                if let Some(child) = trace.children_mut().get_mut(0) {
                    child.value_evaluated(DataValue::Bool(condition_bool));
                }
            });
            Ok(QResult::Jump(self.position.get()))
        } else {
            Ok(QResult::NEXT_INSTRUCTION)
        }
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
            &format!("{}: JumpIf {} {}", index, self.expect, self.position.get()),
            debug,
        );
    }

    fn conditional_jump(&self) -> Option<i32> {
        Some(self.position.get())
    }

    fn error_reporter(&self) -> &Rc<dyn ErrorReporter> {
        &self.error_reporter
    }
}

