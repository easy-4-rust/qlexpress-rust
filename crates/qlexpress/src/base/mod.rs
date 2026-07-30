//! 操作符基类包，对应 Java `com.alibaba.qlexpress4.runtime.operator.base`。
//!
//! 仅承载 Java `base` 子包下的两个抽象基类（`BaseBinaryOperator` /
//! `BaseUnaryOperator`）。一元/二元操作符的 trait 定义分别归位：
//! - `BinaryOperator` trait → `runtime/operator/binary_operator.rs`（Java 顶层包）
//! - `UnaryOperator` trait → `runtime/operator/unary/unary_operator.rs`（Java unary 子包）
//!
//! 本文件仅做模块声明与 re-export，符合"mod.rs 不定义类型"的规范（SPEC §2）。
//! 下方 `pub use` 仅为兼容历史 `base::UnaryOperator` / `base::BinaryOperator` 引用。

pub mod base_binary_operator;
pub mod base_unary_operator;

pub use crate::operator::binary_operator::BinaryOperator;
pub use base_binary_operator::BaseBinaryOperator;
pub use base_unary_operator::BaseUnaryOperator;
// 兼容历史 `base::UnaryOperator` 引用：trait 真身已迁回 `unary/unary_operator.rs`。
pub use crate::unary::UnaryOperator;
