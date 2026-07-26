/// Zero-based position in a document, mirroring Java `lsp.Position`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
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

    /// 执行 `line` 公开操作。对应 Java 源码 `com/alibaba/qlexpress4/exception/lsp/Position.java:1` 的 `Position`；该方法为 Rust 同职责适配接口。
    pub fn line(&self) -> i32 {
        self.line
    }

    /// 执行 `character` 公开操作。对应 Java 源码 `com/alibaba/qlexpress4/exception/lsp/Position.java:1` 的 `Position`；该方法为 Rust 同职责适配接口。
    pub fn character(&self) -> i32 {
        self.character
    }
}
