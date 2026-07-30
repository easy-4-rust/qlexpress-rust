//! QLParser 语法树类型；由 Java 生成式内部类型按对象边界拆分。

use super::node::Node;
use super::terminal_node::TerminalNode;

/// 语法树节点 ArrayInitializerContext。对应 Java: com.alibaba.qlexpress4.aparser.QLParser 内部类 ArrayInitializerContext
/// Java `ArrayInitializerContext`.
#[derive(Clone, Debug)]
pub struct ArrayInitializerContext {
    /// 该语法规则中的 `lbrace` 子节点、终结符或节点集合。
    pub lbrace: TerminalNode,
    /// 该语法规则中的 `initializers` 子节点、终结符或节点集合。
    pub initializers: Option<Box<Node>>,
    /// 数组初始化器右花括号。
    pub rbrace: TerminalNode,
}
