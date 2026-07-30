//! QLParser 语法树类型；由 Java 生成式内部类型按对象边界拆分。

use super::node::Node;
use super::terminal_node::TerminalNode;

/// 语法树节点 TryFinallyContext。对应 Java: com.alibaba.qlexpress4.aparser.QLParser 内部类 TryFinallyContext
/// Java `TryFinallyContext`.
#[derive(Clone, Debug)]
pub struct TryFinallyContext {
    /// 该语法规则中的 `finally_token` 子节点、终结符或节点集合。
    pub finally_token: TerminalNode,
    /// 该语法规则中的 `block_statements` 子节点、终结符或节点集合。
    pub block_statements: Option<Box<Node>>,
}
