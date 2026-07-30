//! QLParser 语法树类型；由 Java 生成式内部类型按对象边界拆分。

use super::node::Node;
use super::terminal_node::TerminalNode;

/// 语法树节点 VarIdExprContext。对应 Java: com.alibaba.qlexpress4.aparser.QLParser 内部类 VarIdExprContext
/// Java `VarIdExprContext` (variable reference or function call head).
#[derive(Clone, Debug)]
pub struct VarIdExprContext {
    /// 该语法规则中的 `var_id` 子节点、终结符或节点集合。
    pub var_id: Box<Node>,
    /// 该语法规则中的 `lparen` 子节点、终结符或节点集合。
    pub lparen: Option<TerminalNode>,
    /// 该语法规则中的 `argument_list` 子节点、终结符或节点集合。
    pub argument_list: Option<Box<Node>>,
}
