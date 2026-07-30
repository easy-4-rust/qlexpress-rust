//! QLParser 语法树类型；由 Java 生成式内部类型按对象边界拆分。

use super::node::Node;
use super::terminal_node::TerminalNode;

/// `DyStrPart` 枚举的 Rust 实现，保留对应对象的领域职责与公开契约。
/// 对应或承接 Java 源文件：`com/alibaba/qlexpress4/aparser/SyntaxTreeFactory.java`；具体对象路径见 `docs/对象级对照表.md`。
/// One piece of a double-quoted string: literal text or an interpolation.
#[derive(Clone, Debug)]
/// 对应 Java: com.alibaba.qlexpress4.aparser.SyntaxTreeFactory。
pub enum DyStrPart {
    /// Java `DyStrText` token.
    Text(TerminalNode),
    /// Java `StringExpressionContext`.
    Expr(Box<Node>),
}
