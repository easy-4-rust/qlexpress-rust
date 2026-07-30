//! QLParser 语法树类型；由 Java 生成式内部类型按对象边界拆分。

use super::node::Node;
use super::terminal_node::TerminalNode;

/// 语法树节点 ThenBodyContext。对应 Java: com.alibaba.qlexpress4.aparser.QLParser 内部类 ThenBodyContext
/// Java `ThenBodyContext`: exactly one of the optionals is `Some`.
#[derive(Clone, Debug)]
pub struct ThenBodyContext {
    /// 该语法规则中的 `lbrace` 子节点、终结符或节点集合。
    pub lbrace: Option<TerminalNode>,
    /// 该语法规则中的 `block_statements` 子节点、终结符或节点集合。
    pub block_statements: Option<Box<Node>>,
    /// 花括号形式 then 体的右花括号。
    pub rbrace: Option<TerminalNode>,
    /// 该语法规则中的 `non_expression_statement` 子节点、终结符或节点集合。
    pub non_expression_statement: Option<Box<Node>>,
    /// 该语法规则中的 `expression` 子节点、终结符或节点集合。
    pub expression: Option<Box<Node>>,
}
