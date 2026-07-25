//! 一元操作符,对应 Java `com.alibaba.qlexpress4.runtime.operator.unary` 包。
//!
//! 仅做 mod 声明与 re-export(SPEC §5.5:mod.rs 禁止实现)。

pub mod minus_minus_prefix_unary_operator;
pub mod minus_minus_suffix_unary_operator;
pub mod minus_unary_operator;
pub mod plus_plus_prefix_unary_operator;
pub mod plus_plus_suffix_unary_operator;
pub mod plus_unary_operator;
pub mod unary_operator;

pub use minus_minus_prefix_unary_operator::MinusMinusPrefixUnaryOperator;
pub use minus_minus_suffix_unary_operator::MinusMinusSuffixUnaryOperator;
pub use minus_unary_operator::MinusUnaryOperator;
pub use plus_plus_prefix_unary_operator::PlusPlusPrefixUnaryOperator;
pub use plus_plus_suffix_unary_operator::PlusPlusSuffixUnaryOperator;
pub use plus_unary_operator::PlusUnaryOperator;
pub use unary_operator::UnaryOperator;
