//! QLParser 语法树类型；由 Java 生成式内部类型按对象边界拆分。

use super::node::Node;

/// 语法树节点 StringKeyContext。对应 Java: com.alibaba.qlexpress4.aparser.QLParser 内部类 StringKeyContext
/// Java `StringKeyContext` (double-quoted map key).
#[derive(Clone, Debug)]
pub struct StringKeyContext {
    /// 该语法规则中的 `double_quote_string` 子节点、终结符或节点集合。
    pub double_quote_string: Box<Node>,
}
