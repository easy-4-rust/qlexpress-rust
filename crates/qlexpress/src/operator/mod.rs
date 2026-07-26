//! Operator-related public types mirroring Java
//! `com.alibaba.qlexpress4.operator`. Stage 0 delivers only the
//! `OperatorCheckStrategy` family required by `CheckOptions`; the operator
//! implementations arrive in Stage 4.

pub mod black_operator_check_strategy;
pub mod default_operator_check_strategy;
pub mod operator_check_strategy;
pub mod white_operator_check_strategy;

pub use black_operator_check_strategy::BlackOperatorCheckStrategy;
pub use default_operator_check_strategy::DefaultOperatorCheckStrategy;
pub use operator_check_strategy::OperatorCheckStrategy;
pub use white_operator_check_strategy::WhiteOperatorCheckStrategy;
