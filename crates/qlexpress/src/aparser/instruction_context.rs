//! QVM 指令 Visitor 当前编译上下文。

/// 区分普通语句块与宏展开语句块。
///
/// 对应 Java:
/// `com.alibaba.qlexpress4.aparser.QvmInstructionVisitor.Context`。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Context {
    /// 普通语句块。
    Block,
    /// 宏展开语句块。
    Macro,
}
