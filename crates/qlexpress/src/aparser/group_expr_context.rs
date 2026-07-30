//! QLParser 语法树类型；由 Java 生成式内部类型按对象边界拆分。

use super::node::Node;
use super::terminal_node::TerminalNode;

/// 语法树节点 GroupExprContext。对应 Java: com.alibaba.qlexpress4.aparser.QLParser 内部类 GroupExprContext
/// Java `GroupExprContext` (parenthesised expression).
#[derive(Clone, Debug)]
pub struct GroupExprContext {
    /// 该语法规则中的 `lparen` 子节点、终结符或节点集合。
    pub lparen: TerminalNode,
    /// 该语法规则中的 `expression` 子节点、终结符或节点集合。
    pub expression: Box<Node>,
}
