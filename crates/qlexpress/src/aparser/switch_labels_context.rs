//! QLParser 语法树类型；由 Java 生成式内部类型按对象边界拆分。

use super::node::Node;

/// 语法树节点 SwitchLabelsContext。对应 Java: com.alibaba.qlexpress4.aparser.QLParser 内部类 SwitchLabelsContext
/// Java `SwitchLabelsContext`.
#[derive(Clone, Debug)]
pub struct SwitchLabelsContext {
    /// 该语法规则中的 `labels` 子节点、终结符或节点集合。
    pub labels: Vec<Node>,
}
