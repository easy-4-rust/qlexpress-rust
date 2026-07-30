//! QLParser 语法树类型；由 Java 生成式内部类型按对象边界拆分。

use super::node::Node;
use super::terminal_node::TerminalNode;

/// 语法树节点 NewEmptyArrExprContext。对应 Java: com.alibaba.qlexpress4.aparser.QLParser 内部类 NewEmptyArrExprContext
/// Java `NewEmptyArrExprContext` (`new int[3]`).
#[derive(Clone, Debug)]
pub struct NewEmptyArrExprContext {
    /// 该语法规则中的 `new_token` 子节点、终结符或节点集合。
    pub new_token: TerminalNode,
    /// 该语法规则中的 `decl_type_no_arr` 子节点、终结符或节点集合。
    pub decl_type_no_arr: Box<Node>,
    /// 该语法规则中的 `dim_exprs` 子节点、终结符或节点集合。
    pub dim_exprs: Box<Node>,
}
