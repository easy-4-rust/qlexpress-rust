use std::fmt;

use super::error_codes;
use super::ex_message_util::ExMessageUtil;
use super::lsp::{Diagnostic, Position, Range};
use super::ql_syntax_exception::QLSyntaxException;
use crate::runtime::value::DataValue;

/// Which Java exception subclass this error corresponds to.
///
/// Java models the error hierarchy as classes (`QLSyntaxException`,
/// `QLRuntimeException`, `QLTimeoutException`); Rust models it as a single
/// error type plus this discriminant so the engine can use
/// `Result<T, QLException>` everywhere (SPEC §3.4).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum QLExceptionKind {
    /// `QLSyntaxException` — parse/scan time error.
    Syntax,
    /// `QLRuntimeException` — runtime error, may carry a catchable object.
    Runtime,
    /// `QLTimeoutException` — script exceeded its timeout.
    Timeout,
}

/// The engine's single error type, mirroring Java `QLException`
/// (and subclasses, via [`QLExceptionKind`]).
#[derive(Clone, Debug)]
pub struct QLException {
    kind: QLExceptionKind,
    /// Fully formatted message (Java `RuntimeException.message`).
    message: String,
    diagnostic: Diagnostic,
    /// Object that can be caught by a QLExpress `catch` clause
    /// (Java `QLRuntimeException.catchObj`).
    catch_obj: Option<DataValue>,
}

impl QLException {
    pub(crate) fn new(
        kind: QLExceptionKind,
        message: impl Into<String>,
        diagnostic: Diagnostic,
        catch_obj: Option<DataValue>,
    ) -> Self {
        QLException {
            kind,
            message: message.into(),
            diagnostic,
            catch_obj,
        }
    }

    /// Builder-style setter for the catchable attachment.
    pub fn with_catch_obj(mut self, catch_obj: DataValue) -> Self {
        self.catch_obj = Some(catch_obj);
        self
    }

    /// Constructor mirroring the Java "Visible for test"
    /// `QLRuntimeException(catchObj, reason, errorCode)`.
    pub fn for_test(kind: QLExceptionKind, reason: impl Into<String>, error_code: &str) -> Self {
        QLException::new(
            kind,
            "",
            Diagnostic::new(0, Range::default(), "", error_code, reason.into(), ""),
            None,
        )
    }

    pub fn kind(&self) -> QLExceptionKind {
        self.kind
    }

    pub fn is_syntax(&self) -> bool {
        self.kind == QLExceptionKind::Syntax
    }

    pub fn is_timeout(&self) -> bool {
        self.kind == QLExceptionKind::Timeout
    }

    pub fn diagnostic(&self) -> &Diagnostic {
        &self.diagnostic
    }

    pub fn pos(&self) -> i32 {
        self.diagnostic.pos()
    }

    /// The unformatted reason of this error.
    pub fn reason(&self) -> &str {
        self.diagnostic.message()
    }

    /// Line no, 1-based.
    pub fn line_no(&self) -> i32 {
        self.diagnostic.range().start().line() + 1
    }

    /// Column no, 1-based.
    pub fn col_no(&self) -> i32 {
        self.diagnostic.range().start().character() + 1
    }

    pub fn err_lexeme(&self) -> &str {
        self.diagnostic.lexeme()
    }

    pub fn error_code(&self) -> &str {
        self.diagnostic.code()
    }

    /// Object catchable in a QLExpress catch clause (Java `getCatchObj`).
    pub fn catch_obj(&self) -> Option<&DataValue> {
        self.catch_obj.as_ref()
    }

    /// Report a scanner/syntax error, mirroring Java
    /// `QLException.reportScannerErr`.
    #[allow(clippy::too_many_arguments)]
    pub fn report_scanner_err(
        script: &str,
        token_start_pos: i32,
        line: i32,
        col: i32,
        lexeme: &str,
        error_code: &str,
        reason: &str,
    ) -> QLSyntaxException {
        let ex_message = ExMessageUtil::format(
            script,
            token_start_pos,
            line,
            col,
            lexeme,
            error_code,
            reason,
        );
        let diagnostic = to_diagnostic(
            token_start_pos,
            line,
            col,
            lexeme,
            error_code,
            reason,
            ex_message.snippet(),
        );
        QLSyntaxException::from_exception(QLException::new(
            QLExceptionKind::Syntax,
            ex_message.message(),
            diagnostic,
            None,
        ))
    }

    /// Report a runtime error carrying a catchable attachment, mirroring Java
    /// `QLException.reportRuntimeErrWithAttach`. A `SCRIPT_TIME_OUT` code
    /// yields [`QLExceptionKind::Timeout`].
    #[allow(clippy::too_many_arguments)]
    pub fn report_runtime_err_with_attach(
        script: &str,
        token_start_pos: i32,
        line: i32,
        col: i32,
        lexeme: &str,
        error_code: &str,
        reason: &str,
        catch_obj: Option<DataValue>,
    ) -> QLException {
        let ex_message = ExMessageUtil::format(
            script,
            token_start_pos,
            line,
            col,
            lexeme,
            error_code,
            reason,
        );
        let diagnostic = to_diagnostic(
            token_start_pos,
            line,
            col,
            lexeme,
            error_code,
            reason,
            ex_message.snippet(),
        );
        let kind = if error_code == error_codes::SCRIPT_TIME_OUT {
            QLExceptionKind::Timeout
        } else {
            QLExceptionKind::Runtime
        };
        QLException::new(kind, ex_message.message(), diagnostic, catch_obj)
    }
}

fn to_diagnostic(
    start_pos: i32,
    line: i32,
    col: i32,
    lexeme: &str,
    error_code: &str,
    reason: &str,
    snippet: &str,
) -> Diagnostic {
    let zero_based_line = line - 1;
    let zero_based_col = col - 1;
    let lexeme_len = lexeme.chars().count() as i32;
    let start = Position::new(zero_based_line, zero_based_col);
    let end = Position::new(zero_based_line, zero_based_col + lexeme_len);
    Diagnostic::new(
        start_pos,
        Range::new(start, end),
        lexeme,
        error_code,
        reason,
        snippet,
    )
}

impl fmt::Display for QLException {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for QLException {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scanner_err_reports_1_based_line_col() {
        let err = QLException::report_scanner_err(
            "a = @",
            4,
            1,
            5,
            "@",
            error_codes::SYNTAX_ERROR,
            "unexpected char",
        )
        .into_exception();
        assert!(err.is_syntax());
        assert_eq!(err.line_no(), 1);
        assert_eq!(err.col_no(), 5);
        assert_eq!(err.pos(), 4);
        assert_eq!(err.err_lexeme(), "@");
        assert_eq!(err.error_code(), error_codes::SYNTAX_ERROR);
        assert_eq!(err.reason(), "unexpected char");
        // Diagnostic stores zero-based positions (Java toDiagnostic).
        assert_eq!(err.diagnostic().range().start().line(), 0);
        assert_eq!(err.diagnostic().range().end().character(), 5);
    }

    #[test]
    fn timeout_code_yields_timeout_kind() {
        let err = QLException::report_runtime_err_with_attach(
            "while(true){}",
            0,
            1,
            1,
            "while",
            error_codes::SCRIPT_TIME_OUT,
            "script exceeds timeout milliseconds, which is 10 ms",
            None,
        );
        assert_eq!(err.kind(), QLExceptionKind::Timeout);
        assert!(err.is_timeout());
    }

    #[test]
    fn runtime_err_keeps_catch_obj() {
        let err = QLException::report_runtime_err_with_attach(
            "1/0",
            0,
            1,
            1,
            "1/0",
            error_codes::INVALID_ARITHMETIC,
            "div by zero",
            Some(DataValue::Int(3)),
        );
        assert_eq!(err.kind(), QLExceptionKind::Runtime);
        assert_eq!(err.catch_obj(), Some(&DataValue::Int(3)));
    }
}
