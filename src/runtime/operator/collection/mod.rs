//! 集合包含操作符,对应 Java
//! `com.alibaba.qlexpress4.runtime.operator.collection` 包。
//!
//! 仅做 mod 声明与 re-export(SPEC §5.5:mod.rs 禁止实现)。

pub mod in_operator;
pub mod not_in_operator;

pub use in_operator::InOperator;
pub use not_in_operator::NotInOperator;
