//! QLParser 语法树类型；由 Java 生成式内部类型按对象边界拆分。

/// `ChainKind` 枚举的 Rust 实现，保留对应对象的领域职责与公开契约。
/// 对应或承接 Java 源文件：`com/alibaba/qlexpress4/aparser/SyntaxTreeFactory.java`；具体对象路径见 `docs/对象级对照表.md`。
/// How a path part is chained, mirroring the Java `Optional*`/`Spread*`
/// subclasses of `MethodInvokeContext`/`FieldAccessContext`.
///
/// Java 用无新增字段的子类区分 Visitor 分派；Rust 用闭合枚举区分同一状态，
/// QVM 编译时仍分别生成普通、可选或展开指令：
///
/// - 对应 Java: `com.alibaba.qlexpress4.aparser.QLParser.OptionalMethodInvokeContext`
/// - 对应 Java: `com.alibaba.qlexpress4.aparser.QLParser.SpreadMethodInvokeContext`
/// - 对应 Java: `com.alibaba.qlexpress4.aparser.QLParser.OptionalFieldAccessContext`
/// - 对应 Java: `com.alibaba.qlexpress4.aparser.QLParser.SpreadFieldAccessContext`
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ChainKind {
    /// `.`
    Plain,
    /// `?.` (Java `OPTIONAL_CHAINING`)
    Optional,
    /// `*.` (Java `SPREAD_CHAINING`)
    Spread,
}
