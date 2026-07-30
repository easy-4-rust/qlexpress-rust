//! QLParser 语法树类型；由 Java 生成式内部类型按对象边界拆分。

use super::node::Node;

/// 语法树节点 LocalVariableDeclarationContext。对应 Java: com.alibaba.qlexpress4.aparser.QLParser 内部类 LocalVariableDeclarationContext
/// Java `LocalVariableDeclarationContext`.
#[derive(Clone, Debug)]
pub struct LocalVariableDeclarationContext {
    /// 该语法规则中的 `decl_type` 子节点、终结符或节点集合。
    pub decl_type: Box<Node>,
    /// 该语法规则中的 `variable_declarator_list` 子节点、终结符或节点集合。
    pub variable_declarator_list: Box<Node>,
}
