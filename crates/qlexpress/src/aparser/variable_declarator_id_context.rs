//! QLParser 语法树类型；由 Java 生成式内部类型按对象边界拆分。

use super::node::Node;

/// 语法树节点 VariableDeclaratorIdContext。对应 Java: com.alibaba.qlexpress4.aparser.QLParser 内部类 VariableDeclaratorIdContext
/// Java `VariableDeclaratorIdContext`.
#[derive(Clone, Debug)]
pub struct VariableDeclaratorIdContext {
    /// 该语法规则中的 `var_id` 子节点、终结符或节点集合。
    pub var_id: Box<Node>,
    /// 该语法规则中的 `dims` 子节点、终结符或节点集合。
    pub dims: Option<Box<Node>>,
}
