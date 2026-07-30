//! QLParser 语法树类型；由 Java 生成式内部类型按对象边界拆分。

use super::node::Node;
use super::terminal_node::TerminalNode;

/// 语法树节点 ExpressionListContext。对应 Java: com.alibaba.qlexpress4.aparser.QLParser 内部类 ExpressionListContext
/// Java `ExpressionListContext`.
#[derive(Clone, Debug)]
pub struct ExpressionListContext {
    /// 该语法规则中的 `expressions` 子节点、终结符或节点集合。
    pub expressions: Vec<Node>,
    /// 表达式之间的逗号。
    pub commas: Vec<TerminalNode>,
}
