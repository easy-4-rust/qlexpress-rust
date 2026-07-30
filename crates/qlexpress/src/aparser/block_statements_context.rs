//! QLParser 语法树类型；由 Java 生成式内部类型按对象边界拆分。

use super::node::Node;

/// 语法树节点 BlockStatementsContext。对应 Java: com.alibaba.qlexpress4.aparser.QLParser 内部类 BlockStatementsContext
/// Java `BlockStatementsContext`.
#[derive(Clone, Debug)]
pub struct BlockStatementsContext {
    /// `BlockStatement` nodes in source order.
    pub statements: Vec<Node>,
}
