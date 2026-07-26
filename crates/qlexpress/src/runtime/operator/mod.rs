//! 脚本操作符包,对应 Java `com.alibaba.qlexpress4.runtime.operator`。
//!
//! Stage 4a 交付:operator.rs / binary_operator.rs / operator_manager.rs /
//! instance_of_operator.rs 与 base/ arithmetic/ bit/ compare/ logic/
//! number/ 六个子包。
//!
//! 说明:assign/ collection/ string/ unary/ 四个子包由其他 Stage 4
//! agent 交付(本 agent 仅声明挂载,不拥有其内容);UnaryOperator trait
//! 本体暂存 base/mod.rs,unary/unary_operator.rs 为 re-export。

pub mod arithmetic;
pub mod assign;
pub mod base;
pub mod binary_operator;
pub mod bit;
pub mod collection;
pub mod compare;
#[path = "operator.rs"]
pub mod contract;
pub mod custom_binary_operator;
pub mod instance_of_operator;
pub mod logic;
pub mod number;
pub mod operator_manager;
pub mod string;
pub mod unary;

pub use base::{BinaryOperator, UnaryOperator};
pub use contract::Operator;
pub use custom_binary_operator::CustomBinaryOperator;
pub use operator_manager::OperatorManager;
