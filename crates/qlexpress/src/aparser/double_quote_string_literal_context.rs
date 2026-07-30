//! QLParser 语法树类型；由 Java 生成式内部类型按对象边界拆分。

use super::dy_str_part::DyStrPart;
use super::terminal_node::TerminalNode;

/// 语法树节点 DoubleQuoteStringLiteralContext。对应 Java: com.alibaba.qlexpress4.aparser.QLParser 内部类 DoubleQuoteStringLiteralContext
/// Java `DoubleQuoteStringLiteralContext`.
#[derive(Clone, Debug)]
pub struct DoubleQuoteStringLiteralContext {
    /// 该语法规则中的 `open_quote` 子节点、终结符或节点集合。
    pub open_quote: TerminalNode,
    /// 该语法规则中的 `static_characters` 子节点、终结符或节点集合。
    pub static_characters: Option<TerminalNode>,
    /// 该语法规则中的 `parts` 子节点、终结符或节点集合。
    pub parts: Vec<DyStrPart>,
    /// 该语法规则中的 `close_quote` 子节点、终结符或节点集合。
    pub close_quote: TerminalNode,
}
