//! User-defined binary operator contract, mirroring Java
//! `com.alibaba.qlexpress4.runtime.operator.CustomBinaryOperator`.

use crate::exception::QLException;
use crate::runtime::value::{DataValue, QValue};

/// `CustomBinaryOperator` 接口的 Rust 实现，保留对应对象的领域职责与公开契约。
/// 对应或承接 Java 源文件：`com/alibaba/qlexpress4/runtime/operator/CustomBinaryOperator.java`；具体对象路径见 `docs/对象级对照表.md`。
/// A user-defined binary operator, mirroring Java `CustomBinaryOperator`.
///
/// Registered through
/// [`OperatorManager::add_binary_operator`](crate::aparser::operator_factory::OperatorManager::add_binary_operator)
/// and adapted into a [`BinaryOperator`](super::base::BinaryOperator) by the
/// operator manager (Java `OperatorManager.adapt2BinOp`).
/// 对应 Java: com.alibaba.qlexpress4.runtime.operator.CustomBinaryOperator。
pub trait CustomBinaryOperator {
    /// 处理 execute 对应的接口职责。
    /// 参数：`left`、`right`；返回：`Result<DataValue, QLException>`。
    /// 对应或承接 Java 源文件：`com/alibaba/qlexpress4/runtime/operator/CustomBinaryOperator.java`，方法 `execute`。
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
