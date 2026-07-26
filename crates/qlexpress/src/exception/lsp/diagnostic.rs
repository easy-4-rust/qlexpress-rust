use super::range::Range;

/// Diagnostic information attached to a [`crate::exception::QLException`],
/// mirroring Java `lsp.Diagnostic`.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Diagnostic {
    /// Start position (absolute char offset) in the script.
    pos: i32,
    /// The range at which the message applies.
    range: Range,
    /// Lexeme in range.
    lexeme: String,
    /// The diagnostic's code (a `QLErrorCodes` constant).
    code: String,
    /// The diagnostic's message (reason).
    message: String,
    /// Snippet near the error position.
    snippet: String,
}

impl Diagnostic {
    /// 构造实例。对应 Java 源码 `com/alibaba/qlexpress4/exception/lsp/Diagnostic.java:38` 的 `Diagnostic::<init>`。
    pub fn new(
        pos: i32,
        range: Range,
        lexeme: impl Into<String>,
        code: impl Into<String>,
        message: impl Into<String>,
        snippet: impl Into<String>,
    ) -> Self {
        Diagnostic {
            pos,
            range,
            lexeme: lexeme.into(),
            code: code.into(),
            message: message.into(),
            snippet: snippet.into(),
        }
    }

    /// 执行 `pos` 公开操作。对应 Java 源码 `com/alibaba/qlexpress4/exception/lsp/Diagnostic.java:1` 的 `Diagnostic`；该方法为 Rust 同职责适配接口。
    pub fn pos(&self) -> i32 {
        self.pos
    }

    /// 执行 `range` 公开操作。对应 Java 源码 `com/alibaba/qlexpress4/exception/lsp/Diagnostic.java:1` 的 `Diagnostic`；该方法为 Rust 同职责适配接口。
    pub fn range(&self) -> &Range {
        &self.range
    }

    /// 执行 `lexeme` 公开操作。对应 Java 源码 `com/alibaba/qlexpress4/exception/lsp/Diagnostic.java:1` 的 `Diagnostic`；该方法为 Rust 同职责适配接口。
    pub fn lexeme(&self) -> &str {
        &self.lexeme
    }

    /// 执行 `code` 公开操作。对应 Java 源码 `com/alibaba/qlexpress4/exception/lsp/Diagnostic.java:1` 的 `Diagnostic`；该方法为 Rust 同职责适配接口。
    pub fn code(&self) -> &str {
        &self.code
    }

    /// 执行 `message` 公开操作。对应 Java 源码 `com/alibaba/qlexpress4/exception/lsp/Diagnostic.java:1` 的 `Diagnostic`；该方法为 Rust 同职责适配接口。
    pub fn message(&self) -> &str {
        &self.message
    }

    /// 执行 `snippet` 公开操作。对应 Java 源码 `com/alibaba/qlexpress4/exception/lsp/Diagnostic.java:1` 的 `Diagnostic`；该方法为 Rust 同职责适配接口。
    pub fn snippet(&self) -> &str {
        &self.snippet
    }
}
