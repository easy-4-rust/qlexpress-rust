//! Operator base traits, mirroring Java
//! `com.alibaba.qlexpress4.runtime.operator.base`.

use crate::exception::error_reporter::ErrorReporter;
use crate::exception::QLException;
use crate::ql_options::QLOptions;
use crate::runtime::qcontext::QContext;
use crate::runtime::value::{DataValue, QValue};

/// Binary (middle) operator, mirroring Java `BinaryOperator`.
///
/// `CustomBinaryOperator` in Java extends this for user-defined operators;
/// both are dispatched identically by `OperatorInstruction`.
pub trait BinaryOperator {
    /// Java `execute(Value left, Value right, QContext, QLOptions,
    /// ErrorReporter)`; returns the operation result (Java `Object`).
    fn execute(
        &self,
        left: &QValue,
        right: &QValue,
        q_context: &mut dyn QContext,
        ql_options: &QLOptions,
        error_reporter: &dyn ErrorReporter,
    ) -> Result<DataValue, QLException>;

    /// Java `getOperator()`: the operator lexeme (e.g. `"+"`).
    fn operator(&self) -> &str;

    /// Java `Operator.getPriority()`: operator precedence
    /// (see `QLPrecedences`).
    fn priority(&self) -> i32;
}

/// Unary operator, mirroring Java `UnaryOperator` (`++`, `--`, `!`, `~`,
/// unary `-`/`+`).
pub trait UnaryOperator {
    /// Java `execute(Value value, ErrorReporter)`; returns the operation
    /// result (Java `Object`).
    fn execute(
        &self,
        value: &QValue,
        error_reporter: &dyn ErrorReporter,
    ) -> Result<DataValue, QLException>;

    /// Java `getOperator()`: the operator lexeme (e.g. `"!"`).
    fn operator(&self) -> &str;

    /// Java `Operator.getPriority()`: operator precedence. Unary operators
    /// report `QLPrecedences.PRIORITY unary` level in Java's
    /// implementations; the value is only consulted for binary operators.
    fn priority(&self) -> i32;
}
