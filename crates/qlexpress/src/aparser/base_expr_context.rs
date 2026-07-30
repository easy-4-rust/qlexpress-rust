//! QLParser 语法树类型；由 Java 生成式内部类型按对象边界拆分。

use super::node::Node;

/// 语法树节点 BaseExprContext。对应 Java: com.alibaba.qlexpress4.aparser.QLParser 内部类 BaseExprContext
/// Java `BaseExprContext`: a primary plus left-associative binary chain.
#[derive(Clone, Debug)]
pub struct BaseExprContext {
    /// 该语法规则中的 `primary` 子节点、终结符或节点集合。
    pub primary: Box<Node>,
    /// 该语法规则中的 `left_assos` 子节点、终结符或节点集合。
    pub left_assos: Vec<Node>,
}
