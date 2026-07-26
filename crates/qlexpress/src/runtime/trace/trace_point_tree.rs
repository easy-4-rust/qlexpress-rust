//! Trace point tree node, mirroring Java `TracePointTree`.
//!
//! In Java this is the return type of `TraceExpressionVisitor`: each
//! visited syntax subtree produces a tree node with `TraceType`,
//! line/column, and a list of child nodes.
//!
//! In Rust the equivalent representation already lives in
//! [`ExpressionTrace`] (which carries `trace_type`, `line`, `col`,
//! `children`, etc.). To preserve the Java symbol name without
//! duplicating the type, we re-export it here.

pub use super::expression_trace::ExpressionTrace as TracePointTree;