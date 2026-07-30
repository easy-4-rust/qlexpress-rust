//! Lambda 数据对象。对应 Java 包
//! `com.alibaba.qlexpress4.runtime.data.lambda`。

pub mod q_lambda_method;
pub use q_lambda_method as qlambda_method;

pub use qlambda_method::QLambdaMethod;
