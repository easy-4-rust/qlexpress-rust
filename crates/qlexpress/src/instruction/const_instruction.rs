//! 常量入栈指令,对应 Java `com.alibaba.qlexpress4.runtime.instruction.ConstInstruction`。
//! 职责:将常量对象压入操作数栈。
//! 本文件由 `const_inst.rs` 拆分而来(SPEC §5.5 一类一文件),仅移动代码与补充中文注释,行为完全一致。

use crate::exception::error_reporter::ErrorReporter;
use crate::exception::QLException;
use crate::ql_options::QLOptions;
use crate::runtime::instruction::{with_trace, QLInstruction};
use crate::runtime::q_result::QResult;
use crate::runtime::qcontext::QContext;
use crate::runtime::value::{DataValue, QValue};
use crate::utils::println_utils::PrintlnUtils;
use std::rc::Rc;

/// 常量入栈指令。对应 Java: com.alibaba.qlexpress4.runtime.instruction.ConstInstruction(职责:将常量对象压入操作数栈)
/// Operation: push constObj to stack
/// Input: 0
/// Output: 1
///
/// Mirrors Java `ConstInstruction`.
pub struct ConstInstruction {
    error_reporter: Rc<dyn ErrorReporter>,
    const_obj: DataValue,
    trace_key: Option<i32>,
}

impl ConstInstruction {
    /// 构造指令,对应 Java 构造器 `ConstInstruction`。
    pub fn new(
        error_reporter: Rc<dyn ErrorReporter>,
        const_obj: DataValue,
        trace_key: Option<i32>,
    ) -> Self {
        ConstInstruction {
            error_reporter,
            const_obj,
            trace_key,
        }
    }

    /// 对应 Java 方法 `constObj`。
    pub fn const_obj(&self) -> &DataValue {
        &self.const_obj
    }

    /// 对应 Java 方法 `traceKey`。
    pub fn trace_key(&self) -> Option<i32> {
        self.trace_key
    }
}

impl QLInstruction for ConstInstruction {
    /// 向下转型支持(供 api/parsecache Exporter 的 Java `instanceof` 分派)。
    fn as_any(&self) -> Option<&dyn std::any::Any> {
        Some(self)
    }

    fn execute(
        &self,
        q_context: &mut dyn QContext,
        _ql_options: &QLOptions,
    ) -> Result<QResult, QLException> {
        q_context.push(QValue::Data(self.const_obj.clone()));

        // trace
        with_trace(q_context, self.trace_key, |trace| {
            trace.value_evaluated(self.const_obj.clone());
        });

        Ok(QResult::NEXT_INSTRUCTION)
    }

    fn stack_input(&self) -> i32 {
        0
    }

    fn stack_output(&self) -> i32 {
        1
    }

    fn println(&self, index: usize, depth: usize, debug: &mut dyn FnMut(String)) {
        PrintlnUtils::println_by_cur_depth(
            depth as i32,
            &format!("{}: LoadConst {}", index, self.const_obj.string_value_of()),
            debug,
        );
    }

    fn error_reporter(&self) -> &Rc<dyn ErrorReporter> {
        &self.error_reporter
    }
}
