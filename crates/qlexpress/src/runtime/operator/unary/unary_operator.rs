//! 一元操作符契约,对应 Java
//! `com.alibaba.qlexpress4.runtime.operator.unary.UnaryOperator`。

/// 一元操作符接口,对应 Java:
/// com.alibaba.qlexpress4.runtime.operator.unary.UnaryOperator
/// (unary operator, include: prefix operator / suffix operator;
/// Author: DQinYuan)。
///
/// Java 签名:`Object execute(Value value, ErrorReporter errorReporter)`
/// (`value` 为操作数,`errorReporter` 用于报错,返回操作结果)。
///
/// Rust 侧该 trait 与 `BinaryOperator` 一同定义在
/// [`crate::runtime::operator::base`](Java `base` 包)中,此处 re-export
/// 以保持与 Java `unary` 包的一一对应。
pub use crate::runtime::operator::base::UnaryOperator;
