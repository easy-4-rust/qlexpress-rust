//! QLParser 语法树类型；由 Java 生成式内部类型按对象边界拆分。

use super::node::Node;
use super::terminal_node::TerminalNode;

/// 语法树节点 FunctionStatementContext。对应 Java: com.alibaba.qlexpress4.aparser.QLParser 内部类 FunctionStatementContext
/// Java `FunctionStatementContext`.
#[derive(Clone, Debug)]
pub struct FunctionStatementContext {
    /// 该语法规则中的 `function_token` 子节点、终结符或节点集合。
    pub function_token: TerminalNode,
    /// 该语法规则中的 `var_id` 子节点、终结符或节点集合。
    pub var_id: Box<Node>,
    /// 参数列表左圆括号。
    pub lparen: TerminalNode,
    /// 该语法规则中的 `params` 子节点、终结符或节点集合。
    pub params: Option<Box<Node>>,
    /// 参数列表右圆括号。
    pub rparen: TerminalNode,
    /// 函数体左花括号。
    pub lbrace: TerminalNode,
    /// 该语法规则中的 `block_statements` 子节点、终结符或节点集合。
    pub block_statements: Option<Box<Node>>,
    /// 函数体右花括号。
    pub rbrace: TerminalNode,
}
