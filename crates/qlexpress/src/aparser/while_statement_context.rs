//! QLParser 语法树类型；由 Java 生成式内部类型按对象边界拆分。

use super::node::Node;
use super::terminal_node::TerminalNode;

/// 语法树节点 WhileStatementContext。对应 Java: com.alibaba.qlexpress4.aparser.QLParser 内部类 WhileStatementContext
/// Java `WhileStatementContext`.
#[derive(Clone, Debug)]
pub struct WhileStatementContext {
    /// 该语法规则中的 `while_token` 子节点、终结符或节点集合。
    pub while_token: TerminalNode,
    /// 该语法规则中的 `expression` 子节点、终结符或节点集合。
    pub expression: Box<Node>,
    /// `None` for an empty `{}` body (Java returns null from
    /// `parseBracedBlock`).
    pub block_statements: Option<Box<Node>>,
}
