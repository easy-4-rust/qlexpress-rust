//! QLParser 语法树类型；由 Java 生成式内部类型按对象边界拆分。

use super::node::Node;
use super::terminal_node::TerminalNode;

/// 语法树节点 NewInitArrExprContext。对应 Java: com.alibaba.qlexpress4.aparser.QLParser 内部类 NewInitArrExprContext
/// Java `NewInitArrExprContext` (`new int[]{1,2}`).
#[derive(Clone, Debug)]
pub struct NewInitArrExprContext {
    /// 该语法规则中的 `new_token` 子节点、终结符或节点集合。
    pub new_token: TerminalNode,
    /// 该语法规则中的 `decl_type_no_arr` 子节点、终结符或节点集合。
    pub decl_type_no_arr: Box<Node>,
    /// 该语法规则中的 `dims` 子节点、终结符或节点集合。
    pub dims: Box<Node>,
    /// 该语法规则中的 `array_initializer` 子节点、终结符或节点集合。
    pub array_initializer: Box<Node>,
}
