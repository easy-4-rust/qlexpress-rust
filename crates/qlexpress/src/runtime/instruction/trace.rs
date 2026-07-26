//! Trace no-op instructions, mirroring Java `TraceEvaluatedInstruction`
//! and `TracePeekInstruction`.

use std::rc::Rc;

use crate::exception::error_reporter::ErrorReporter;
use crate::exception::QLException;
use crate::ql_options::QLOptions;
use crate::ql_result::QResult;
use crate::runtime::instruction::{with_trace, QLInstruction};
use crate::runtime::qcontext::QContext;
use crate::runtime::value::DataValue;
use crate::utils::println_utils::PrintlnUtils;

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
    pub fn new(error_reporter: Rc<dyn ErrorReporter>, trace_key: Option<i32>) -> Self {
        TraceEvaluatedInstruction {
            error_reporter,
            trace_key,
        }
    }

    pub fn trace_key(&self) -> Option<i32> {
        self.trace_key
    }
}

impl QLInstruction for TraceEvaluatedInstruction {
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

/// Operation: no op, only for tracing peek value of stack
/// Input: 0
/// Output: 0
///
/// Mirrors Java `TracePeekInstruction`.
pub struct TracePeekInstruction {
    error_reporter: Rc<dyn ErrorReporter>,
    trace_key: Option<i32>,
}

impl TracePeekInstruction {
    pub fn new(error_reporter: Rc<dyn ErrorReporter>, trace_key: Option<i32>) -> Self {
        TracePeekInstruction {
            error_reporter,
            trace_key,
        }
    }

    pub fn trace_key(&self) -> Option<i32> {
        self.trace_key
    }
}

impl QLInstruction for TracePeekInstruction {
    fn execute(
        &self,
        q_context: &mut dyn QContext,
        _ql_options: &QLOptions,
    ) -> Result<QResult, QLException> {
        // Java peeks only when the trace point exists.
        if q_context
            .traces()
            .get_expression_trace_by_key(self.trace_key)
            .is_some()
        {
            let peeked = q_context.peek().get();
            with_trace(q_context, self.trace_key, |trace| {
                trace.value_evaluated(peeked);
            });
        }
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
                "{}: TracePeek {}",
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
