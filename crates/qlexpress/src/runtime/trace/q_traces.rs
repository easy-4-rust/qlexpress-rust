//! Trace registry, mirroring Java `com.alibaba.qlexpress4.runtime.trace.QTraces`.

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use crate::runtime::trace::ExpressionTrace;

/// Holds the expression trace tree plus the `traceKey → trace point` index,
/// mirroring Java `QTraces`. Trace points are shared (`Rc<RefCell<_>>`) so
/// instructions can mark them evaluated while the tree stays readable.
#[derive(Clone, Debug, Default)]
pub struct QTraces {
    expression_traces: Vec<Rc<RefCell<ExpressionTrace>>>,
    expression_trace_map: HashMap<i32, Rc<RefCell<ExpressionTrace>>>,
}

impl QTraces {
    pub fn new(
        expression_traces: Vec<Rc<RefCell<ExpressionTrace>>>,
        expression_trace_map: HashMap<i32, Rc<RefCell<ExpressionTrace>>>,
    ) -> Self {
        QTraces {
            expression_traces,
            expression_trace_map,
        }
    }

    /// Empty registry (Java: traces with a null map — every lookup misses).
    pub fn empty() -> Self {
        QTraces::default()
    }

    /// Java `getExpressionTraceByKey(Integer)`: `None` when the key is
    /// `None` or unknown.
    pub fn get_expression_trace_by_key(
        &self,
        trace_key: Option<i32>,
    ) -> Option<Rc<RefCell<ExpressionTrace>>> {
        trace_key.and_then(|key| self.expression_trace_map.get(&key).map(Rc::clone))
    }

    /// Java `getExpressionTraces()`.
    pub fn expression_traces(&self) -> &[Rc<RefCell<ExpressionTrace>>] {
        &self.expression_traces
    }

    /// Snapshot of the trace trees (owned copies), for building `QLResult`.
    pub fn snapshot(&self) -> Vec<ExpressionTrace> {
        self.expression_traces
            .iter()
            .map(|trace| trace.borrow().clone())
            .collect()
    }
}
