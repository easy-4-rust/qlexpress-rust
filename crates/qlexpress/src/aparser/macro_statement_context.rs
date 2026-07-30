//! QLParser 语法树类型；由 Java 生成式内部类型按对象边界拆分。

use super::node::Node;
use super::terminal_node::TerminalNode;

/// 语法树节点 MacroStatementContext。对应 Java: com.alibaba.qlexpress4.aparser.QLParser 内部类 MacroStatementContext
/// Java `MacroStatementContext`.
#[derive(Clone, Debug)]
pub struct MacroStatementContext {
    /// 该语法规则中的 `macro_token` 子节点、终结符或节点集合。
    pub macro_token: TerminalNode,
    /// 该语法规则中的 `var_id` 子节点、终结符或节点集合。
    pub var_id: Box<Node>,
    /// 宏体左花括号。
    pub lbrace: TerminalNode,
    /// 该语法规则中的 `block_statements` 子节点、终结符或节点集合。
    pub block_statements: Option<Box<Node>>,
    /// 宏体右花括号。
    pub rbrace: TerminalNode,
}
