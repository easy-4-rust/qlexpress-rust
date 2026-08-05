use super::range::Range;

/// 提供给编辑器或 LSP 客户端的语法诊断。
/// 对应或承接 Java 源文件：`com/alibaba/qlexpress4/exception/lsp/Diagnostic.java`；具体对象路径见 `docs/对象级对照表.md`。
/// Diagnostic information attached to a [`crate::exception::QLException`],
/// mirroring Java `lsp.Diagnostic`.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
/// 对应 Java: com.alibaba.qlexpress4.exception.lsp.Diagnostic。
pub struct Diagnostic {
    /// Start position (absolute char offset) in the script.
    pos: i32,
    /// The range at which the message applies.
    range: Option<Range>,
    /// Lexeme in range.
    lexeme: Option<String>,
    /// The diagnostic's code (a `QLErrorCodes` constant).
    code: Option<String>,
    /// The diagnostic's message (reason).
    message: Option<String>,
    /// Snippet near the error position.
    snippet: Option<String>,
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
            range: Some(range),
            lexeme: Some(lexeme.into()),
            code: Some(code.into()),
            message: Some(message.into()),
            snippet: Some(snippet.into()),
        }
    }

    /// 以 Java 可空引用字段构造诊断。
    ///
    /// 对应 Java: `Diagnostic(int, Range, String, String, String, String)`；
    /// Java 构造器不拒绝 `null`，本入口用 [`Option`] 原样表达这些状态。
    #[allow(clippy::too_many_arguments)]
    pub fn from_options(
        pos: i32,
        range: Option<Range>,
        lexeme: Option<String>,
        code: Option<String>,
        message: Option<String>,
        snippet: Option<String>,
    ) -> Self {
        Diagnostic {
            pos,
            range,
            lexeme,
            code,
            message,
            snippet,
        }
    }

    /// 返回诊断起点的绝对字符偏移。
    /// 对应 Java: `Diagnostic#pos`。
    pub fn pos(&self) -> i32 {
        self.pos
    }

    /// 返回可供 LSP 客户端定位的行列范围。
    /// 对应 Java: `Diagnostic#range`。
    pub fn range(&self) -> Option<&Range> {
        self.range.as_ref()
    }

    /// 返回触发诊断的词素。
    /// 对应 Java: `Diagnostic#lexeme`。
    pub fn lexeme(&self) -> Option<&str> {
        self.lexeme.as_deref()
    }

    /// 返回 QLExpress 错误码。
    /// 对应 Java: `Diagnostic#code`。
    pub fn code(&self) -> Option<&str> {
        self.code.as_deref()
    }

    /// 返回面向用户的诊断消息。
    /// 对应 Java: `Diagnostic#message`。
    pub fn message(&self) -> Option<&str> {
        self.message.as_deref()
    }

    /// 返回带错误位置标记的脚本片段。
    /// 对应 Java: `Diagnostic#snippet`。
    pub fn snippet(&self) -> Option<&str> {
        self.snippet.as_deref()
    }
}

#[cfg(test)]
mod tests {
    use super::Diagnostic;
    use crate::lsp::{Position, Range};

    #[test]
    fn preserves_non_null_and_null_java_reference_fields() {
        let diagnostic = Diagnostic::new(
            12,
            Range::new(Position::new(1, 2), Position::new(1, 5)),
            "abc",
            "E001",
            "bad input",
            "a = abc",
        );
        assert_eq!(diagnostic.pos(), 12);
        assert_eq!(diagnostic.lexeme(), Some("abc"));
        assert_eq!(diagnostic.code(), Some("E001"));
        assert_eq!(diagnostic.message(), Some("bad input"));
        assert_eq!(diagnostic.snippet(), Some("a = abc"));
        assert!(diagnostic.range().is_some());

        let nullable = Diagnostic::from_options(0, None, None, None, None, None);
        assert!(nullable.range().is_none());
        assert!(nullable.lexeme().is_none());
        assert!(nullable.code().is_none());
        assert!(nullable.message().is_none());
        assert!(nullable.snippet().is_none());
    }
}
