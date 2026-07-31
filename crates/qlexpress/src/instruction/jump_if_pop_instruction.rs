//! 条件跳转弹栈指令,对应 Java `com.alibaba.qlexpress4.runtime.instruction.JumpIfPopInstruction`。
//! 职责:弹出栈顶布尔值,满足期望时跳转。
//! 本文件由 `flow.rs` 拆分而来(SPEC §5.5 一类一文件),仅移动代码与补充中文注释,行为完全一致。

use crate::exception::QLException;
use crate::exception::error_codes;
use crate::exception::error_reporter::ErrorReporter;
use crate::ql_options::QLOptions;
use crate::runtime::instruction::QLInstruction;
use crate::runtime::q_result::QResult;
use crate::runtime::qcontext::QContext;
use crate::runtime::value::DataValue;
use crate::utils::println_utils::PrintlnUtils;
use std::cell::Cell;
use std::rc::Rc;

/// 条件跳转弹栈指令。对应 Java: com.alibaba.qlexpress4.runtime.instruction.JumpIfPopInstruction(职责:弹出栈顶布尔值,满足期望时跳转)
/// Operation: pop the top of stack, if the element is ${expect}, jump to
/// position, else execute next instruction as normal. jump if null
/// Input: 1
/// Output: 0
///
/// Mirrors Java `JumpIfPopInstruction`.
pub struct JumpIfPopInstruction {
    error_reporter: Rc<dyn ErrorReporter>,
    expect: bool,
    position: Cell<i32>,
}

impl JumpIfPopInstruction {
    /// 构造指令,对应 Java 构造器 `JumpIfPopInstruction`。
    pub fn new(error_reporter: Rc<dyn ErrorReporter>, expect: bool, position: i32) -> Self {
        JumpIfPopInstruction {
            error_reporter,
            expect,
            position: Cell::new(position),
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

    /// Java `conditionToBool`: `null` becomes `expect`, non-Boolean errors.
    fn condition_to_bool(&self, condition: &DataValue) -> Result<bool, QLException> {
        if condition.is_null() {
            return Ok(self.expect);
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

impl QLInstruction for JumpIfPopInstruction {
    /// 向下转型支持(供 api/parsecache Exporter 的 Java `instanceof` 分派)。
    fn as_any(&self) -> Option<&dyn std::any::Any> {
        Some(self)
    }

    fn execute(
        &self,
        q_context: &mut dyn QContext,
        _ql_options: &QLOptions,
    ) -> Result<QResult, QLException> {
        let condition_bool = self.condition_to_bool(&q_context.pop().get())?;
        if condition_bool == self.expect {
            Ok(QResult::Jump(self.position.get()))
        } else {
            Ok(QResult::NEXT_INSTRUCTION)
        }
    }

    fn stack_input(&self) -> i32 {
        1
    }

    fn stack_output(&self) -> i32 {
        0
    }

    fn println(&self, index: usize, depth: usize, debug: &mut dyn FnMut(String)) {
        PrintlnUtils::println_by_cur_depth(
            depth as i32,
            &format!(
                "{}: JumpIfPop {} {}",
                index,
                self.expect,
                self.position.get()
            ),
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
