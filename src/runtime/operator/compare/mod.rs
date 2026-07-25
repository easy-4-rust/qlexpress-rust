//! 比较操作符包,对应 Java `com.alibaba.qlexpress4.runtime.operator.compare`。

pub mod equal_operator;
pub mod greater_equal_operator;
pub mod greater_operator;
pub mod less_equal_operator;
pub mod less_operator;
pub mod unequal_operator;

pub use equal_operator::EqualOperator;
pub use greater_equal_operator::GreaterEqualOperator;
pub use greater_operator::GreaterOperator;
pub use less_equal_operator::LessEqualOperator;
pub use less_operator::LessOperator;
pub use unequal_operator::UnequalOperator;
