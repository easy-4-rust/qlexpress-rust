//! QLParser 语法树类型；由 Java 生成式内部类型按对象边界拆分。

use super::node::Node;
use super::terminal_node::TerminalNode;

/// 语法树节点 TraditionalForStatementContext。对应 Java: com.alibaba.qlexpress4.aparser.QLParser 内部类 TraditionalForStatementContext
/// Java `TraditionalForStatementContext`.
#[derive(Clone, Debug)]
pub struct TraditionalForStatementContext {
    /// 该语法规则中的 `for_token` 子节点、终结符或节点集合。
    pub for_token: TerminalNode,
    /// `for` 头部左圆括号。
    pub lparen: TerminalNode,
    /// 该语法规则中的 `for_init` 子节点、终结符或节点集合。
    pub for_init: Box<Node>,
    /// 该语法规则中的 `for_condition` 子节点、终结符或节点集合。
    pub for_condition: Option<Box<Node>>,
    /// 条件后的第二个分号；第一个分号属于 `for_init` 子节点。
    pub condition_semi: TerminalNode,
    /// 该语法规则中的 `for_update` 子节点、终结符或节点集合。
    pub for_update: Option<Box<Node>>,
    /// `for` 头部右圆括号。
    pub rparen: TerminalNode,
    /// 循环体左花括号。
    pub lbrace: TerminalNode,
    /// 该语法规则中的 `block_statements` 子节点、终结符或节点集合。
    pub block_statements: Option<Box<Node>>,
    /// 循环体右花括号。
    pub rbrace: TerminalNode,
}
