//! QLParser 语法树类型；由 Java 生成式内部类型按对象边界拆分。

use super::node::Node;
use super::terminal_node::TerminalNode;

/// 语法树节点 ListExprContext。对应 Java: com.alibaba.qlexpress4.aparser.QLParser 内部类 ListExprContext
/// Java `ListExprContext`.
#[derive(Clone, Debug)]
pub struct ListExprContext {
    /// 该语法规则中的 `lbrack` 子节点、终结符或节点集合。
    pub lbrack: TerminalNode,
    /// 该语法规则中的 `list_items` 子节点、终结符或节点集合。
    pub list_items: Option<Box<Node>>,
    /// 列表字面量右方括号。
    pub rbrack: TerminalNode,
}
