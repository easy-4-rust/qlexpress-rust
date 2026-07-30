//! QLParser 语法树类型；由 Java 生成式内部类型按对象边界拆分。

use super::node::Node;

/// 语法树节点 ClsTypeContext。对应 Java: com.alibaba.qlexpress4.aparser.QLParser 内部类 ClsTypeContext
/// Java `ClsTypeContext` (type arguments are consumed but not kept, like
/// Java's `parseTypeArguments`).
#[derive(Clone, Debug)]
pub struct ClsTypeContext {
    /// 该语法规则中的 `var_ids` 子节点、终结符或节点集合。
    pub var_ids: Vec<Node>,
}
