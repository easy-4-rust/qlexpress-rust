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

    pub fn pos(&self) -> i32 {
        self.pos
    }

    pub fn range(&self) -> &Range {
        &self.range
    }

    pub fn lexeme(&self) -> &str {
        &self.lexeme
    }

    pub fn code(&self) -> &str {
        &self.code
    }

    pub fn message(&self) -> &str {
        &self.message
    }

    pub fn snippet(&self) -> &str {
        &self.snippet
    }
}
