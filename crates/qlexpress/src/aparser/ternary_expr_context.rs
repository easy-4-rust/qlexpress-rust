//! QLParser 语法树类型；由 Java 生成式内部类型按对象边界拆分。

use super::node::Node;
use super::terminal_node::TerminalNode;

/// 语法树节点 TernaryExprContext。对应 Java: com.alibaba.qlexpress4.aparser.QLParser 内部类 TernaryExprContext
/// Java `TernaryExprContext`.
#[derive(Clone, Debug)]
pub struct TernaryExprContext {
    /// 该语法规则中的 `condition` 子节点、终结符或节点集合。
    pub condition: Box<Node>,
    /// 该语法规则中的 `question` 子节点、终结符或节点集合。
    pub question: Option<TerminalNode>,
    /// 该语法规则中的 `then_expr` 子节点、终结符或节点集合。
    pub then_expr: Option<Box<Node>>,
    /// 三元表达式冒号；无三元分支时为 `None`。
    pub colon: Option<TerminalNode>,
    /// 该语法规则中的 `else_expr` 子节点、终结符或节点集合。
    pub else_expr: Option<Box<Node>>,
}
