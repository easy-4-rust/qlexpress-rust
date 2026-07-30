//! QLParser 语法树类型；由 Java 生成式内部类型按对象边界拆分。

use super::node::Node;

/// 语法树节点 PrimaryContext。对应 Java: com.alibaba.qlexpress4.aparser.QLParser 内部类 PrimaryContext
/// Java `PrimaryContext`.
#[derive(Clone, Debug)]
pub struct PrimaryContext {
    /// 该语法规则中的 `prefix` 子节点、终结符或节点集合。
    pub prefix: Option<Box<Node>>,
    /// 该语法规则中的 `pathable` 子节点、终结符或节点集合。
    pub pathable: Option<Box<Node>>,
    /// 该语法规则中的 `path_parts` 子节点、终结符或节点集合。
    pub path_parts: Vec<Node>,
    /// 该语法规则中的 `suffix` 子节点、终结符或节点集合。
    pub suffix: Option<Box<Node>>,
    /// 该语法规则中的 `non_pathable` 子节点、终结符或节点集合。
    pub non_pathable: Option<Box<Node>>,
}
