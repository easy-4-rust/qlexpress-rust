//! 二元运算指令,对应 Java `com.alibaba.qlexpress4.runtime.instruction.OperatorInstruction`。
//! 职责:执行二元运算符。
//! 本文件由 `unary_binary.rs` 拆分而来(SPEC §5.5 一类一文件),仅移动代码与补充中文注释,行为完全一致。

use crate::exception::error_codes;
use crate::exception::error_reporter::ErrorReporter;
use crate::exception::QLException;
use crate::ql_options::QLOptions;
use crate::runtime::instruction::{with_trace, QLInstruction};
use crate::runtime::operator::base::BinaryOperator;
use crate::runtime::q_result::QResult;
use crate::runtime::qcontext::QContext;
use crate::runtime::util::throw_utils::wrap_throwable;
use crate::runtime::value::QValue;
use crate::utils::println_utils::PrintlnUtils;
use std::rc::Rc;

/// 二元运算指令。对应 Java: com.alibaba.qlexpress4.runtime.instruction.OperatorInstruction(职责:执行二元运算符)
/// Operation: do middle operator +=,&gt;&gt;,&gt;&gt;&gt;,&lt;&lt;,.
/// Input: 2
/// Output: 1, operator result
///
/// Mirrors Java `OperatorInstruction`.
pub struct OperatorInstruction {
    error_reporter: Rc<dyn ErrorReporter>,
    operator: Rc<dyn BinaryOperator>,
    trace_key: Option<i32>,
}

impl OperatorInstruction {
    /// 构造指令,对应 Java 构造器 `OperatorInstruction`。
    pub fn new(
        error_reporter: Rc<dyn ErrorReporter>,
        operator: Rc<dyn BinaryOperator>,
        trace_key: Option<i32>,
    ) -> Self {
        OperatorInstruction {
            error_reporter,
            operator,
            trace_key,
        }
    }

    /// 对应 Java 方法 `operator`。
    pub fn operator(&self) -> &Rc<dyn BinaryOperator> {
        &self.operator
    }

    /// 对应 Java 方法 `traceKey`。
    pub fn trace_key(&self) -> Option<i32> {
        self.trace_key
    }
}

impl QLInstruction for OperatorInstruction {
    /// 向下转型支持(供 api/parsecache Exporter 的 Java `instanceof` 分派)。
    fn as_any(&self) -> Option<&dyn std::any::Any> {
        Some(self)
    }

    fn execute(
        &self,
        q_context: &mut dyn QContext,
        ql_options: &QLOptions,
    ) -> Result<QResult, QLException> {
        let right_value = q_context.pop();
        let left_value = q_context.pop();
        // Java traces the children before executing the operator (inside
        // the same try block).
        with_trace(q_context, self.trace_key, |trace| {
            if let Some(child) = trace.children_mut().get_mut(0) {
                child.value_evaluated(left_value.get());
            }
            if let Some(child) = trace.children_mut().get_mut(1) {
                child.value_evaluated(right_value.get());
            }
        });
        let operator_result = self.operator.execute(
            &left_value,
            &right_value,
            q_context,
            ql_options,
            &*self.error_reporter,
        );
        match operator_result {
            Ok(result) => {
                q_context.push(QValue::Data(result.clone()));

                // trace result
                with_trace(q_context, self.trace_key, |trace| {
                    trace.value_evaluated(result);
                });

                Ok(QResult::NEXT_INSTRUCTION)
            }
            Err(err) => Err(wrap_throwable(
                err,
                &*self.error_reporter,
                error_codes::EXECUTE_OPERATOR_EXCEPTION,
                error_codes::error_msg(error_codes::EXECUTE_OPERATOR_EXCEPTION),
                &[
                    left_value.get().string_value_of(),
                    self.operator.operator().to_string(),
                    right_value.get().string_value_of(),
                ],
            )),
        }
    }

    fn stack_input(&self) -> i32 {
        2
    }

    fn stack_output(&self) -> i32 {
        1
    }

    fn println(&self, index: usize, depth: usize, debug: &mut dyn FnMut(String)) {
        PrintlnUtils::println_by_cur_depth(
            depth as i32,
            &format!("{}: Operator {}", index, self.operator.operator()),
            debug,
        );
    }

    fn error_reporter(&self) -> &Rc<dyn ErrorReporter> {
        &self.error_reporter
    }
}
