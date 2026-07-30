//! 对应 Java 类：com.alibaba.qlexpress4.exception.QLException
//!
//! QLExpress4 统一异常类型，携带完整诊断信息（错误码、源位置、cause 链）。
//! Rust 侧以单一 struct + `QLExceptionKind` 枚举（Syntax/Runtime/Timeout/UserDefine）
//! 表达 Java 的四类异常，体积较大是架构性选择（文件级 allow clippy::result_large_err）。

use std::fmt;

use super::error_codes;
use super::ex_message_util::ExMessageUtil;
use super::lsp::{Diagnostic, Position, Range};
use super::ql_syntax_exception::QLSyntaxException;
use crate::runtime::value::DataValue;

/// `QLExceptionKind` 枚举的 Rust 实现，保留对应对象的领域职责与公开契约。
/// 对应或承接 Java 源文件：`com/alibaba/qlexpress4/exception/QLException.java`；具体对象路径见 `docs/对象级对照表.md`。
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

/// `QLException` 结构体的 Rust 实现，保留对应对象的领域职责与公开契约。
/// 对应或承接 Java 源文件：`com/alibaba/qlexpress4/exception/QLException.java`；具体对象路径见 `docs/对象级对照表.md`。
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
    /// 被当前异常包装的底层异常。对应 Java `Throwable#getCause()`。
    cause: Option<Box<QLException>>,
    /// 是否代表宿主函数/原生方法抛出的底层异常。Java 通过
    /// `Throwable` 与 `QLRuntimeException` 的实际类型区分；Rust 的统一
    /// 错误类型用此标记保留同一分派语义。
    host_origin: bool,
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
            cause: None,
            host_origin: false,
        }
    }

    /// 创建宿主函数或原生方法抛出的底层异常。
    ///
    /// 对应 Java 中尚未被 QLExpress 包装的任意 `Throwable`；调用指令会
    /// 使用当前位置的 `ErrorReporter` 生成外层异常，并把本值设为 cause。
    pub fn host_error(kind: QLExceptionKind, reason: impl Into<String>, error_code: &str) -> Self {
        let reason = reason.into();
        let mut error = QLException::new(
            kind,
            reason.clone(),
            Diagnostic::new(0, Range::default(), "", error_code, reason, ""),
            None,
        );
        error.host_origin = true;
        error
    }

    /// 附加底层 cause 并返回新值。对应 Java `Throwable(Throwable cause)`。
    pub fn with_cause(mut self, cause: QLException) -> Self {
        self.cause = Some(Box::new(cause));
        self
    }

    /// 返回底层 cause。对应 Java `Throwable#getCause()`。
    pub fn cause(&self) -> Option<&QLException> {
        self.cause.as_deref()
    }

    /// 判断该错误是否是尚未被引擎包装的宿主异常。
    pub(crate) fn is_host_origin(&self) -> bool {
        self.host_origin
    }

    /// 附加 catch obj 配置并返回新值。
    /// 参数：`catch_obj`；返回：`Self`。
    /// 对应或承接 Java 源文件：`com/alibaba/qlexpress4/exception/QLException.java`，方法 `withCatchObj`；Rust 侧按所有权与 `Result` 语义适配。
    /// Builder-style setter for the catchable attachment.
    pub fn with_catch_obj(mut self, catch_obj: DataValue) -> Self {
        self.catch_obj = Some(catch_obj);
        self
    }

    /// 处理 for test 对应的领域职责。
    /// 参数：`kind`、`reason`、`error_code`；返回：`Self`。
    /// 对应或承接 Java 源文件：`com/alibaba/qlexpress4/exception/QLException.java`，方法 `forTest`；Rust 侧按所有权与 `Result` 语义适配。
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

    /// 返回异常类别，用于区分语法、运行时、超时与用户异常。
    /// 对应 Java: `QLException` 的具体子类类型。
    pub fn kind(&self) -> QLExceptionKind {
        self.kind
    }

    /// 判断当前异常是否来自语法分析阶段。
    /// 对应 Java: `exception instanceof QLSyntaxException`。
    pub fn is_syntax(&self) -> bool {
        self.kind == QLExceptionKind::Syntax
    }

    /// 判断当前异常是否为执行超时。
    /// 对应 Java: `exception instanceof QLTimeoutException`。
    pub fn is_timeout(&self) -> bool {
        self.kind == QLExceptionKind::Timeout
    }

    /// 返回完整 LSP 诊断；没有脚本位置时返回 `None`。
    /// 对应 Java: `QLException#getDiagnostic`。
    pub fn diagnostic(&self) -> &Diagnostic {
        &self.diagnostic
    }

    /// 返回错误 token 的绝对字符偏移。
    /// 对应 Java: `QLException#getPos`。
    pub fn pos(&self) -> i32 {
        self.diagnostic.pos()
    }

    /// 处理 reason 对应的领域职责。
    /// 无显式参数；返回：`&str`。
    /// 对应或承接 Java 源文件：`com/alibaba/qlexpress4/exception/QLException.java`，方法 `reason`；Rust 侧按所有权与 `Result` 语义适配。
    /// The unformatted reason of this error.
    pub fn reason(&self) -> &str {
        self.diagnostic.message()
    }

    /// 处理 line no 对应的领域职责。
    /// 无显式参数；返回：`i32`。
    /// 对应或承接 Java 源文件：`com/alibaba/qlexpress4/exception/QLException.java`，方法 `lineNo`；Rust 侧按所有权与 `Result` 语义适配。
    /// Line no, 1-based.
    pub fn line_no(&self) -> i32 {
        self.diagnostic.range().start().line() + 1
    }

    /// 处理 col no 对应的领域职责。
    /// 无显式参数；返回：`i32`。
    /// 对应或承接 Java 源文件：`com/alibaba/qlexpress4/exception/QLException.java`，方法 `colNo`；Rust 侧按所有权与 `Result` 语义适配。
    /// Column no, 1-based.
    pub fn col_no(&self) -> i32 {
        self.diagnostic.range().start().character() + 1
    }

    /// 返回触发错误的词素。
    /// 对应 Java: `QLException#getErrLexeme`。
    pub fn err_lexeme(&self) -> &str {
        self.diagnostic.lexeme()
    }

    /// 返回稳定的 QLExpress 错误码。
    /// 对应 Java: `QLException#getErrorCode`。
    pub fn error_code(&self) -> &str {
        self.diagnostic.code()
    }

    /// 处理 catch obj 对应的领域职责。
    /// 无显式参数；返回：`Option<&DataValue>`。
    /// 对应或承接 Java 源文件：`com/alibaba/qlexpress4/exception/QLException.java`，方法 `catchObj`；Rust 侧按所有权与 `Result` 语义适配。
    /// Object catchable in a QLExpress catch clause (Java `getCatchObj`).
    pub fn catch_obj(&self) -> Option<&DataValue> {
        self.catch_obj.as_ref()
    }

    /// 处理 report scanner err 对应的领域职责。
    /// 参数：`script`、`token_start_pos`、`line`、`col`、`lexeme`、`error_code`、`reason`；返回：`QLSyntaxException`。
    /// 对应或承接 Java 源文件：`com/alibaba/qlexpress4/exception/QLException.java`，方法 `reportScannerErr`；Rust 侧按所有权与 `Result` 语义适配。
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

    /// 处理 report runtime err with attach 对应的领域职责。
    /// 参数：`script`、`token_start_pos`、`line`、`col`、`lexeme`、`error_code`、`reason`、`catch_obj`；返回：`QLException`。
    /// 对应或承接 Java 源文件：`com/alibaba/qlexpress4/exception/QLException.java`，方法 `reportRuntimeErrWithAttach`；Rust 侧按所有权与 `Result` 语义适配。
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

    /// 逐项对应 Java `QLExceptionTest#reportTest`。
    #[test]
    fn scanner_error_message_matches_java_report_test() {
        let script = "if (3>1) {\n  break 9;\n} else {\n  return 11;\n}";
        let error = QLException::report_scanner_err(
            script,
            13,
            2,
            3,
            "break",
            "BREAK_MUST_IN_FOR_OR_WHILE",
            "break must in for/while",
        );
        assert_eq!(
            error.to_string(),
            concat!(
                "[Error BREAK_MUST_IN_FOR_OR_WHILE: break must in for/while]\n",
                "[Near: if (3>1) {   break 9; } else {   retur...]\n",
                "                    ^^^^^\n",
                "[Line: 2, Column: 3]"
            )
        );
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
