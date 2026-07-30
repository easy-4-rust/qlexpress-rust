//! QLParser 语法树类型；由 Java 生成式内部类型按对象边界拆分。

use super::node::Node;

/// 语法树节点 DeclTypeContext。对应 Java: com.alibaba.qlexpress4.aparser.QLParser 内部类 DeclTypeContext
/// Java `DeclTypeContext`.
#[derive(Clone, Debug)]
pub struct DeclTypeContext {
    /// 该语法规则中的 `primitive_type` 子节点、终结符或节点集合。
    pub primitive_type: Option<Box<Node>>,
    /// 该语法规则中的 `cls_type` 子节点、终结符或节点集合。
    pub cls_type: Option<Box<Node>>,
    /// 该语法规则中的 `dims` 子节点、终结符或节点集合。
    pub dims: Option<Box<Node>>,
}
