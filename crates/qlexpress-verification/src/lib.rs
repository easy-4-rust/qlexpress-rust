//! QlExpress 整体生产验收与 Java/Rust 差分测试工具库。

/// 逐 case Java/Rust 差分语料的 Rust 执行器。
pub mod differential;
/// 差分结果的稳定、窄范围规范化规则。
pub mod normalization;
