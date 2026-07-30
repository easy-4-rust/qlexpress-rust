//! QLParser 语法树类型；由 Java 生成式内部类型按对象边界拆分。

use super::node::Node;
use super::terminal_node::TerminalNode;

/// 语法树节点 StringExpressionContext。对应 Java: com.alibaba.qlexpress4.aparser.QLParser 内部类 StringExpressionContext
/// Java `StringExpressionContext` (`${expr}` or `${#var}` inside a string).
#[derive(Clone, Debug)]
pub struct StringExpressionContext {
    /// 该语法规则中的 `start` 子节点、终结符或节点集合。
    pub start: TerminalNode,
    /// 该语法规则中的 `selector_variable` 子节点、终结符或节点集合。
    pub selector_variable: Option<TerminalNode>,
    /// 该语法规则中的 `expression` 子节点、终结符或节点集合。
    pub expression: Option<Box<Node>>,
}
