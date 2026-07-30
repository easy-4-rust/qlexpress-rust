//! QLParser 语法树类型；由 Java 生成式内部类型按对象边界拆分。

use super::node::Node;

/// 语法树节点 VariableInitializerContext。对应 Java: com.alibaba.qlexpress4.aparser.QLParser 内部类 VariableInitializerContext
/// Java `VariableInitializerContext`: exactly one variant is `Some`.
#[derive(Clone, Debug)]
pub struct VariableInitializerContext {
    /// 该语法规则中的 `expression` 子节点、终结符或节点集合。
    pub expression: Option<Box<Node>>,
    /// 该语法规则中的 `array_initializer` 子节点、终结符或节点集合。
    pub array_initializer: Option<Box<Node>>,
}
