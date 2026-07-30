//! QLParser 语法树类型；由 Java 生成式内部类型按对象边界拆分。

use super::node::Node;
use super::terminal_node::TerminalNode;

/// 语法树节点 VariableDeclaratorContext。对应 Java: com.alibaba.qlexpress4.aparser.QLParser 内部类 VariableDeclaratorContext
/// Java `VariableDeclaratorContext`.
#[derive(Clone, Debug)]
pub struct VariableDeclaratorContext {
    /// 该语法规则中的 `id` 子节点、终结符或节点集合。
    pub id: Box<Node>,
    /// 初始化赋值符；无初始化器时为 `None`。
    pub equals: Option<TerminalNode>,
    /// 该语法规则中的 `initializer` 子节点、终结符或节点集合。
    pub initializer: Option<Box<Node>>,
}
