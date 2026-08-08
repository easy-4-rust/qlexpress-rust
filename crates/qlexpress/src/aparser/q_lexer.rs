//! Hand-written scanner, mirroring Java `com.alibaba.qlexpress4.aparser.QLexer`.
//!
//! Produces the full token stream for a script: keywords, identifiers
//! (Unicode letters plus `#`/`@`/`$`/`_` starts), numbers (decimal, `0x`
//! hex, `0b` binary, exponents, `_` separators, `l/L` integer and
//! `f/F/d/D` float suffixes — note: like the Java version, leading-zero
//! "octal" is *not* special-cased), single- and double-quoted strings with
//! `${}` interpolation ([`InterpolationMode`]), selectors, the full operator
//! table with longest-match first, custom operators (OPID), line/block
//! comments, and NEWLINE tokens under `strictNewLines`.
//!
//! Java indexes the script by UTF-16 code unit. The scanner keeps Unicode
//! scalar values for safe Rust slicing, but converts every token offset and
//! column back to UTF-16 units, matching `String.charAt` positions.
//!
//! Scanner errors are reported with
//! [`QLException::report_scanner_err`] (`SYNTAX_ERROR` code), mirroring
//! Java `QLexer.scannerError` — the Java lexer does not go through
//! `ErrorReporter` for scanning failures.

// The Err variant is `QLSyntaxException` by value, mirroring the Java
// scanner which throws it directly; boxing it would deviate from the
// Stage-0 error model.
#![allow(clippy::result_large_err)]

use super::interpolation_mode::InterpolationMode;
use super::parser_operator_manager::ParserOperatorManager;
use super::token::{self, Token};
use crate::exception::error_codes;
use crate::exception::ql_exception::QLException;
use crate::exception::ql_syntax_exception::QLSyntaxException;

/// 将脚本源码扫描为带位置的 Token 序列。
/// 参数：`script`、`operator_manager`、`interpolation_mode`、`selector_start`、`selector_end`、`strict_new_lines`；返回：`Result<Vec<Token>, QLSyntaxException>`。
/// 对应或承接 Java 源文件：`com/alibaba/qlexpress4/aparser/QLexer.java`，方法 `tokenize`；Rust 侧按所有权与 `Result` 语义适配。
/// Java `QLexer.tokenize`: scan the whole script and append a final
/// [`token::EOF`] token (`<EOF>`).
///
/// `script` is treated as empty when `None`-equivalent (Java maps `null` to
/// `""`); pass `Some(operator_manager)` to enable word-operator aliases.
#[allow(clippy::too_many_arguments)]
/// 对应 Java: com.alibaba.qlexpress4.aparser.QLexer#tokenize。
pub fn tokenize(
    script: &str,
    operator_manager: Option<&dyn ParserOperatorManager>,
    interpolation_mode: InterpolationMode,
    selector_start: &str,
    selector_end: &str,
    strict_new_lines: bool,
) -> Result<Vec<Token>, QLSyntaxException> {
    tokenize_with_limit(
        script,
        operator_manager,
        interpolation_mode,
        selector_start,
        selector_end,
        strict_new_lines,
        None,
    )
}

/// 带 Token 数量硬上限的词法入口；达到上限后不再向 Token 向量分配。
#[allow(clippy::too_many_arguments)]
/// 对应 Java：`QLexer` 词法流程（Rust 安全增强 Token 数量预算）。
pub fn tokenize_with_limit(
    script: &str,
    operator_manager: Option<&dyn ParserOperatorManager>,
    interpolation_mode: InterpolationMode,
    selector_start: &str,
    selector_end: &str,
    strict_new_lines: bool,
    max_tokens: Option<usize>,
) -> Result<Vec<Token>, QLSyntaxException> {
    let mut lexer = QLexer::new(
        script,
        operator_manager,
        interpolation_mode,
        selector_start,
        selector_end,
        strict_new_lines,
        max_tokens,
    );
    lexer.lex_default(false)?;
    if lexer.token_limit_exceeded {
        return Err(QLException::report_scanner_err(
            script,
            lexer.utf16_index(lexer.p),
            lexer.line,
            lexer.col + 1,
            "",
            "SANDBOX_TOKENS_EXCEEDED",
            "sandbox token budget exceeded",
        ));
    }
    let p = lexer.p as i32;
    let line = lexer.line;
    let col = lexer.col;
    lexer.add(token::EOF, "<EOF>".to_string(), p, p - 1, line, col);
    Ok(lexer.tokens)
}

/// The scanner state, mirroring the fields of Java `QLexer`.
struct QLexer<'a> {
    /// Original script, kept for error reporting (`report_scanner_err`).
    script: &'a str,
    /// Script as chars; `p` indexes this vector (Java indexes the String).
    chars: Vec<char>,
    /// `utf16_offsets[i]` 是 `chars[i]` 在 Java `String` 中的起始下标；
    /// 末尾额外保存脚本 UTF-16 长度。
    utf16_offsets: Vec<i32>,
    operator_manager: Option<&'a dyn ParserOperatorManager>,
    interpolation_mode: InterpolationMode,
    selector_start: Vec<char>,
    selector_end: Vec<char>,
    strict_new_lines: bool,
    tokens: Vec<Token>,
    max_tokens: Option<usize>,
    token_limit_exceeded: bool,
    p: usize,
    line: i32,
    col: i32,
}

include!("q_lexer/core_and_strings.rs");
include!("q_lexer/numbers_and_identifiers.rs");
include!("q_lexer/operators_and_tokens.rs");

/// Java `isIdStart`: `#`, `@`, `$`, `_`, or a Unicode letter.
fn is_id_start(c: char) -> bool {
    c == '#' || c == '@' || c == '$' || c == '_' || (c.len_utf16() == 1 && c.is_alphabetic())
}

/// Java `isIdPart`: id start, digits, and the CJK punctuation `、（）【】`.
fn is_id_part(c: char) -> bool {
    is_id_start(c)
        || is_java_digit(c)
        || c == '\u{3001}'
        || c == '\u{FF08}'
        || c == '\u{FF09}'
        || c == '\u{3010}'
        || c == '\u{3011}'
}

/// Java `isAsciiLetter`.
fn is_ascii_letter(c: char) -> bool {
    c.is_ascii_lowercase() || c.is_ascii_uppercase()
}

/// Java `isFloatSuffix`.
fn is_float_suffix(c: char) -> bool {
    c == 'f' || c == 'F' || c == 'd' || c == 'D'
}

/// Java `isCustomOperatorStart`.
fn is_custom_operator_start(c: char) -> bool {
    matches!(
        c,
        '^' | '~' | '&' | '|' | '*' | '%' | '=' | '!' | '/' | '+' | '-' | '?' | '.'
    )
}

/// Java `isCustomOperatorPart`.
fn is_custom_operator_part(c: char) -> bool {
    is_custom_operator_start(c) || c == '<' || c == '>' || c == ':'
}

/// Java `Character.isDigit(char)`：只接受 BMP 的 Unicode `Nd`。
/// 对应 Java：`java.lang.Character#isDigit(char)`。
pub(crate) fn is_java_digit(c: char) -> bool {
    java_decimal_digit_value(c).is_some()
}

/// Java `Character.digit(char, radix)` 的 QLexer 所需子集。
///
/// 除 Unicode 十进制数字外，Java 还接受 ASCII 与全角拉丁字母作为
/// 10..35。返回值大于等于 `radix` 时视为非法。
/// 对应 Java：`java.lang.Character#digit(char,int)`。
pub(crate) fn java_digit_value(c: char, radix: u32) -> Option<u32> {
    let value = java_decimal_digit_value(c).or_else(|| match c {
        'a'..='z' => Some(c as u32 - 'a' as u32 + 10),
        'A'..='Z' => Some(c as u32 - 'A' as u32 + 10),
        '\u{FF41}'..='\u{FF5A}' => Some(c as u32 - '\u{FF41}' as u32 + 10),
        '\u{FF21}'..='\u{FF3A}' => Some(c as u32 - '\u{FF21}' as u32 + 10),
        _ => None,
    })?;
    (value < radix).then_some(value)
}

fn java_decimal_digit_value(c: char) -> Option<u32> {
    const STARTS: &[u32] = &[
        0x0030, 0x0660, 0x06F0, 0x07C0, 0x0966, 0x09E6, 0x0A66, 0x0AE6, 0x0B66, 0x0BE6, 0x0C66,
        0x0CE6, 0x0D66, 0x0DE6, 0x0E50, 0x0ED0, 0x0F20, 0x1040, 0x1090, 0x17E0, 0x1810, 0x1946,
        0x19D0, 0x1A80, 0x1A90, 0x1B50, 0x1BB0, 0x1C40, 0x1C50, 0xA620, 0xA8D0, 0xA900, 0xA9D0,
        0xA9F0, 0xAA50, 0xABF0, 0xFF10,
    ];
    let code = c as u32;
    STARTS
        .iter()
        .find_map(|start| (code >= *start && code < *start + 10).then(|| code - *start))
}

#[cfg(test)]
#[path = "q_lexer_tests.rs"]
mod tests;
