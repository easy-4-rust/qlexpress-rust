//! 空 Lambda,对应 Java `com.alibaba.qlexpress4.runtime.QLambdaEmpty`。
//! 职责:空调用体,`call` 直接返回 `QResult.NEXT_INSTRUCTION`
//! (调用语义在 [`crate::runtime::qlambda::QLambda`] 的 `Empty` 变体分支中实现,与原类一致)。
//! 本文件由 `qlambda.rs` 拆分而来(SPEC §5.5 一类一文件),仅移动代码与补充中文注释,行为完全一致。

/// 空 Lambda。对应 Java: com.alibaba.qlexpress4.runtime.QLambdaEmpty
/// (单例 `INSTANCE`;Rust 以 unit 结构体承载,作为 `QLambda::Empty` 的负载)
pub struct QLambdaEmpty;

impl QLambdaEmpty {
    /// 单例。对应 Java `QLambdaEmpty.INSTANCE`。
    /// Java `QLambdaEmpty.INSTANCE`.
    pub const INSTANCE: QLambdaEmpty = QLambdaEmpty;
}
