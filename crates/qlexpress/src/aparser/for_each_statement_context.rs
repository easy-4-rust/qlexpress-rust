//! QLParser 语法树类型；由 Java 生成式内部类型按对象边界拆分。

use super::node::Node;
use super::terminal_node::TerminalNode;

/// 语法树节点 ForEachStatementContext。对应 Java: com.alibaba.qlexpress4.aparser.QLParser 内部类 ForEachStatementContext
/// Java `ForEachStatementContext`.
#[derive(Clone, Debug)]
pub struct ForEachStatementContext {
    /// 该语法规则中的 `for_token` 子节点、终结符或节点集合。
    pub for_token: TerminalNode,
    /// `for` 头部左圆括号。
    pub lparen: TerminalNode,
    /// Declared element type; `None` for `for (x : xs)` (inferred).
    pub decl_type: Option<Box<Node>>,
    /// 该语法规则中的 `var_id` 子节点、终结符或节点集合。
    pub var_id: Box<Node>,
    /// 变量与被遍历表达式之间的冒号。
    pub colon: TerminalNode,
    /// 该语法规则中的 `expression` 子节点、终结符或节点集合。
    pub expression: Box<Node>,
    /// `for` 头部右圆括号。
    pub rparen: TerminalNode,
    /// 循环体左花括号。
    pub lbrace: TerminalNode,
    /// 该语法规则中的 `block_statements` 子节点、终结符或节点集合。
    pub block_statements: Option<Box<Node>>,
    /// 循环体右花括号。
    pub rbrace: TerminalNode,
}
