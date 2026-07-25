//! User-defined binary operator contract, mirroring Java
//! `com.alibaba.qlexpress4.runtime.operator.CustomBinaryOperator`.

use crate::exception::QLException;
use crate::runtime::value::{DataValue, QValue};

/// A user-defined binary operator, mirroring Java `CustomBinaryOperator`.
///
/// Registered through
/// [`OperatorManager::add_binary_operator`](crate::aparser::operator_factory::OperatorManager::add_binary_operator)
/// and adapted into a [`BinaryOperator`](super::base::BinaryOperator) by the
/// operator manager (Java `OperatorManager.adapt2BinOp`).
pub trait CustomBinaryOperator {
    /// Java `execute(Value left, Value right)`.
    ///
    /// Java throws `UserDefineException` for a custom error message and any
    /// other `Throwable` for an internal failure; the Rust port folds both
    /// into [`QLException`] (the operator manager wraps it with the
    /// `OPERATOR_INNER_EXCEPTION` code, mirroring Java's
    /// `ThrowUtils.wrapThrowable`).
    fn execute(&self, left: &QValue, right: &QValue) -> Result<DataValue, QLException>;
}
