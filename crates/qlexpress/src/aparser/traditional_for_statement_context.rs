//! QLParser 语法树类型；由 Java 生成式内部类型按对象边界拆分。

use super::node::Node;
use super::terminal_node::TerminalNode;

/// 语法树节点 TraditionalForStatementContext。对应 Java: com.alibaba.qlexpress4.aparser.QLParser 内部类 TraditionalForStatementContext
/// Java `TraditionalForStatementContext`.
#[derive(Clone, Debug)]
pub struct TraditionalForStatementContext {
    /// 该语法规则中的 `for_token` 子节点、终结符或节点集合。
    pub for_token: TerminalNode,
    /// 该语法规则中的 `for_init` 子节点、终结符或节点集合。
    pub for_init: Box<Node>,
    /// 该语法规则中的 `for_condition` 子节点、终结符或节点集合。
    pub for_condition: Option<Box<Node>>,
    /// 该语法规则中的 `for_update` 子节点、终结符或节点集合。
    pub for_update: Option<Box<Node>>,
    /// 该语法规则中的 `block_statements` 子节点、终结符或节点集合。
    pub block_statements: Option<Box<Node>>,
}
