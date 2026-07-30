//! 语法树子节点借用类型，承接 Java `RuleContext.children` 的元素语义。

use super::syntax_tree_factory::Node;
use super::terminal_node::TerminalNode;

/// 语法节点的借用孩子：规则节点或终结符。
///
/// 对应 Java: `com.alibaba.qlexpress4.aparser.ParseTree` 子节点引用；
/// Rust 使用借用枚举避免复制 AST。
#[derive(Clone, Copy, Debug)]
pub enum ChildRef<'a> {
    /// 非终结语法树节点。
    Node(&'a Node),
    /// 终结符语法树节点。
    Term(&'a TerminalNode),
}
