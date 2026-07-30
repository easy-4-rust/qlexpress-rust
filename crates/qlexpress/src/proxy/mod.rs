//! Java `com.alibaba.qlexpress4.proxy` 子包:仅 mod 声明 + re-export。

pub mod q_lambda_invocation_handler;
pub use q_lambda_invocation_handler as qlambda_invocation_handler;

pub use qlambda_invocation_handler::QLambdaInvocationHandler;
