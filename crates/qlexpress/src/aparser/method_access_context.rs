//! QLParser 语法树类型；由 Java 生成式内部类型按对象边界拆分。

use super::node::Node;
use super::terminal_node::TerminalNode;

/// 语法树节点 MethodAccessContext。对应 Java: com.alibaba.qlexpress4.aparser.QLParser 内部类 MethodAccessContext
/// Java `MethodAccessContext` (`Cls::method`).
#[derive(Clone, Debug)]
pub struct MethodAccessContext {
    /// 该语法规则中的 `dcolon` 子节点、终结符或节点集合。
    pub dcolon: TerminalNode,
    /// 该语法规则中的 `var_id` 子节点、终结符或节点集合。
    pub var_id: Box<Node>,
}
