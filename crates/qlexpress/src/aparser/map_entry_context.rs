//! QLParser 语法树类型；由 Java 生成式内部类型按对象边界拆分。

use super::node::Node;

/// 语法树节点 MapEntryContext。对应 Java: com.alibaba.qlexpress4.aparser.QLParser 内部类 MapEntryContext
/// Java `MapEntryContext`.
#[derive(Clone, Debug)]
pub struct MapEntryContext {
    /// 该语法规则中的 `map_key` 子节点、终结符或节点集合。
    pub map_key: Box<Node>,
    /// 该语法规则中的 `map_value` 子节点、终结符或节点集合。
    pub map_value: Box<Node>,
}
