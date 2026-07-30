//! QLParser 语法树类型；由 Java 生成式内部类型按对象边界拆分。

use super::node::Node;
use super::terminal_node::TerminalNode;

/// 语法树节点 ReturnStatementContext。对应 Java: com.alibaba.qlexpress4.aparser.QLParser 内部类 ReturnStatementContext
/// Java `ReturnStatementContext`.
#[derive(Clone, Debug)]
pub struct ReturnStatementContext {
    /// 该语法规则中的 `return_token` 子节点、终结符或节点集合。
    pub return_token: TerminalNode,
    /// 该语法规则中的 `expression` 子节点、终结符或节点集合。
    pub expression: Option<Box<Node>>,
}
