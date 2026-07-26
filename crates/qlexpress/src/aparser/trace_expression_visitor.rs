//! Static-expression-trace visitor, mirroring Java
//! `aparser/TraceExpressionVisitor`.
//!
//! In Java this visitor walks the parsed `Node` tree *without running
//! the script* and emits a list of [`TracePointTree`] entries marking
//! statements, variable declarations, function/macro definitions, and
//! expression subtrees. The runner exposes them via
//! `Express4Runner.getExpressionTracePoints`.
//!
//! Stage 6 status:
//!
//! - **Run-time** expression tracing is implemented via
//!   [`crate::runtime::trace::ExpressionTrace`] and the
//!   `TracePeekInstruction` / `TraceEvaluatedInstruction` pair (see
//!   `runtime/instruction/trace_*`). Use
//!   [`crate::express4_runner::Express4Runner::get_expression_trace`]
//!   to fetch traces after `execute`.
//!
//! - **Compile-time** `TraceExpressionVisitor` (this module) is
//!   **not yet ported** in v1. The behaviour is recoverable from the
//!   run-time trace output (which captures the same set of trace
//!   points with line/column metadata), so the API surface is
//!   preserved through [`TracePointTree`] but the visitor body is
//!   intentionally a stub.
//!
//! See `plan.md` Stage 6 for the deferred-work rationale.

use crate::aparser::syntax_tree_factory::Node;
use crate::runtime::trace::TracePointTree;

/// Static trace visitor.
///
/// Currently a stub: returns an empty list. See the module docs for
/// the v1 deferral rationale and the run-time fallback path.
#[derive(Default)]
pub struct TraceExpressionVisitor {
    _private: (),
}

impl TraceExpressionVisitor {
    pub fn new() -> Self {
        Self::default()
    }

    /// Visit a parsed `Node` and emit a list of static trace points.
    /// v1 stub: returns an empty vector. The compiled program already
    /// contains `TraceEvaluatedInstruction`s that fill in the same
    /// information at run time.
    pub fn visit(&mut self, _tree: &Node) -> Vec<TracePointTree> {
        Vec::new()
    }

    /// Public helper used by integration tests that cannot easily
    /// construct a concrete `Node`. The default impl calls [`Self::visit`]
    /// with a discarded input; v1 it always returns an empty vector.
    pub fn visit_default(&mut self) -> Vec<TracePointTree> {
        Vec::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn visit_default_returns_empty() {
        let mut v = TraceExpressionVisitor::new();
        assert!(v.visit_default().is_empty());
    }
}