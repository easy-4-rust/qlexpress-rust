//! QLParser 语法树类型；由 Java 生成式内部类型按对象边界拆分。

use super::node::Node;
use super::terminal_node::TerminalNode;

/// 语法树节点 LiteralContext。对应 Java: com.alibaba.qlexpress4.aparser.QLParser 内部类 LiteralContext
/// Java `LiteralContext`: exactly one of the fields is `Some`.
#[derive(Clone, Debug)]
pub struct LiteralContext {
    /// Number / single-quoted string / `null` token.
    pub token: Option<TerminalNode>,
    /// 该语法规则中的 `boolen` 子节点、终结符或节点集合。
    pub boolen: Option<Box<Node>>,
    /// 该语法规则中的 `double_quote_string` 子节点、终结符或节点集合。
    pub double_quote_string: Option<Box<Node>>,
}
