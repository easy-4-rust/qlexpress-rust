//! trace 求值记录指令,对应 Java `com.alibaba.qlexpress4.runtime.instruction.TraceEvaluatedInstruction`。
//! 职责:记录表达式求值结果到 trace。
//! 本文件由 `trace.rs` 拆分而来(SPEC §5.5 一类一文件),仅移动代码与补充中文注释,行为完全一致。

use crate::exception::error_reporter::ErrorReporter;
use crate::exception::QLException;
use crate::ql_options::QLOptions;
use crate::runtime::instruction::{with_trace, QLInstruction};
use crate::runtime::q_result::QResult;
use crate::runtime::qcontext::QContext;
use crate::runtime::value::DataValue;
use crate::utils::println_utils::PrintlnUtils;
use std::rc::Rc;

/// trace 求值记录指令。对应 Java: com.alibaba.qlexpress4.runtime.instruction.TraceEvaluatedInstruction(职责:记录表达式求值结果到 trace)
/// Operation: no op, only for marking evaludated as true
/// Input: 0
/// Output: 0
///
/// Mirrors Java `TraceEvaluatedInstruction`.
pub struct TraceEvaluatedInstruction {
    error_reporter: Rc<dyn ErrorReporter>,
    trace_key: Option<i32>,
}

impl TraceEvaluatedInstruction {
    /// 构造指令,对应 Java 构造器 `TraceEvaluatedInstruction`。
    pub fn new(error_reporter: Rc<dyn ErrorReporter>, trace_key: Option<i32>) -> Self {
        TraceEvaluatedInstruction {
            error_reporter,
            trace_key,
        }
    }

    /// 对应 Java 方法 `traceKey`。
    pub fn trace_key(&self) -> Option<i32> {
        self.trace_key
    }
}

impl QLInstruction for TraceEvaluatedInstruction {
    /// 向下转型支持(供 api/parsecache Exporter 的 Java `instanceof` 分派)。
    fn as_any(&self) -> Option<&dyn std::any::Any> {
        Some(self)
    }

    fn execute(
        &self,
        q_context: &mut dyn QContext,
        _ql_options: &QLOptions,
    ) -> Result<QResult, QLException> {
        with_trace(q_context, self.trace_key, |trace| {
            // Java valueEvaluated(null) marks evaluated without a value.
            trace.value_evaluated(DataValue::Null);
        });
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
            &format!(
                "{}: TraceEvaludated {}",
                index,
                self.trace_key
                    .map(|k| k.to_string())
                    .unwrap_or_else(|| "null".to_string())
            ),
            debug,
        );
    }

    fn error_reporter(&self) -> &Rc<dyn ErrorReporter> {
        &self.error_reporter
    }
}
