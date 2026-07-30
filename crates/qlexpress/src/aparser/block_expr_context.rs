//! QLParser 语法树类型；由 Java 生成式内部类型按对象边界拆分。

use super::node::Node;
use super::terminal_node::TerminalNode;

/// 语法树节点 BlockExprContext。对应 Java: com.alibaba.qlexpress4.aparser.QLParser 内部类 BlockExprContext
/// Java `BlockExprContext` (a block used as an expression).
#[derive(Clone, Debug)]
pub struct BlockExprContext {
    /// 该语法规则中的 `lbrace` 子节点、终结符或节点集合。
    pub lbrace: TerminalNode,
    /// 该语法规则中的 `block_statements` 子节点、终结符或节点集合。
    pub block_statements: Option<Box<Node>>,
}
