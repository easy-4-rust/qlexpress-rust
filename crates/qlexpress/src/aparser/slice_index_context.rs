//! QLParser 语法树类型；由 Java 生成式内部类型按对象边界拆分。

use super::node::Node;
use super::terminal_node::TerminalNode;

/// 语法树节点 SliceIndexContext。对应 Java: com.alibaba.qlexpress4.aparser.QLParser 内部类 SliceIndexContext
/// Java `SliceIndexContext` (`a[start:end]`).
#[derive(Clone, Debug)]
pub struct SliceIndexContext {
    /// 该语法规则中的 `start` 子节点、终结符或节点集合。
    pub start: Option<Box<Node>>,
    /// 该语法规则中的 `colon` 子节点、终结符或节点集合。
    pub colon: TerminalNode,
    /// 该语法规则中的 `end` 子节点、终结符或节点集合。
    pub end: Option<Box<Node>>,
}
