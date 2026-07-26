//! Operator-related public types mirroring Java
//! `com.alibaba.qlexpress4.operator`. Stage 0 delivers only the
//! `OperatorCheckStrategy` family required by `CheckOptions`; the operator
//! implementations arrive in Stage 4.

pub mod operator_check_strategy;

pub use operator_check_strategy::{
    BlackOperatorCheckStrategy, DefaultOperatorCheckStrategy, OperatorCheckStrategy,
    WhiteOperatorCheckStrategy,
};
