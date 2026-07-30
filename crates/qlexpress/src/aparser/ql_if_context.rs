//! QLParser 语法树类型；由 Java 生成式内部类型按对象边界拆分。

use super::node::Node;
use super::terminal_node::TerminalNode;

/// 语法树节点 QlIfContext。对应 Java: com.alibaba.qlexpress4.aparser.QLParser 内部类 QlIfContext
/// Java `QlIfContext`.
#[derive(Clone, Debug)]
pub struct QlIfContext {
    /// 该语法规则中的 `if_token` 子节点、终结符或节点集合。
    pub if_token: TerminalNode,
    /// 该语法规则中的 `then_keyword` 子节点、终结符或节点集合。
    pub then_keyword: Option<TerminalNode>,
    /// 该语法规则中的 `condition` 子节点、终结符或节点集合。
    pub condition: Box<Node>,
    /// 该语法规则中的 `then_body` 子节点、终结符或节点集合。
    pub then_body: Box<Node>,
    /// 该语法规则中的 `else_body` 子节点、终结符或节点集合。
    pub else_body: Option<Box<Node>>,
}
