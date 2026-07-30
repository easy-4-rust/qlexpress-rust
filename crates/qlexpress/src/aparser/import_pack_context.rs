//! QLParser 语法树类型；由 Java 生成式内部类型按对象边界拆分。

use super::node::Node;
use super::terminal_node::TerminalNode;

/// 语法树节点 ImportPackContext。对应 Java: com.alibaba.qlexpress4.aparser.QLParser 内部类 ImportPackContext
/// Java `ImportPackContext` (`import a.b.*;` / `import a.b.*;`).
#[derive(Clone, Debug)]
pub struct ImportPackContext {
    /// 该语法规则中的 `import_token` 子节点、终结符或节点集合。
    pub import_token: TerminalNode,
    /// 该语法规则中的 `var_ids` 子节点、终结符或节点集合。
    pub var_ids: Vec<Node>,
}
