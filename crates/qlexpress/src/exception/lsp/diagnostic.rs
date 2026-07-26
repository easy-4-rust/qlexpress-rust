use super::range::Range;

/// `Diagnostic` 结构体的 Rust 实现，保留对应对象的领域职责与公开契约。
/// 对应或承接 Java 源文件：`com/alibaba/qlexpress4/exception/lsp/Diagnostic.java`；具体对象路径见 `docs/对象级对照表.md`。
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

    /// 返回诊断起点的绝对字符偏移。
    /// 对应 Java: `Diagnostic#pos`。
    pub fn pos(&self) -> i32 {
        self.pos
    }

    /// 返回可供 LSP 客户端定位的行列范围。
    /// 对应 Java: `Diagnostic#range`。
    pub fn range(&self) -> &Range {
        &self.range
    }

    /// 返回触发诊断的词素。
    /// 对应 Java: `Diagnostic#lexeme`。
    pub fn lexeme(&self) -> &str {
        &self.lexeme
    }

    /// 返回 QLExpress 错误码。
    /// 对应 Java: `Diagnostic#code`。
    pub fn code(&self) -> &str {
        &self.code
    }

    /// 返回面向用户的诊断消息。
    /// 对应 Java: `Diagnostic#message`。
    pub fn message(&self) -> &str {
        &self.message
    }

    /// 返回带错误位置标记的脚本片段。
    /// 对应 Java: `Diagnostic#snippet`。
    pub fn snippet(&self) -> &str {
        &self.snippet
    }
}
