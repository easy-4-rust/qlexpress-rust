//! QLParser 语法树类型；由 Java 生成式内部类型按对象边界拆分。

use super::node::Node;
use super::terminal_node::TerminalNode;

/// 语法树节点 ElseBodyContext。对应 Java: com.alibaba.qlexpress4.aparser.QLParser 内部类 ElseBodyContext
/// Java `ElseBodyContext`: exactly one of the optionals is `Some`.
#[derive(Clone, Debug)]
pub struct ElseBodyContext {
    /// 该语法规则中的 `lbrace` 子节点、终结符或节点集合。
    pub lbrace: Option<TerminalNode>,
    /// 该语法规则中的 `block_statements` 子节点、终结符或节点集合。
    pub block_statements: Option<Box<Node>>,
    /// 花括号形式 else 体的右花括号。
    pub rbrace: Option<TerminalNode>,
    /// 该语法规则中的 `ql_if` 子节点、终结符或节点集合。
    pub ql_if: Option<Box<Node>>,
    /// 该语法规则中的 `non_expression_statement` 子节点、终结符或节点集合。
    pub non_expression_statement: Option<Box<Node>>,
    /// 该语法规则中的 `expression` 子节点、终结符或节点集合。
    pub expression: Option<Box<Node>>,
}
