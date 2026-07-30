//! Operator-related public types mirroring Java
//! `com.alibaba.qlexpress4.operator`. Stage 0 delivers only the
//! `OperatorCheckStrategy` family required by `CheckOptions`; the operator
//! implementations arrive in Stage 4.

pub mod binary_operator;
pub mod black_operator_check_strategy;
pub mod custom_binary_operator;
pub mod default_operator_check_strategy;
pub mod instance_of_operator;
// Java 的 operator.Operator 按“一对象一文件”规则映射为 operator/operator.rs。
#[allow(clippy::module_inception)]
pub mod operator;
pub mod operator_check_strategy;
pub mod operator_manager;
pub mod white_operator_check_strategy;

pub use crate::arithmetic;
pub use crate::assign;
pub use crate::base;
pub use crate::bit;
pub use crate::collection;
pub use crate::compare;
pub use crate::logic;
pub use crate::number;
pub use crate::string;
pub use crate::unary;
pub use binary_operator::BinaryOperator;
pub use black_operator_check_strategy::BlackOperatorCheckStrategy;
pub use custom_binary_operator::CustomBinaryOperator;
pub use default_operator_check_strategy::DefaultOperatorCheckStrategy;
pub use operator as contract;
pub use operator::Operator;
pub use operator_check_strategy::OperatorCheckStrategy;
pub use operator_manager::OperatorManager;
pub use unary::UnaryOperator;
pub use white_operator_check_strategy::WhiteOperatorCheckStrategy;
