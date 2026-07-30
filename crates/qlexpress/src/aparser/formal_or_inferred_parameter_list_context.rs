//! QLParser 语法树类型；由 Java 生成式内部类型按对象边界拆分。

use super::node::Node;
use super::terminal_node::TerminalNode;

/// 语法树节点 FormalOrInferredParameterListContext。对应 Java: com.alibaba.qlexpress4.aparser.QLParser 内部类 FormalOrInferredParameterListContext
/// Java `FormalOrInferredParameterListContext`.
#[derive(Clone, Debug)]
pub struct FormalOrInferredParameterListContext {
    /// 该语法规则中的 `params` 子节点、终结符或节点集合。
    pub params: Vec<Node>,
    /// 参数之间的逗号。
    pub commas: Vec<TerminalNode>,
}
