//! 对应 Java 类：com.alibaba.qlexpress4.runtime.operator.unary.UnaryOperator
//!
//! 一元操作符契约（前缀 `++` / `--` / `!` / `~` / `+` / `-`，
//! 后缀 `++` / `--`）。Java 中 `UnaryOperator` 是一个接口，定义在
//! `runtime/operator/unary/` 子包下（与 `BinaryOperator` 在顶层包不同），
//! 因此 Rust 侧的 `UnaryOperator` trait 也归位到 `runtime/operator/unary/`。
//!
//! 历史：该 trait 此前临时定义在 `runtime/operator/base/mod.rs`，
//! Stage 4b 收尾时按"一文件一对象"规范迁回此处；
//! `base::UnaryOperator` 路径通过 `pub use` 保留向后兼容。
//!
//! Java 原型（节选）：
//! ```java
//! public interface UnaryOperator extends Operator {
//!     Object execute(Value value, ErrorReporter errorReporter);
//! }
//! ```
//!
//! 注意：Java 的 `Operator` 接口（`getOperator`/`getPriority`）在 Rust 侧
//! 直接合并到本 trait 的 `operator` / `priority` 方法，避免多余的 trait 层级。

use crate::exception::error_reporter::ErrorReporter;
use crate::exception::QLException;
use crate::runtime::value::{DataValue, QValue};

/// 一元操作符接口，对应 Java
/// `com.alibaba.qlexpress4.runtime.operator.unary.UnaryOperator`
/// （unary operator, include: prefix operator / suffix operator;
/// Author: DQinYuan）。
///
/// Java 签名：`Object execute(Value value, ErrorReporter errorReporter)`
/// （`value` 为操作数，`errorReporter` 用于报错，返回操作结果）。
pub trait UnaryOperator {
    /// 执行一元运算。
    ///
    /// 对应 Java 方法：`execute(Value value, ErrorReporter errorReporter)`，
    /// 返回 Java `Object`。
    fn execute(
        &self,
        value: &QValue,
        error_reporter: &dyn ErrorReporter,
    ) -> Result<DataValue, QLException>;

    /// 对应 Java 方法：`Operator.getOperator()` —— 操作符词素（如 `"!"`）。
    fn operator(&self) -> &str;

    /// 对应 Java 方法：`Operator.getPriority()` —— 优先级。
    ///
    /// Java 实现里一元操作符报 `QLPrecedences.UNARY` 级别，
    /// 该值仅二元操作符会被查询。
    fn priority(&self) -> i32;
}
