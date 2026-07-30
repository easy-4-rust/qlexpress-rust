//! 操作符顶层 trait(仅词素与优先级)。
//!
//! 对应 Java: com.alibaba.qlexpress4.runtime.operator.Operator
//! (interface Operator:`getOperator()` / `getPriority()`)。
//!
//! Rust 说明:为避免改动既有 `BinaryOperator`/`UnaryOperator` 的实现方
//! (trait 合方法,Rust 不能在子 trait impl 块中实现超 trait 方法),这里
//! 用 blanket impl 让所有 `BinaryOperator` 自动成为 `Operator`;
//! 一元操作符由各实现类自行实现本 trait(Stage 4a 范围内为
//! BitwiseInvertOperator / LogicNotOperator)。

use super::binary_operator::BinaryOperator;

/// 操作符接口。
///
/// 对应 Java: Operator(操作符接口,@author bingo)。
pub trait Operator {
    /// 对应 Java 方法: `getOperator()` —— 返回操作符词素。
    fn operator(&self) -> &str;

    /// 对应 Java 方法: `getPriority()` —— 返回操作符优先级。
    fn priority(&self) -> i32;
}

/// 所有二元操作符自动实现 Operator(Java 的 extends 关系)。
impl<T: BinaryOperator + ?Sized> Operator for T {
    fn operator(&self) -> &str {
        BinaryOperator::operator(self)
    }

    fn priority(&self) -> i32 {
        BinaryOperator::priority(self)
    }
}
