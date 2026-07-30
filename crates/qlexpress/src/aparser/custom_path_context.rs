//! QLParser 语法树类型；由 Java 生成式内部类型按对象边界拆分。

use super::node::Node;
use super::terminal_node::TerminalNode;

/// 语法树节点 CustomPathContext。对应 Java: com.alibaba.qlexpress4.aparser.QLParser 内部类 CustomPathContext
/// Java `CustomPathContext` (custom operator path, e.g. `a %% 'path'`).
#[derive(Clone, Debug)]
pub struct CustomPathContext {
    /// 该语法规则中的 `op_id` 子节点、终结符或节点集合。
    pub op_id: Box<Node>,
    /// 该语法规则中的 `var_id` 子节点、终结符或节点集合。
    pub var_id: Option<Box<Node>>,
    /// 该语法规则中的 `quote` 子节点、终结符或节点集合。
    pub quote: Option<TerminalNode>,
    /// 该语法规则中的 `path_text` 子节点、终结符或节点集合。
    pub path_text: String,
}
