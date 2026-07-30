//! QLParser 语法树类型；由 Java 生成式内部类型按对象边界拆分。

use super::node::Node;
use super::terminal_node::TerminalNode;

/// 语法树节点 VariableDeclaratorListContext。对应 Java: com.alibaba.qlexpress4.aparser.QLParser 内部类 VariableDeclaratorListContext
/// Java `VariableDeclaratorListContext`.
#[derive(Clone, Debug)]
pub struct VariableDeclaratorListContext {
    /// 该语法规则中的 `variables` 子节点、终结符或节点集合。
    pub variables: Vec<Node>,
    /// 变量声明之间的逗号。
    pub commas: Vec<TerminalNode>,
}
