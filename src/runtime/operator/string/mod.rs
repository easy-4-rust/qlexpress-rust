//! 字符串操作符,对应 Java `com.alibaba.qlexpress4.runtime.operator.string` 包。
//!
//! 仅做 mod 声明与 re-export(SPEC §5.5:mod.rs 禁止实现)。

pub mod like_operator;
pub mod not_like_operator;

pub use like_operator::LikeOperator;
pub use not_like_operator::NotLikeOperator;
