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

/// 让闭包/函数指针直接充当自定义二元操作符,对应 Java 中以 lambda 实现
/// `CustomBinaryOperator` 函数式接口的写法
/// (`Express4Runner.addOperator(operator, (left, right) -> ...)`)。
impl<F> CustomBinaryOperator for F
where
    F: Fn(&QValue, &QValue) -> Result<DataValue, QLException>,
{
    fn execute(&self, left: &QValue, right: &QValue) -> Result<DataValue, QLException> {
        (self)(left, right)
    }
}
