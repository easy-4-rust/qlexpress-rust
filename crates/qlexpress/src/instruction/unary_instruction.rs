//! 一元运算指令,对应 Java `com.alibaba.qlexpress4.runtime.instruction.UnaryInstruction`。
//! 职责:执行一元运算符。
//! 本文件由 `unary_binary.rs` 拆分而来(SPEC §5.5 一类一文件),仅移动代码与补充中文注释,行为完全一致。

use crate::exception::QLException;
use crate::exception::error_reporter::ErrorReporter;
use crate::ql_options::QLOptions;
use crate::runtime::instruction::{QLInstruction, with_trace};
use crate::runtime::operator::base::UnaryOperator;
use crate::runtime::q_result::QResult;
use crate::runtime::qcontext::QContext;
use crate::runtime::value::QValue;
use crate::utils::println_utils::PrintlnUtils;
use std::rc::Rc;

/// 一元运算指令。对应 Java: com.alibaba.qlexpress4.runtime.instruction.UnaryInstruction(职责:执行一元运算符)
/// Operation: do unary operator like, ++,--,!,~
/// Input: 1
/// Output: 1, unary result
///
/// Mirrors Java `UnaryInstruction`.
pub struct UnaryInstruction {
    error_reporter: Rc<dyn ErrorReporter>,
    unary_operator: Rc<dyn UnaryOperator>,
    trace_key: Option<i32>,
}

impl UnaryInstruction {
    /// 构造指令,对应 Java 构造器 `UnaryInstruction`。
    pub fn new(
        error_reporter: Rc<dyn ErrorReporter>,
        unary_operator: Rc<dyn UnaryOperator>,
        trace_key: Option<i32>,
    ) -> Self {
        UnaryInstruction {
            error_reporter,
            unary_operator,
            trace_key,
        }
    }

    /// 对应 Java 方法 `unaryOperator`。
    pub fn unary_operator(&self) -> &Rc<dyn UnaryOperator> {
        &self.unary_operator
    }

    /// 对应 Java 方法 `traceKey`。
    pub fn trace_key(&self) -> Option<i32> {
        self.trace_key
    }
}

impl QLInstruction for UnaryInstruction {
    /// 向下转型支持(供 api/parsecache Exporter 的 Java `instanceof` 分派)。
    fn as_any(&self) -> Option<&dyn std::any::Any> {
        Some(self)
    }

    fn execute(
        &self,
        q_context: &mut dyn QContext,
        _ql_options: &QLOptions,
    ) -> Result<QResult, QLException> {
        let value = q_context.pop();
        let result = self.unary_operator.execute(&value, &*self.error_reporter)?;
        q_context.push(QValue::Data(result.clone()));

        // trace
        with_trace(q_context, self.trace_key, |trace| {
            trace.value_evaluated(result);
            if let Some(child) = trace.children_mut().get_mut(0) {
                child.value_evaluated(value.get());
            }
        });

        Ok(QResult::NEXT_INSTRUCTION)
    }

    fn stack_input(&self) -> i32 {
        1
    }

    fn stack_output(&self) -> i32 {
        1
    }

    fn println(&self, index: usize, depth: usize, debug: &mut dyn FnMut(String)) {
        PrintlnUtils::println_by_cur_depth(
            depth as i32,
            &format!("{}: UnaryOp {}", index, self.unary_operator.operator()),
            debug,
        );
    }

    fn error_reporter(&self) -> &Rc<dyn ErrorReporter> {
        &self.error_reporter
    }
}
