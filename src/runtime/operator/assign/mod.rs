//! 赋值操作符,对应 Java `com.alibaba.qlexpress4.runtime.operator.assign` 包。
//!
//! 仅做 mod 声明与 re-export(SPEC §5.5:mod.rs 禁止实现)。

pub mod assign_operator;

pub use assign_operator::AssignOperator;
