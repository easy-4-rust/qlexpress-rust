//! QLParser 语法树类型；由 Java 生成式内部类型按对象边界拆分。

use super::node::Node;
use super::terminal_node::TerminalNode;

/// 语法树节点 SwitchLabelContext。对应 Java: com.alibaba.qlexpress4.aparser.QLParser 内部类 SwitchLabelContext
/// Java `SwitchLabelContext`.
#[derive(Clone, Debug)]
pub struct SwitchLabelContext {
    /// 该语法规则中的 `case_token` 子节点、终结符或节点集合。
    pub case_token: Option<TerminalNode>,
    /// 该语法规则中的 `default_token` 子节点、终结符或节点集合。
    pub default_token: Option<TerminalNode>,
    /// 该语法规则中的 `expression` 子节点、终结符或节点集合。
    pub expression: Option<Box<Node>>,
}
