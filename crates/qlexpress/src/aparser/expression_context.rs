//! QLParser 语法树类型；由 Java 生成式内部类型按对象边界拆分。

use super::node::Node;

/// 语法树节点 ExpressionContext。对应 Java: com.alibaba.qlexpress4.aparser.QLParser 内部类 ExpressionContext
/// Java `ExpressionContext`: assignment or ternary.
#[derive(Clone, Debug)]
pub struct ExpressionContext {
    /// 该语法规则中的 `left` 子节点、终结符或节点集合。
    pub left: Option<Box<Node>>,
    /// 该语法规则中的 `assign_operator` 子节点、终结符或节点集合。
    pub assign_operator: Option<Box<Node>>,
    /// 该语法规则中的 `expression` 子节点、终结符或节点集合。
    pub expression: Option<Box<Node>>,
    /// 该语法规则中的 `ternary` 子节点、终结符或节点集合。
    pub ternary: Option<Box<Node>>,
}
