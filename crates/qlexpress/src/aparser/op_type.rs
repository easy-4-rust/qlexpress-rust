//! 解析阶段操作符位置类型。

/// 操作符在表达式中的位置类别。
///
/// 对应 Java:
/// `com.alibaba.qlexpress4.aparser.ParserOperatorManager.OpType`。
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum OpType {
    /// 前缀一元操作符，例如 `!x`。
    Prefix,
    /// 后缀一元操作符，例如 `x++`。
    Suffix,
    /// 中缀二元操作符，例如 `a + b`。
    Middle,
}
