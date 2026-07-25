//! Script operators, mirroring Java `com.alibaba.qlexpress4.runtime.operator`.
//!
//! Stage 3a delivers only the base traits (`base/` in Java) required by
//! `UnaryInstruction`/`OperatorInstruction`; concrete operator
//! implementations arrive in Stage 4 (`src/operator/`).

pub mod base;
pub mod custom_binary_operator;

pub use base::{BinaryOperator, UnaryOperator};
pub use custom_binary_operator::CustomBinaryOperator;
