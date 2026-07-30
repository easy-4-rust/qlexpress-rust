//! 切片指令边界模式。

/// 切片左右边界的提供方式。
///
/// 对应 Java:
/// `com.alibaba.qlexpress4.runtime.instruction.SliceInstruction.Mode`。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SliceMode {
    /// 省略左边界，仅提供右边界。
    Left,
    /// 仅提供左边界，省略右边界。
    Right,
    /// 同时提供左右边界。
    Both,
    /// 省略左右边界并复制完整序列。
    Copy,
}
