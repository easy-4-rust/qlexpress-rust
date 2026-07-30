//! QLParser 语法树类型；由 Java 生成式内部类型按对象边界拆分。

use super::node::Node;
use super::terminal_node::TerminalNode;

/// 语法树节点 TryCatchExprContext。对应 Java: com.alibaba.qlexpress4.aparser.QLParser 内部类 TryCatchExprContext
/// Java `TryCatchExprContext`.
#[derive(Clone, Debug)]
pub struct TryCatchExprContext {
    /// 该语法规则中的 `try_token` 子节点、终结符或节点集合。
    pub try_token: TerminalNode,
    /// 该语法规则中的 `block_statements` 子节点、终结符或节点集合。
    pub block_statements: Option<Box<Node>>,
    /// 该语法规则中的 `try_catches` 子节点、终结符或节点集合。
    pub try_catches: Option<Box<Node>>,
    /// 该语法规则中的 `try_finally` 子节点、终结符或节点集合。
    pub try_finally: Option<Box<Node>>,
}
