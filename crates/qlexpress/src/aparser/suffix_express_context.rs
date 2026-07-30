//! QLParser 语法树类型；由 Java 生成式内部类型按对象边界拆分。

use super::node::Node;

/// 语法树节点 SuffixExpressContext。对应 Java: com.alibaba.qlexpress4.aparser.QLParser 内部类 SuffixExpressContext
/// Java `SuffixExpressContext`.
#[derive(Clone, Debug)]
pub struct SuffixExpressContext {
    /// 该语法规则中的 `op_id` 子节点、终结符或节点集合。
    pub op_id: Box<Node>,
}
