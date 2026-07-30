//! QLParser 语法树类型；由 Java 生成式内部类型按对象边界拆分。

use super::node::Node;
use super::terminal_node::TerminalNode;

/// 语法树节点 MapExprContext。对应 Java: com.alibaba.qlexpress4.aparser.QLParser 内部类 MapExprContext
/// Java `MapExprContext`.
#[derive(Clone, Debug)]
pub struct MapExprContext {
    /// 该语法规则中的 `lbrace` 子节点、终结符或节点集合。
    pub lbrace: TerminalNode,
    /// 该语法规则中的 `map_entries` 子节点、终结符或节点集合。
    pub map_entries: Box<Node>,
    /// Map 字面量右花括号。
    pub rbrace: TerminalNode,
}
