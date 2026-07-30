/// `Position` 结构体的 Rust 实现，保留对应对象的领域职责与公开契约。
/// 对应或承接 Java 源文件：`com/alibaba/qlexpress4/exception/lsp/Position.java`；具体对象路径见 `docs/对象级对照表.md`。
/// Zero-based position in a document, mirroring Java `lsp.Position`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
/// 对应 Java: com.alibaba.qlexpress4.exception.lsp.Position。
pub struct Position {
    /// Line position in a document (zero-based).
    line: i32,
    /// Character offset on a line (zero-based).
    character: i32,
}

impl Position {
    /// 构造实例。对应 Java 源码 `com/alibaba/qlexpress4/exception/lsp/Position.java:20` 的 `Position::<init>`。
    pub fn new(line: i32, character: i32) -> Self {
        Position { line, character }
    }

    /// 返回 LSP 零基行号。
    /// 对应 Java: `Position#line`。
    pub fn line(&self) -> i32 {
        self.line
    }

    /// 返回 LSP 零基字符列。
    /// 对应 Java: `Position#character`。
    pub fn character(&self) -> i32 {
        self.character
    }
}
