//! QLParser 语法树类型；由 Java 生成式内部类型按对象边界拆分。

use super::node::Node;
use super::terminal_node::TerminalNode;

/// 语法树节点 MapEntriesContext。对应 Java: com.alibaba.qlexpress4.aparser.QLParser 内部类 MapEntriesContext
/// Java `MapEntriesContext`. `empty_colon` is `Some` for the empty-map
/// literal `{:}`.
#[derive(Clone, Debug)]
pub struct MapEntriesContext {
    /// 该语法规则中的 `empty_colon` 子节点、终结符或节点集合。
    pub empty_colon: Option<TerminalNode>,
    /// 该语法规则中的 `entries` 子节点、终结符或节点集合。
    pub entries: Vec<Node>,
}
