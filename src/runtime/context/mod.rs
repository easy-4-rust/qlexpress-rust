//! 外部变量上下文,对应 Java `com.alibaba.qlexpress4.runtime.context` 包。
//!
//! 一类一文件(SPEC §5.5):本文件仅做 mod 声明与 re-export,不含实现。

pub mod dynamic_variable_context;
pub mod empty_context;
pub mod express_context;
pub mod map_express_context;
pub mod object_field_express_context;
pub mod ql_alias_context;

pub use dynamic_variable_context::{DynamicScriptRunner, DynamicVariableContext};
pub use empty_context::EmptyContext;
pub use express_context::ExpressContext;
pub use map_express_context::MapExpressContext;
pub use object_field_express_context::ObjectFieldExpressContext;
pub use ql_alias_context::QLAliasContext;
