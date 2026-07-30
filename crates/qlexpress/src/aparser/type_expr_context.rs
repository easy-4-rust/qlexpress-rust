//! QLParser 语法树类型；由 Java 生成式内部类型按对象边界拆分。

use super::node::Node;

/// 语法树节点 TypeExprContext。对应 Java: com.alibaba.qlexpress4.aparser.QLParser 内部类 TypeExprContext
/// Java `TypeExprContext`（作为值使用的类型，含原语、具名类和数组类型）。
#[derive(Clone, Debug)]
pub struct TypeExprContext {
    /// 该语法规则中的 `decl_type` 子节点、终结符或节点集合。
    pub decl_type: Box<Node>,
}
