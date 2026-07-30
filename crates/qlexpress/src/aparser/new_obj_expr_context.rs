//! QLParser 语法树类型；由 Java 生成式内部类型按对象边界拆分。

use super::node::Node;
use super::terminal_node::TerminalNode;

/// 语法树节点 NewObjExprContext。对应 Java: com.alibaba.qlexpress4.aparser.QLParser 内部类 NewObjExprContext
/// Java `NewObjExprContext`.
#[derive(Clone, Debug)]
pub struct NewObjExprContext {
    /// 该语法规则中的 `new_token` 子节点、终结符或节点集合。
    pub new_token: TerminalNode,
    /// 该语法规则中的 `var_ids` 子节点、终结符或节点集合。
    pub var_ids: Vec<Node>,
    /// 该语法规则中的 `argument_list` 子节点、终结符或节点集合。
    pub argument_list: Option<Box<Node>>,
}
