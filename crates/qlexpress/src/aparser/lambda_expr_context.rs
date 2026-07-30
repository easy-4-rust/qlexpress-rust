//! QLParser 语法树类型；由 Java 生成式内部类型按对象边界拆分。

use super::node::Node;
use super::terminal_node::TerminalNode;

/// 语法树节点 LambdaExprContext。对应 Java: com.alibaba.qlexpress4.aparser.QLParser 内部类 LambdaExprContext
/// Java `LambdaExprContext`.
#[derive(Clone, Debug)]
pub struct LambdaExprContext {
    /// 该语法规则中的 `lambda_parameters` 子节点、终结符或节点集合。
    pub lambda_parameters: Box<Node>,
    /// 该语法规则中的 `arrow` 子节点、终结符或节点集合。
    pub arrow: TerminalNode,
    /// 该语法规则中的 `lbrace` 子节点、终结符或节点集合。
    pub lbrace: Option<TerminalNode>,
    /// 该语法规则中的 `block_statements` 子节点、终结符或节点集合。
    pub block_statements: Option<Box<Node>>,
    /// 块形式 Lambda 的右花括号。
    pub rbrace: Option<TerminalNode>,
    /// 该语法规则中的 `expression` 子节点、终结符或节点集合。
    pub expression: Option<Box<Node>>,
}
