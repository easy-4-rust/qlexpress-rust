//! QLParser 语法树类型；由 Java 生成式内部类型按对象边界拆分。

use super::terminal_node::TerminalNode;

/// 语法树节点 ContextSelectExprContext。对应 Java: com.alibaba.qlexpress4.aparser.QLParser 内部类 ContextSelectExprContext
/// Java `ContextSelectExprContext` (selector expression).
#[derive(Clone, Debug)]
pub struct ContextSelectExprContext {
    /// 该语法规则中的 `selector_start` 子节点、终结符或节点集合。
    pub selector_start: TerminalNode,
    /// 该语法规则中的 `selector_variable` 子节点、终结符或节点集合。
    pub selector_variable: TerminalNode,
}
