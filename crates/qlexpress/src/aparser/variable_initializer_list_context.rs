//! QLParser 语法树类型；由 Java 生成式内部类型按对象边界拆分。

use super::node::Node;

/// 语法树节点 VariableInitializerListContext。对应 Java: com.alibaba.qlexpress4.aparser.QLParser 内部类 VariableInitializerListContext
/// Java `VariableInitializerListContext`.
#[derive(Clone, Debug)]
pub struct VariableInitializerListContext {
    /// 该语法规则中的 `initializers` 子节点、终结符或节点集合。
    pub initializers: Vec<Node>,
}
