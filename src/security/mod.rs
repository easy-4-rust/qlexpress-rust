//! Security model mirroring Java `com.alibaba.qlexpress4.security`.
//!
//! Stage 0 delivers the `QLSecurityStrategy` surface needed by
//! `InitOptions`; enforcement hooks are Stage-5 work.

pub mod ql_security_strategy;

pub use ql_security_strategy::QLSecurityStrategy;
