//! QLParser 语法树类型；由 Java 生成式内部类型按对象边界拆分。

use super::node::Node;
use super::terminal_node::TerminalNode;

/// 语法树节点 ForInitContext。对应 Java: com.alibaba.qlexpress4.aparser.QLParser 内部类 ForInitContext
/// Java `ForInitContext`. Exactly one of the optionals is `Some`; when both
/// are `None` the init was just `;`.
#[derive(Clone, Debug)]
pub struct ForInitContext {
    /// 该语法规则中的 `local_variable_declaration` 子节点、终结符或节点集合。
    pub local_variable_declaration: Option<Box<Node>>,
    /// 该语法规则中的 `expression` 子节点、终结符或节点集合。
    pub expression: Option<Box<Node>>,
    /// 该语法规则中的 `semi` 子节点、终结符或节点集合。
    pub semi: TerminalNode,
}
