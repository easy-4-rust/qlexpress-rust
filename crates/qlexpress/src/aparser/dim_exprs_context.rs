//! QLParser 语法树类型；由 Java 生成式内部类型按对象边界拆分。

use super::node::Node;
use super::terminal_node::TerminalNode;

/// 语法树节点 DimExprsContext。对应 Java: com.alibaba.qlexpress4.aparser.QLParser 内部类 DimExprsContext
/// Java `DimExprsContext` (`new int[3][4]`).
#[derive(Clone, Debug)]
pub struct DimExprsContext {
    /// 该语法规则中的 `expressions` 子节点、终结符或节点集合。
    pub expressions: Vec<Node>,
    /// 每个维度表达式两侧的方括号，按 `[`, `]` 成对保存。
    pub brackets: Vec<TerminalNode>,
}
