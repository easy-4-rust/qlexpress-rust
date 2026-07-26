//! Annotation metadata mirroring Java `com.alibaba.qlexpress4.annotation`.
//!
//! Java 注解(@QLAlias / @QLFunction)在 Rust 无对应机制,按 SPEC 平移为
//! 「元数据结构体 + 注册参数」(见各文件头注释)。

pub mod ql_alias;
pub mod ql_function;

pub use ql_alias::QLAlias;
pub use ql_function::QLFunction;
