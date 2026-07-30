//! QLParser 语法树类型；由 Java 生成式内部类型按对象边界拆分。

use super::node::Node;
use super::terminal_node::TerminalNode;

/// 语法树节点 TryCatchContext。对应 Java: com.alibaba.qlexpress4.aparser.QLParser 内部类 TryCatchContext
/// Java `TryCatchContext` (one catch clause).
#[derive(Clone, Debug)]
pub struct TryCatchContext {
    /// 该语法规则中的 `catch_token` 子节点、终结符或节点集合。
    pub catch_token: TerminalNode,
    /// catch 参数左圆括号。
    pub lparen: TerminalNode,
    /// 该语法规则中的 `catch_params` 子节点、终结符或节点集合。
    pub catch_params: Box<Node>,
    /// catch 参数右圆括号。
    pub rparen: TerminalNode,
    /// catch 主体左花括号。
    pub lbrace: TerminalNode,
    /// 该语法规则中的 `block_statements` 子节点、终结符或节点集合。
    pub block_statements: Option<Box<Node>>,
    /// catch 主体右花括号。
    pub rbrace: TerminalNode,
}
