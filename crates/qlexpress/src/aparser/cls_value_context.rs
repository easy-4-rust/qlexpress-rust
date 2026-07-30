//! QLParser 语法树类型；由 Java 生成式内部类型按对象边界拆分。

use super::terminal_node::TerminalNode;

/// 语法树节点 ClsValueContext。对应 Java: com.alibaba.qlexpress4.aparser.QLParser 内部类 ClsValueContext
/// Java `ClsValueContext` (map value of the special `'@class'` key).
#[derive(Clone, Debug)]
pub struct ClsValueContext {
    /// 该语法规则中的 `quote` 子节点、终结符或节点集合。
    pub quote: TerminalNode,
}
