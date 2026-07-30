//! QLParser 语法树类型；由 Java 生成式内部类型按对象边界拆分。

use super::node::Node;

/// 语法树节点 ProgramContext。对应 Java: com.alibaba.qlexpress4.aparser.QLParser 内部类 ProgramContext
/// Java `ProgramContext`.
#[derive(Clone, Debug)]
pub struct ProgramContext {
    /// Import declarations (`ImportCls`/`ImportPack` nodes).
    pub imports: Vec<Node>,
    /// Top-level statements; `None` for an import-only or empty script.
    pub block_statements: Option<Box<Node>>,
}
