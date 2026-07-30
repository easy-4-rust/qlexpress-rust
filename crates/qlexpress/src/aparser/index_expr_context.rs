//! QLParser 语法树类型；由 Java 生成式内部类型按对象边界拆分。

use super::node::Node;
use super::terminal_node::TerminalNode;

/// 语法树节点 IndexExprContext。对应 Java: com.alibaba.qlexpress4.aparser.QLParser 内部类 IndexExprContext
/// Java `IndexExprContext` (`a[i]` / `a[i:j]`); `None` index for `a[]`.
#[derive(Clone, Debug)]
pub struct IndexExprContext {
    /// 该语法规则中的 `lbrack` 子节点、终结符或节点集合。
    pub lbrack: TerminalNode,
    /// 该语法规则中的 `index_value_expr` 子节点、终结符或节点集合。
    pub index_value_expr: Option<Box<Node>>,
    /// 右方括号；Java `ParserRuleContext#getStop()` 在 `a[]` 上返回此 token。
    pub rbrack: TerminalNode,
}
