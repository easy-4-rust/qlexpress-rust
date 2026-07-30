//! QLParser 语法树类型；由 Java 生成式内部类型按对象边界拆分。

use super::node::Node;

/// 语法树节点 LeftAssoContext。对应 Java: com.alibaba.qlexpress4.aparser.QLParser 内部类 LeftAssoContext
/// Java `LeftAssoContext`: one `op right` step.
#[derive(Clone, Debug)]
pub struct LeftAssoContext {
    /// 该语法规则中的 `binaryop` 子节点、终结符或节点集合。
    pub binaryop: Box<Node>,
    /// 该语法规则中的 `right` 子节点、终结符或节点集合。
    pub right: Box<Node>,
}
