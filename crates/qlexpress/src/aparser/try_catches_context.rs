//! QLParser 语法树类型；由 Java 生成式内部类型按对象边界拆分。

use super::node::Node;

/// 语法树节点 TryCatchesContext。对应 Java: com.alibaba.qlexpress4.aparser.QLParser 内部类 TryCatchesContext
/// Java `TryCatchesContext`.
#[derive(Clone, Debug)]
pub struct TryCatchesContext {
    /// 该语法规则中的 `catches` 子节点、终结符或节点集合。
    pub catches: Vec<Node>,
}
