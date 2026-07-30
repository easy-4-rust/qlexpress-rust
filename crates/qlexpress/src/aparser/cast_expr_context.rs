//! QLParser 语法树类型；由 Java 生成式内部类型按对象边界拆分。

use super::node::Node;
use super::terminal_node::TerminalNode;

/// 语法树节点 CastExprContext。对应 Java: com.alibaba.qlexpress4.aparser.QLParser 内部类 CastExprContext
/// Java `CastExprContext`.
#[derive(Clone, Debug)]
pub struct CastExprContext {
    /// 该语法规则中的 `lparen` 子节点、终结符或节点集合。
    pub lparen: TerminalNode,
    /// 该语法规则中的 `decl_type` 子节点、终结符或节点集合。
    pub decl_type: Box<Node>,
    /// 该语法规则中的 `primary` 子节点、终结符或节点集合。
    pub primary: Box<Node>,
}
