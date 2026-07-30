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

impl<'a> QLexer<'a> {
    fn new(
        script: &'a str,
        operator_manager: Option<&'a dyn ParserOperatorManager>,
        interpolation_mode: InterpolationMode,
        selector_start: &str,
        selector_end: &str,
        strict_new_lines: bool,
        max_tokens: Option<usize>,
    ) -> Self {
        let chars: Vec<char> = script.chars().collect();
        let mut utf16_offsets = Vec::with_capacity(chars.len() + 1);
        let mut utf16_offset = 0_i32;
        for character in &chars {
            utf16_offsets.push(utf16_offset);
            utf16_offset += character.len_utf16() as i32;
        }
        utf16_offsets.push(utf16_offset);
        QLexer {
            script,
            chars,
            utf16_offsets,
            operator_manager,
            interpolation_mode,
            selector_start: selector_start.chars().collect(),
            selector_end: selector_end.chars().collect(),
            strict_new_lines,
            tokens: Vec::new(),
            max_tokens,
            token_limit_exceeded: false,
            p: 0,
            line: 1,
            col: 0,
        }
    }

    /// Java `lexDefault`: the main scan loop. With
    /// `stop_at_string_expression_brace` it scans one `${...}` interpolation
    /// expression, tracking nested `{...}` depth.
    fn lex_default(
        &mut self,
        stop_at_string_expression_brace: bool,
    ) -> Result<(), QLSyntaxException> {
        let mut brace_depth = 1;
        while !self.eof() {
            let c = self.ch();
            if stop_at_string_expression_brace && c == '}' {
                let (p, line, col) = (self.p as i32, self.line, self.col);
                self.add(token::RBRACE as i32, "}".to_string(), p, p, line, col);
                self.advance();
                brace_depth -= 1;
                if brace_depth == 0 {
                    return Ok(());
                }
                continue;
            }
            if stop_at_string_expression_brace && c == '{' {
                let (p, line, col) = (self.p as i32, self.line, self.col);
                self.add(token::LBRACE as i32, "{".to_string(), p, p, line, col);
                self.advance();
                brace_depth += 1;
                continue;
            }
            if c == ' ' || c == '\t' || c == '\x0C' {
                self.advance();
                continue;
            }
            if c == '\r' || c == '\n' {
                self.read_newline();
                continue;
            }
            if self.starts_with("//") {
                self.skip_line_comment();
                continue;
            }
            if self.starts_with("/*") {
                self.skip_block_comment()?;
                continue;
            }
            if self.starts_with_chars(&self.selector_start.clone()) {
                self.read_selector()?;
                continue;
            }
            if c == '\'' {
                self.read_quote_string()?;
                continue;
            }
            if c == '"' {
                self.read_double_quote_string()?;
                continue;
            }
            if is_java_digit(c)
                || (c == '.'
                    && self.p + 1 < self.chars.len()
                    && is_java_digit(self.chars[self.p + 1]))
            {
                self.read_number();
                continue;
            }
            if is_id_start(c) {
                self.read_identifier();
                continue;
            }
            self.read_operator_or_punctuation();
        }
        if stop_at_string_expression_brace {
            let tok = self.current_token("<EOF>");
            return Err(self.scanner_error(&tok, "mismatched input '<EOF>' expecting '}'"));
        }
        Ok(())
    }

    /// Java `readNewline`: consume one line break (`\r\n` counts once) and
    /// emit a NEWLINE token only under `strictNewLines`.
    fn read_newline(&mut self) {
        let start = self.p;
        let start_line = self.line;
        let start_col = self.col;
        if self.ch() == '\r' {
            self.advance();
            if !self.eof() && self.ch() == '\n' {
                self.advance();
            }
        } else {
            self.advance();
        }
        if self.strict_new_lines {
            let text: String = self.chars[start..self.p].iter().collect();
            self.add(
                token::NEWLINE as i32,
                text,
                start as i32,
                self.p as i32 - 1,
                start_line,
                start_col,
            );
        }
    }

    /// Java `skipLineComment`: `//` to end of line (the newline itself is
    /// left for `readNewline`).
    fn skip_line_comment(&mut self) {
        while !self.eof() && self.ch() != '\r' && self.ch() != '\n' {
            self.advance();
        }
    }

    /// Java `skipBlockComment`: `/* ... */`; unterminated is a scanner error.
    fn skip_block_comment(&mut self) -> Result<(), QLSyntaxException> {
        let start_token = self.current_token("/*");
        self.advance();
        self.advance();
        while !self.eof() {
            if self.starts_with("*/") {
                self.advance();
                self.advance();
                return Ok(());
            }
            self.advance();
        }
        Err(self.scanner_error(&start_token, "unterminated comment"))
    }

    /// Java `readSelector`: `selectorStart ... selectorEnd` on a single
    /// line, emitting SELECTOR_START + SelectorVariable_VANME.
    fn read_selector(&mut self) -> Result<(), QLSyntaxException> {
        let start = self.p;
        let start_line = self.line;
        let start_col = self.col;
        let selector_start = self.selector_start.clone();
        self.add(
            token::SELECTOR_START as i32,
            selector_start.iter().collect(),
            start as i32,
            (start + selector_start.len() - 1) as i32,
            start_line,
            start_col,
        );
        for _ in 0..selector_start.len() {
            self.advance();
        }
        let content_start = self.p;
        let content_line = self.line;
        let content_col = self.col;
        while !self.eof() {
            if self.starts_with_chars(&self.selector_end.clone()) {
                let text: String = self.chars[content_start..self.p].iter().collect();
                self.add(
                    token::SELECTOR_VARIABLE_VANME as i32,
                    text,
                    content_start as i32,
                    self.p as i32 - 1,
                    content_line,
                    content_col,
                );
                for _ in 0..self.selector_end.len() {
                    self.advance();
                }
                return Ok(());
            }
            if self.ch() == '\n' || self.ch() == '\r' {
                let text: String = self.chars[content_start..self.p].iter().collect();
                let tok = self.current_token(&text);
                return Err(self.scanner_error(&tok, "unterminated selector"));
            }
            self.advance();
        }
        let text: String = self.chars[content_start..].iter().collect();
        let tok = self.current_token(&text);
        Err(self.scanner_error(&tok, "unterminated selector"))
    }

    /// Java `readQuoteString`: single-quoted literal (text includes quotes).
    /// Only `\'` is escape-consumed; any other `\x` leaves `x` to the next
    /// iteration, exactly like the Java version.
    fn read_quote_string(&mut self) -> Result<(), QLSyntaxException> {
        let start = self.p;
        let start_line = self.line;
        let start_col = self.col;
        self.advance();
        while !self.eof() {
            let c = self.ch();
            self.advance();
            if c == '\\' {
                if !self.eof() && self.ch() == '\'' {
                    self.advance();
                }
                continue;
            }
            if c == '\'' {
                let text: String = self.chars[start..self.p].iter().collect();
                self.add(
                    token::QUOTE_STRING_LITERAL as i32,
                    text,
                    start as i32,
                    self.p as i32 - 1,
                    start_line,
                    start_col,
                );
                return Ok(());
            }
        }
        let text: String = self.chars[start..].iter().collect();
        let tok = Token::new(
            token::QUOTE_STRING_LITERAL as i32,
            text,
            self.utf16_index(start),
            self.utf16_stop_index(self.p.saturating_sub(1)),
            start_line,
            start_col,
        );
        Err(self.scanner_error(&tok, "unterminated string literal"))
    }

    /// Java `readDoubleQuoteString`: emits DOUBLE_QUOTE ... DOUBLE_QUOTE,
    /// with StaticStringCharacters (DISABLE) or DyStrText/DyStrExprStart
    /// plus a nested expression (VARIABLE/SCRIPT) in between.
    fn read_double_quote_string(&mut self) -> Result<(), QLSyntaxException> {
        let quote_start = self.p;
        let quote_line = self.line;
        let quote_col = self.col;
        self.add(
            token::DOUBLE_QUOTE as i32,
            "\"".to_string(),
            quote_start as i32,
            quote_start as i32,
            quote_line,
            quote_col,
        );
        self.advance();
        if self.interpolation_mode == InterpolationMode::Disable {
            let text_start = self.p;
            let text_line = self.line;
            let text_col = self.col;
            while !self.eof() {
                let c = self.ch();
                if c == '"' {
                    if self.p > text_start {
                        let text: String = self.chars[text_start..self.p].iter().collect();
                        self.add(
                            token::STATIC_STRING_CHARACTERS as i32,
                            text,
                            text_start as i32,
                            self.p as i32 - 1,
                            text_line,
                            text_col,
                        );
                    }
                    let (p, line, col) = (self.p as i32, self.line, self.col);
                    self.add(
                        token::DOUBLE_QUOTE as i32,
                        "\"".to_string(),
                        p,
                        p,
                        line,
                        col,
                    );
                    self.advance();
                    return Ok(());
                }
                self.advance();
                if c == '\\' && !self.eof() {
                    self.advance();
                }
            }
            let text: String = self.chars[text_start..].iter().collect();
            let tok = self.current_token(&text);
            return Err(self.scanner_error(&tok, "unterminated string literal"));
        }
        while !self.eof() {
            let text_start = self.p;
            let text_line = self.line;
            let text_col = self.col;
            while !self.eof() && self.ch() != '"' && !self.starts_with("${") {
                let c = self.ch();
                self.advance();
                if c == '\\' && !self.eof() {
                    self.advance();
                }
            }
            if self.p > text_start {
                let text: String = self.chars[text_start..self.p].iter().collect();
                self.add(
                    token::DY_STR_TEXT as i32,
                    text,
                    text_start as i32,
                    self.p as i32 - 1,
                    text_line,
                    text_col,
                );
            }
            if self.eof() {
                let text: String = self.chars[quote_start..].iter().collect();
                let tok = self.current_token(&text);
                return Err(self.scanner_error(&tok, "unterminated string literal"));
            }
            if self.ch() == '"' {
                let (p, line, col) = (self.p as i32, self.line, self.col);
                self.add(
                    token::DOUBLE_QUOTE as i32,
                    "\"".to_string(),
                    p,
                    p,
                    line,
                    col,
                );
                self.advance();
                return Ok(());
            }
            let expr_start = self.p;
            let expr_line = self.line;
            let expr_col = self.col;
            self.add(
                token::DY_STR_EXPR_START as i32,
                "${".to_string(),
                expr_start as i32,
                expr_start as i32 + 1,
                expr_line,
                expr_col,
            );
            self.advance();
            self.advance();
            if self.interpolation_mode == InterpolationMode::Variable {
                self.read_variable_string_expression()?;
            } else if self.interpolation_mode == InterpolationMode::Script {
                self.lex_default(true)?;
            }
        }
        Ok(())
    }

    /// Java `readVariableStringExpression`: VARIABLE-mode interpolation body
    /// up to `selectorEnd`, emitted as SelectorVariable_VANME.
    fn read_variable_string_expression(&mut self) -> Result<(), QLSyntaxException> {
        let content_start = self.p;
        let content_line = self.line;
        let content_col = self.col;
        while !self.eof() {
            if self.starts_with_chars(&self.selector_end.clone()) {
                let text: String = self.chars[content_start..self.p].iter().collect();
                self.add(
                    token::SELECTOR_VARIABLE_VANME as i32,
                    text,
                    content_start as i32,
                    self.p as i32 - 1,
                    content_line,
                    content_col,
                );
                for _ in 0..self.selector_end.len() {
                    self.advance();
                }
                return Ok(());
            }
            if self.ch() == '\r' || self.ch() == '\n' {
                let text: String = self.chars[content_start..self.p].iter().collect();
                let tok = self.current_token(&text);
                return Err(self.scanner_error(&tok, "unterminated selector"));
            }
            self.advance();
        }
        let text: String = self.chars[content_start..].iter().collect();
        let tok = self.current_token(&text);
        Err(self.scanner_error(&tok, "unterminated selector"))
    }

    /// Java `readNumber`. `.5` and digit-starting numbers; `0x`/`0X` hex and
    /// `0b`/`0B` binary become INTEGER_LITERAL, decimal point / exponent /
    /// float suffix yield FLOATING_POINT_LITERAL, everything else stays
    /// INTEGER_OR_FLOATING_LITERAL (including `1L`, as in Java).
    fn read_number(&mut self) {
        let start = self.p;
        let start_line = self.line;
        let start_col = self.col;
        let mut ty = token::INTEGER_OR_FLOATING_LITERAL;
        if self.ch() == '.' {
            self.advance();
            self.read_digits();
            self.read_exponent();
            self.read_float_suffix();
            let text: String = self.chars[start..self.p].iter().collect();
            self.add(
                token::FLOATING_POINT_LITERAL as i32,
                text,
                start as i32,
                self.p as i32 - 1,
                start_line,
                start_col,
            );
            return;
        }
        if self.starts_with("0x") || self.starts_with("0X") {
            self.advance();
            self.advance();
            self.read_digits_for_radix(16);
            self.read_integer_suffix();
            let text: String = self.chars[start..self.p].iter().collect();
            self.add(
                token::INTEGER_LITERAL as i32,
                text,
                start as i32,
                self.p as i32 - 1,
                start_line,
                start_col,
            );
            return;
        }
        if self.starts_with("0b") || self.starts_with("0B") {
            self.advance();
            self.advance();
            self.read_digits_for_radix(2);
            self.read_integer_suffix();
            let text: String = self.chars[start..self.p].iter().collect();
            self.add(
                token::INTEGER_LITERAL as i32,
                text,
                start as i32,
                self.p as i32 - 1,
                start_line,
                start_col,
            );
            return;
        }
        self.read_digits();
        let mut has_exponent = false;
        if !self.eof() && self.ch() == '.' && self.should_consume_decimal_dot() {
            self.advance();
            self.read_digits();
            self.read_exponent();
            self.read_float_suffix();
        } else if self.read_exponent() {
            has_exponent = true;
            self.read_float_suffix();
        } else if !self.eof() && is_float_suffix(self.ch()) {
            self.read_float_suffix();
            ty = token::FLOATING_POINT_LITERAL;
        } else {
            self.read_integer_suffix();
        }
        if has_exponent {
            ty = token::FLOATING_POINT_LITERAL;
        }
        let text: String = self.chars[start..self.p].iter().collect();
        self.add(
            ty as i32,
            text,
            start as i32,
            self.p as i32 - 1,
            start_line,
            start_col,
        );
    }

    /// Java `shouldConsumeDecimalDot`: `1.toString` must not swallow the
    /// dot — if the two chars after `.` are both ASCII letters, the `.`
    /// starts a member access instead of a fraction.
    fn should_consume_decimal_dot(&self) -> bool {
        if self.p + 2 >= self.chars.len() {
            return true;
        }
        let c1 = self.chars[self.p + 1];
        let c2 = self.chars[self.p + 2];
        !(is_ascii_letter(c1) && is_ascii_letter(c2))
    }

    /// Java `readDigits`: decimal digits and `_` separators.
    fn read_digits(&mut self) {
        while !self.eof() && (is_java_digit(self.ch()) || self.ch() == '_') {
            self.advance();
        }
    }

    /// Java `readDigitsForRadix`: digits valid in `radix` plus `_`.
    fn read_digits_for_radix(&mut self, radix: u32) {
        while !self.eof() && (java_digit_value(self.ch(), radix).is_some() || self.ch() == '_') {
            self.advance();
        }
    }

    /// Java `readExponent`: `e`/`E` with optional sign and at least one
    /// digit; backtracks (without consuming) when malformed.
    fn read_exponent(&mut self) -> bool {
        if self.eof() || (self.ch() != 'e' && self.ch() != 'E') {
            return false;
        }
        let save = self.p;
        self.advance();
        if !self.eof() && (self.ch() == '+' || self.ch() == '-') {
            self.advance();
        }
        if self.eof() || !is_java_digit(self.ch()) {
            self.p = save;
            return false;
        }
        self.read_digits();
        true
    }

    /// Java `readIntegerSuffix`: `l`/`L`.
    fn read_integer_suffix(&mut self) {
        if !self.eof() && (self.ch() == 'l' || self.ch() == 'L') {
            self.advance();
        }
    }

    /// Java `readFloatSuffix`: one of `f`/`F`/`d`/`D`.
    fn read_float_suffix(&mut self) {
        if !self.eof() && is_float_suffix(self.ch()) {
            self.advance();
        }
    }

    /// Java `readIdentifier`: keyword lookup first, then
    /// `ParserOperatorManager.getAlias` for word operators, else ID.
    fn read_identifier(&mut self) {
        let start = self.p;
        let start_line = self.line;
        let start_col = self.col;
        self.advance();
        while !self.eof() && is_id_part(self.ch()) {
            self.advance();
        }
        let text: String = self.chars[start..self.p].iter().collect();
        let mut ty: i32 = match token::keyword_type(&text) {
            Some(keyword) => keyword as i32,
            None => token::ID as i32,
        };
        if ty == token::ID as i32 {
            if let Some(manager) = self.operator_manager {
                if let Some(alias_type) = manager.get_alias(&text) {
                    ty = alias_type;
                }
            }
        }
        self.add(
            ty,
            text,
            start as i32,
            self.p as i32 - 1,
            start_line,
            start_col,
        );
    }

    /// Java `readOperatorOrPunctuation`: longest match first
    /// (`>>>=`, `>>>`, `>>=`, `<<=`, then 2-char operators, then custom
    /// operators, then single chars, else CATCH_ALL).
    fn read_operator_or_punctuation(&mut self) {
        let start = self.p;
        let start_line = self.line;
        let start_col = self.col;
        if self.starts_with(">>>=") {
            self.fixed(token::URSHIFT_ASSGIN, 4, start, start_line, start_col);
            return;
        }
        if self.starts_with(">>>") {
            self.fixed(token::URSHIFT, 3, start, start_line, start_col);
            return;
        }
        if self.starts_with(">>=") {
            self.fixed(token::RIGHSHIFT_ASSGIN, 3, start, start_line, start_col);
            return;
        }
        if self.starts_with("<<=") {
            self.fixed(token::LSHIFT_ASSGIN, 3, start, start_line, start_col);
            return;
        }
        if self.starts_with("->") {
            self.fixed(token::ARROW, 2, start, start_line, start_col);
            return;
        }
        if self.starts_with("::") {
            self.fixed(token::DCOLON, 2, start, start_line, start_col);
            return;
        }
        if self.starts_with("<>") {
            self.fixed(token::NOEQ, 2, start, start_line, start_col);
            return;
        }
        if self.starts_with(">>") {
            self.fixed(token::RIGHSHIFT, 2, start, start_line, start_col);
            return;
        }
        if self.starts_with("<<") {
            self.fixed(token::LEFTSHIFT, 2, start, start_line, start_col);
            return;
        }
        if self.starts_with(">=") {
            self.fixed(token::GE, 2, start, start_line, start_col);
            return;
        }
        if self.starts_with("<=") {
            self.fixed(token::LE, 2, start, start_line, start_col);
            return;
        }
        if self.starts_with("?.") {
            self.fixed(token::OPTIONAL_CHAINING, 2, start, start_line, start_col);
            return;
        }
        if self.starts_with("*.") {
            self.fixed(token::SPREAD_CHAINING, 2, start, start_line, start_col);
            return;
        }
        if self.starts_with(".*") {
            self.fixed(token::DOTMUL, 2, start, start_line, start_col);
            return;
        }
        if self.starts_with("+=") {
            self.fixed(token::ADD_ASSIGN, 2, start, start_line, start_col);
            return;
        }
        if self.starts_with("-=") {
            self.fixed(token::SUB_ASSIGN, 2, start, start_line, start_col);
            return;
        }
        if self.starts_with("&=") {
            self.fixed(token::AND_ASSIGN, 2, start, start_line, start_col);
            return;
        }
        if self.starts_with("|=") {
            self.fixed(token::OR_ASSIGN, 2, start, start_line, start_col);
            return;
        }
        if self.starts_with("*=") {
            self.fixed(token::MUL_ASSIGN, 2, start, start_line, start_col);
            return;
        }
        if self.starts_with("%=") {
            self.fixed(token::MOD_ASSIGN, 2, start, start_line, start_col);
            return;
        }
        if self.starts_with("/=") {
            self.fixed(token::DIV_ASSIGN, 2, start, start_line, start_col);
            return;
        }
        if self.starts_with("^=") {
            self.fixed(token::XOR_ASSIGN, 2, start, start_line, start_col);
            return;
        }
        if self.starts_with("++") {
            self.fixed(token::INC, 2, start, start_line, start_col);
            return;
        }
        if self.starts_with("--") {
            self.fixed(token::DEC, 2, start, start_line, start_col);
            return;
        }
        if self.starts_with("==") {
            self.fixed(token::OPID, 2, start, start_line, start_col);
            return;
        }
        if self.starts_with("!=") {
            self.fixed(token::OPID, 2, start, start_line, start_col);
            return;
        }
        if self.starts_with("&&") {
            self.fixed(token::OPID, 2, start, start_line, start_col);
            return;
        }
        if self.starts_with("||") {
            self.fixed(token::OPID, 2, start, start_line, start_col);
            return;
        }
        if is_custom_operator_start(self.ch())
            && self.p + 1 < self.chars.len()
            && is_custom_operator_part(self.chars[self.p + 1])
        {
            self.advance();
            while !self.eof() && is_custom_operator_part(self.ch()) {
                self.advance();
            }
            let text: String = self.chars[start..self.p].iter().collect();
            self.add(
                token::OPID as i32,
                text,
                start as i32,
                self.p as i32 - 1,
                start_line,
                start_col,
            );
            return;
        }
        let ty = match self.ch() {
            '(' => token::LPAREN,
            ')' => token::RPAREN,
            '{' => token::LBRACE,
            '}' => token::RBRACE,
            '[' => token::LBRACK,
            ']' => token::RBRACK,
            '.' => token::DOT,
            ';' => token::SEMI,
            ',' => token::COMMA,
            '?' => token::QUESTION,
            ':' => token::COLON,
            '>' => token::GT,
            '<' => token::LT,
            '=' => token::EQ,
            '^' => token::CARET,
            '!' => token::BANG,
            '~' => token::TILDE,
            '+' => token::ADD,
            '-' => token::SUB,
            '*' => token::MUL,
            '/' => token::DIV,
            '&' => token::BIT_AND,
            '|' => token::BIT_OR,
            '%' => token::MOD,
            _ => token::CATCH_ALL,
        };
        if ty == token::CATCH_ALL && self.ch().len_utf16() == 2 {
            // Java `charAt` 会把非 BMP 标量拆成两个 surrogate `char`，
            // 因而产生两个 CATCH_ALL Token。Rust `String` 不能保存未配对
            // surrogate，文本以 U+FFFD 表示，但 token 数、UTF-16 范围和列
            // 与 Java 保持一致。
            let start_utf16 = self.utf16_index(start);
            self.add_utf16_unit_token(start_utf16, start_line, start_col);
            self.add_utf16_unit_token(start_utf16 + 1, start_line, start_col + 1);
            self.advance();
            return;
        }
        self.fixed(ty, 1, start, start_line, start_col);
    }

    /// Java `fixed`: emit a fixed-length token starting at `start`.
    fn fixed(&mut self, ty: u16, length: usize, start: usize, start_line: i32, start_col: i32) {
        for _ in 0..length {
            self.advance();
        }
        let text: String = self.chars[start..start + length].iter().collect();
        self.add(
            ty as i32,
            text,
            start as i32,
            (start + length - 1) as i32,
            start_line,
            start_col,
        );
    }

    /// Java `add`: append a token (stop index clamped to the start index).
    fn add(
        &mut self,
        ty: i32,
        text: String,
        start_index: i32,
        stop_index: i32,
        line: i32,
        col: i32,
    ) {
        if self
            .max_tokens
            .is_some_and(|max_tokens| self.tokens.len() >= max_tokens)
        {
            self.token_limit_exceeded = true;
            return;
        }
        let start_char_index = start_index.max(0) as usize;
        let start_utf16_index = self.utf16_index(start_char_index);
        let stop_utf16_index = if stop_index < 0 || stop_index < start_index {
            start_utf16_index
        } else {
            self.utf16_stop_index(stop_index as usize)
                .max(start_utf16_index)
        };
        self.tokens.push(Token::new(
            ty,
            text,
            start_utf16_index,
            stop_utf16_index,
            line,
            col,
        ));
    }

    /// 添加一个 Java UTF-16 `char` 对应的 token。
    ///
    /// 只用于 Rust 无法直接表示的未配对 surrogate；U+FFFD 是 UTF-8
    /// 输出适配，位置和 token 计数仍按 Java 原值。
    fn add_utf16_unit_token(&mut self, utf16_index: i32, line: i32, col: i32) {
        if self
            .max_tokens
            .is_some_and(|max_tokens| self.tokens.len() >= max_tokens)
        {
            self.token_limit_exceeded = true;
            return;
        }
        self.tokens.push(Token::new(
            token::CATCH_ALL as i32,
            "\u{FFFD}",
            utf16_index,
            utf16_index,
            line,
            col,
        ));
    }

    fn eof(&self) -> bool {
        self.p >= self.chars.len()
    }

    fn ch(&self) -> char {
        self.chars[self.p]
    }

    /// Java `startsWith(String)`.
    fn starts_with(&self, text: &str) -> bool {
        let pat: Vec<char> = text.chars().collect();
        self.starts_with_chars(&pat)
    }

    fn starts_with_chars(&self, pat: &[char]) -> bool {
        if self.p + pat.len() > self.chars.len() {
            return false;
        }
        self.chars[self.p..self.p + pat.len()] == *pat
    }

    /// Java `advance`: `\r\n` counts as one line break.
    fn advance(&mut self) {
        if self.eof() {
            return;
        }
        let c = self.chars[self.p];
        self.p += 1;
        if c == '\r' {
            if self.p < self.chars.len() && self.chars[self.p] == '\n' {
                self.p += 1;
            }
            self.line += 1;
            self.col = 0;
        } else if c == '\n' {
            self.line += 1;
            self.col = 0;
        } else {
            self.col += c.len_utf16() as i32;
        }
    }

    /// 把 Rust 字符下标转换为 Java `String` UTF-16 code-unit 下标。
    fn utf16_index(&self, char_index: usize) -> i32 {
        self.utf16_offsets
            .get(char_index)
            .copied()
            .unwrap_or_else(|| *self.utf16_offsets.last().unwrap_or(&0))
    }

    /// 把包含式 Rust 字符结束下标转换为 Java 包含式 UTF-16 结束下标。
    fn utf16_stop_index(&self, char_stop_index: usize) -> i32 {
        self.utf16_index(char_stop_index.saturating_add(1))
            .saturating_sub(1)
    }

    /// Java `currentToken`: a CATCH_ALL token describing the error site.
    fn current_token(&self, text: &str) -> Token {
        let p = self.utf16_index(self.p);
        Token::new(
            token::CATCH_ALL as i32,
            text,
            p,
            p.max(p + text.encode_utf16().count() as i32 - 1),
            self.line,
            self.col,
        )
    }

    /// Java `scannerError`: report via `QLException.report_scanner_err` with
    /// the `SYNTAX_ERROR` code; col is converted to 1-based here (Java
    /// passes `charPositionInLine + 1`).
    fn scanner_error(&self, tok: &Token, reason: &str) -> QLSyntaxException {
        QLException::report_scanner_err(
            self.script,
            tok.start_index(),
            tok.line(),
            tok.char_position_in_line() + 1,
            tok.text(),
            error_codes::SYNTAX_ERROR,
            reason,
        )
    }
}

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
pub(crate) fn is_java_digit(c: char) -> bool {
    java_decimal_digit_value(c).is_some()
}

/// Java `Character.digit(char, radix)` 的 QLexer 所需子集。
///
/// 除 Unicode 十进制数字外，Java 还接受 ASCII 与全角拉丁字母作为
/// 10..35。返回值大于等于 `radix` 时视为非法。
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
        0x0030, 0x0660, 0x06F0, 0x07C0, 0x0966, 0x09E6, 0x0A66, 0x0AE6, 0x0B66,
        0x0BE6, 0x0C66, 0x0CE6, 0x0D66, 0x0DE6, 0x0E50, 0x0ED0, 0x0F20, 0x1040,
        0x1090, 0x17E0, 0x1810, 0x1946, 0x19D0, 0x1A80, 0x1A90, 0x1B50, 0x1BB0,
        0x1C40, 0x1C50, 0xA620, 0xA8D0, 0xA900, 0xA9D0, 0xA9F0, 0xAA50, 0xABF0,
        0xFF10,
    ];
    let code = c as u32;
    STARTS
        .iter()
        .find_map(|start| (code >= *start && code < *start + 10).then_some(code - *start))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::aparser::parser_operator_manager::OpType;

    /// Default scan: Script interpolation, `${`/`}` selectors, strict
    /// newlines — mirrors `InitOptions.DEFAULT` usage.
    fn lex(script: &str) -> Vec<Token> {
        tokenize(script, None, InterpolationMode::Script, "${", "}", true)
            .expect("scan should succeed")
    }

    fn types(tokens: &[Token]) -> Vec<i32> {
        tokens.iter().map(Token::token_type).collect()
    }

    fn texts(tokens: &[Token]) -> Vec<&str> {
        tokens.iter().map(Token::text).collect()
    }

    struct AliasManager;
    impl ParserOperatorManager for AliasManager {
        fn is_op_type(&self, _lexeme: &str, _op_type: OpType) -> bool {
            true
        }
        fn precedence(&self, _lexeme: &str) -> Option<i32> {
            None
        }
        fn get_alias(&self, lexeme: &str) -> Option<i32> {
            match lexeme {
                "and" | "or" | "like" => Some(token::OPID as i32),
                _ => None,
            }
        }
    }

    #[test]
    fn empty_input_yields_only_eof() {
        let tokens = lex("");
        assert_eq!(types(&tokens), vec![token::EOF]);
        assert_eq!(tokens[0].text(), "<EOF>");
        assert_eq!(tokens[0].line(), 1);
        assert_eq!(tokens[0].char_position_in_line(), 0);
    }

    #[test]
    fn keywords_and_identifiers() {
        let tokens =
            lex("for if else while break continue return function macro import static new");
        assert_eq!(
            types(&tokens),
            vec![
                token::FOR as i32,
                token::IF as i32,
                token::ELSE as i32,
                token::WHILE as i32,
                token::BREAK as i32,
                token::CONTINUE as i32,
                token::RETURN as i32,
                token::FUNCTION as i32,
                token::MACRO as i32,
                token::IMPORT as i32,
                token::STATIC as i32,
                token::NEW as i32,
                token::EOF,
            ]
        );
        let tokens = lex("switch case default byte short int long float double char boolean");
        assert_eq!(
            types(&tokens)[..11],
            [
                token::SWITCH as i32,
                token::CASE as i32,
                token::DEFAULT as i32,
                token::BYTE as i32,
                token::SHORT as i32,
                token::INT as i32,
                token::LONG as i32,
                token::FLOAT as i32,
                token::DOUBLE as i32,
                token::CHAR as i32,
                token::BOOL as i32,
            ]
        );
        let tokens = lex("null true false extends super try catch finally throw then class this");
        assert_eq!(
            types(&tokens)[..12],
            [
                token::NULL as i32,
                token::TRUE as i32,
                token::FALSE as i32,
                token::EXTENDS as i32,
                token::SUPER as i32,
                token::TRY as i32,
                token::CATCH as i32,
                token::FINALLY as i32,
                token::THROW as i32,
                token::THEN as i32,
                token::CLASS as i32,
                token::THIS as i32,
            ]
        );
        assert_eq!(tokens[12].token_type(), token::EOF);
    }

    #[test]
    fn identifiers_with_special_start_and_unicode() {
        let tokens = lex("#a @b $c _d 变量x ef01");
        assert_eq!(
            texts(&tokens)[..6],
            ["#a", "@b", "$c", "_d", "变量x", "ef01"]
        );
        assert_eq!(types(&tokens)[..6], [token::ID as i32; 6]);
        // CJK punctuation allowed as id part.
        let tokens = lex("func（1）");
        assert_eq!(texts(&tokens)[..2], ["func（1）", "<EOF>"]);
    }

    #[test]
    fn word_operator_alias_via_manager() {
        let manager = AliasManager;
        let tokens = tokenize(
            "a and b",
            Some(&manager),
            InterpolationMode::Script,
            "${",
            "}",
            true,
        )
        .unwrap();
        assert_eq!(
            types(&tokens),
            vec![
                token::ID as i32,
                token::OPID as i32,
                token::ID as i32,
                token::EOF
            ]
        );
        assert_eq!(tokens[1].text(), "and");
    }

    #[test]
    fn number_literals() {
        let tokens = lex("1 1L 0x1F 0X2a 0b101 0B11L 1.5 .5 1. 1e3 1E-3 2e+2f 3d 4F 1_000 0xff_ff");
        let expect = vec![
            (token::INTEGER_OR_FLOATING_LITERAL, "1"),
            (token::INTEGER_OR_FLOATING_LITERAL, "1L"),
            (token::INTEGER_LITERAL, "0x1F"),
            (token::INTEGER_LITERAL, "0X2a"),
            (token::INTEGER_LITERAL, "0b101"),
            (token::INTEGER_LITERAL, "0B11L"),
            // Like Java, `1.5`/`1.` stay IntegerOrFloatingLiteral; the
            // parser refines the kind (only `.5`, exponents, and float
            // suffixes are FloatingPointLiteral at scan time).
            (token::INTEGER_OR_FLOATING_LITERAL, "1.5"),
            (token::FLOATING_POINT_LITERAL, ".5"),
            (token::INTEGER_OR_FLOATING_LITERAL, "1."),
            (token::FLOATING_POINT_LITERAL, "1e3"),
            (token::FLOATING_POINT_LITERAL, "1E-3"),
            (token::FLOATING_POINT_LITERAL, "2e+2f"),
            (token::FLOATING_POINT_LITERAL, "3d"),
            (token::FLOATING_POINT_LITERAL, "4F"),
            (token::INTEGER_OR_FLOATING_LITERAL, "1_000"),
            (token::INTEGER_LITERAL, "0xff_ff"),
        ];
        let got: Vec<(u16, &str)> = tokens[..expect.len()]
            .iter()
            .map(|t| (t.token_type() as u16, t.text()))
            .collect();
        assert_eq!(got, expect);
    }

    #[test]
    fn number_dot_before_method_not_consumed() {
        // `1.toString` -> `1` `.` `toString` (two ASCII letters after dot).
        let tokens = lex("1.toString");
        assert_eq!(
            types(&tokens),
            vec![
                token::INTEGER_OR_FLOATING_LITERAL as i32,
                token::DOT as i32,
                token::ID as i32,
                token::EOF,
            ]
        );
        // `1.x1` -> dot consumed (second char not a letter).
        let tokens = lex("1.x1");
        assert_eq!(texts(&tokens)[0], "1.");
        // `1.e` -> p+2 >= len, dot consumed.
        let tokens = lex("1.e");
        assert_eq!(texts(&tokens)[..2], ["1.", "e"]);
        // malformed exponent backtracks: `1e+` -> `1` `e` `+`
        let tokens = lex("1e+");
        assert_eq!(texts(&tokens)[..4], ["1", "e", "+", "<EOF>"]);
    }

    #[test]
    fn single_quote_string() {
        let tokens = lex("'abc' 'it\\'s' ''");
        assert_eq!(texts(&tokens)[..4], ["'abc'", "'it\\'s'", "''", "<EOF>"]);
        assert_eq!(types(&tokens)[..3], [token::QUOTE_STRING_LITERAL as i32; 3]);
    }

    #[test]
    fn double_quote_string_disable_mode() {
        let tokens = tokenize(
            "\"a ${x} b\"",
            None,
            InterpolationMode::Disable,
            "${",
            "}",
            true,
        )
        .unwrap();
        assert_eq!(texts(&tokens)[..4], ["\"", "a ${x} b", "\"", "<EOF>"]);
        assert_eq!(
            types(&tokens)[..3],
            [
                token::DOUBLE_QUOTE as i32,
                token::STATIC_STRING_CHARACTERS as i32,
                token::DOUBLE_QUOTE as i32,
            ]
        );
        // Empty string emits no text token.
        let tokens = tokenize("\"\"", None, InterpolationMode::Disable, "${", "}", true).unwrap();
        assert_eq!(
            types(&tokens),
            vec![
                token::DOUBLE_QUOTE as i32,
                token::DOUBLE_QUOTE as i32,
                token::EOF,
            ]
        );
    }

    #[test]
    fn double_quote_string_script_interpolation() {
        let tokens = lex("\"a ${x + {1}} b\"");
        assert_eq!(
            types(&tokens),
            vec![
                token::DOUBLE_QUOTE as i32,
                token::DY_STR_TEXT as i32,
                token::DY_STR_EXPR_START as i32,
                token::ID as i32,                          // x
                token::ADD as i32,                         // +
                token::LBRACE as i32,                      // {
                token::INTEGER_OR_FLOATING_LITERAL as i32, // 1
                token::RBRACE as i32,                      // }
                token::RBRACE as i32,                      // } closing interpolation
                token::DY_STR_TEXT as i32,
                token::DOUBLE_QUOTE as i32,
                token::EOF,
            ]
        );
        assert_eq!(texts(&tokens)[1], "a ");
        assert_eq!(texts(&tokens)[9], " b");
    }

    #[test]
    fn double_quote_string_variable_interpolation() {
        let tokens = tokenize(
            "\"hello ${name}!\"",
            None,
            InterpolationMode::Variable,
            "${",
            "}",
            true,
        )
        .unwrap();
        assert_eq!(
            types(&tokens),
            vec![
                token::DOUBLE_QUOTE as i32,
                token::DY_STR_TEXT as i32,
                token::DY_STR_EXPR_START as i32,
                token::SELECTOR_VARIABLE_VANME as i32,
                token::DY_STR_TEXT as i32,
                token::DOUBLE_QUOTE as i32,
                token::EOF,
            ]
        );
        assert_eq!(tokens[3].text(), "name");
    }

    #[test]
    fn selector_outside_string() {
        let tokens = lex("${user.name} + 1");
        assert_eq!(
            types(&tokens)[..5],
            [
                token::SELECTOR_START as i32,
                token::SELECTOR_VARIABLE_VANME as i32,
                token::ADD as i32,
                token::INTEGER_OR_FLOATING_LITERAL as i32,
                token::EOF,
            ]
        );
        assert_eq!(tokens[1].text(), "user.name");
        // custom selector delimiters
        let tokens = tokenize("$[a]", None, InterpolationMode::Script, "$[", "]", true).unwrap();
        assert_eq!(texts(&tokens)[..3], ["$[", "a", "<EOF>"]);
    }

    #[test]
    fn operator_longest_match() {
        let tokens =
            lex(">>>= >>> >>= <<= -> :: <> >> << >= <= ?. *. .* += -= &= |= *= %= /= ^= ++ --");
        assert_eq!(
            types(&tokens)[..24],
            [
                token::URSHIFT_ASSGIN as i32,
                token::URSHIFT as i32,
                token::RIGHSHIFT_ASSGIN as i32,
                token::LSHIFT_ASSGIN as i32,
                token::ARROW as i32,
                token::DCOLON as i32,
                token::NOEQ as i32,
                token::RIGHSHIFT as i32,
                token::LEFTSHIFT as i32,
                token::GE as i32,
                token::LE as i32,
                token::OPTIONAL_CHAINING as i32,
                token::SPREAD_CHAINING as i32,
                token::DOTMUL as i32,
                token::ADD_ASSIGN as i32,
                token::SUB_ASSIGN as i32,
                token::AND_ASSIGN as i32,
                token::OR_ASSIGN as i32,
                token::MUL_ASSIGN as i32,
                token::MOD_ASSIGN as i32,
                token::DIV_ASSIGN as i32,
                token::XOR_ASSIGN as i32,
                token::INC as i32,
                token::DEC as i32,
            ]
        );
    }

    #[test]
    fn opid_operators_and_custom() {
        let tokens = lex("== != && || ** %% ~^");
        assert_eq!(
            texts(&tokens)[..8],
            ["==", "!=", "&&", "||", "**", "%%", "~^", "<EOF>"]
        );
        assert_eq!(types(&tokens)[..7], [token::OPID as i32; 7]);
        // `<` is not a custom-operator start, so `<=>` splits as `<=` + `>`.
        let tokens = lex("<=>");
        assert_eq!(
            types(&tokens),
            vec![token::LE as i32, token::GT as i32, token::EOF]
        );
    }

    #[test]
    fn single_char_punctuation_and_operators() {
        let tokens = lex("( ) { } [ ] . ; , ? : > < = ^ ! ~ + - * / & | %");
        assert_eq!(
            types(&tokens)[..24],
            [
                token::LPAREN as i32,
                token::RPAREN as i32,
                token::LBRACE as i32,
                token::RBRACE as i32,
                token::LBRACK as i32,
                token::RBRACK as i32,
                token::DOT as i32,
                token::SEMI as i32,
                token::COMMA as i32,
                token::QUESTION as i32,
                token::COLON as i32,
                token::GT as i32,
                token::LT as i32,
                token::EQ as i32,
                token::CARET as i32,
                token::BANG as i32,
                token::TILDE as i32,
                token::ADD as i32,
                token::SUB as i32,
                token::MUL as i32,
                token::DIV as i32,
                token::BIT_AND as i32,
                token::BIT_OR as i32,
                token::MOD as i32,
            ]
        );
    }

    #[test]
    fn unknown_char_is_catch_all() {
        let tokens = lex("a ` b");
        assert_eq!(
            types(&tokens),
            vec![
                token::ID as i32,
                token::CATCH_ALL as i32,
                token::ID as i32,
                token::EOF,
            ]
        );
        assert_eq!(tokens[1].text(), "`");
    }

    #[test]
    fn comments_are_skipped() {
        let tokens = lex("1 // hello\n/* multi\nline */ 2");
        assert_eq!(texts(&tokens)[..4], ["1", "\n", "2", "<EOF>"]);
        assert_eq!(
            types(&tokens),
            vec![
                token::INTEGER_OR_FLOATING_LITERAL as i32,
                token::NEWLINE as i32,
                token::INTEGER_OR_FLOATING_LITERAL as i32,
                token::EOF,
            ]
        );
    }

    #[test]
    fn newlines_strict_vs_relaxed() {
        let tokens = lex("a\nb\r\nc\rd");
        assert_eq!(
            types(&tokens),
            vec![
                token::ID as i32,
                token::NEWLINE as i32,
                token::ID as i32,
                token::NEWLINE as i32,
                token::ID as i32,
                token::NEWLINE as i32,
                token::ID as i32,
                token::EOF,
            ]
        );
        // \r\n is a single NEWLINE token.
        assert_eq!(tokens[3].text(), "\r\n");
        assert_eq!(tokens[4].line(), 3);
        let tokens = tokenize("a\nb", None, InterpolationMode::Script, "${", "}", false).unwrap();
        assert_eq!(
            types(&tokens),
            vec![token::ID as i32, token::ID as i32, token::EOF]
        );
    }

    #[test]
    fn token_positions_are_tracked() {
        let tokens = lex("ab 12\ncd");
        assert_eq!(
            (
                tokens[0].line(),
                tokens[0].char_position_in_line(),
                tokens[0].start_index(),
                tokens[0].stop_index()
            ),
            (1, 0, 0, 1)
        );
        assert_eq!(
            (tokens[1].line(), tokens[1].char_position_in_line()),
            (1, 3)
        );
        assert_eq!(
            (tokens[2].line(), tokens[2].char_position_in_line()),
            (1, 5)
        );
        assert_eq!(
            (tokens[3].line(), tokens[3].char_position_in_line()),
            (2, 0)
        );
    }

    #[test]
    fn unterminated_single_quote_string_reports_position() {
        let err = tokenize("a = 'abc", None, InterpolationMode::Script, "${", "}", true)
            .unwrap_err()
            .into_exception();
        assert!(err.is_syntax());
        assert_eq!(err.error_code(), error_codes::SYNTAX_ERROR);
        assert_eq!(err.reason(), "unterminated string literal");
        assert_eq!(err.line_no(), 1);
        assert_eq!(err.col_no(), 5);
        assert_eq!(err.err_lexeme(), "'abc");
    }

    #[test]
    fn unterminated_double_quote_string() {
        let err = tokenize("\"abc", None, InterpolationMode::Script, "${", "}", true)
            .unwrap_err()
            .into_exception();
        assert_eq!(err.reason(), "unterminated string literal");
        assert_eq!(err.line_no(), 1);
        assert_eq!(err.col_no(), 5);
    }

    #[test]
    fn unterminated_interpolation_reports_mismatched_eof() {
        let err = tokenize(
            "\"a ${x + 1",
            None,
            InterpolationMode::Script,
            "${",
            "}",
            true,
        )
        .unwrap_err()
        .into_exception();
        assert_eq!(err.reason(), "mismatched input '<EOF>' expecting '}'");
    }

    #[test]
    fn unterminated_block_comment_reports_start_line() {
        let err = tokenize(
            "1\n/* oops\n2",
            None,
            InterpolationMode::Script,
            "${",
            "}",
            true,
        )
        .unwrap_err()
        .into_exception();
        assert_eq!(err.reason(), "unterminated comment");
        assert_eq!(err.line_no(), 2);
        assert_eq!(err.col_no(), 1);
        assert_eq!(err.err_lexeme(), "/*");
    }

    #[test]
    fn unterminated_selector() {
        let err = tokenize("${abc\n", None, InterpolationMode::Script, "${", "}", true)
            .unwrap_err()
            .into_exception();
        assert_eq!(err.reason(), "unterminated selector");
        assert_eq!(err.line_no(), 1);
        let err = tokenize("${abc", None, InterpolationMode::Script, "${", "}", true)
            .unwrap_err()
            .into_exception();
        assert_eq!(err.reason(), "unterminated selector");
    }

    #[test]
    fn variable_mode_unterminated_selector() {
        let err = tokenize(
            "\"a ${x\n\"",
            None,
            InterpolationMode::Variable,
            "${",
            "}",
            true,
        )
        .unwrap_err()
        .into_exception();
        assert_eq!(err.reason(), "unterminated selector");
    }

    #[test]
    fn crlf_counts_as_one_line_advance() {
        // Mirrors Java advance(): \r\n inside strings is a single newline.
        let tokens = lex("'a\r\nb' c");
        assert_eq!(tokens[0].text(), "'a\r\nb'");
        assert_eq!(tokens[1].line(), 2);
    }

    /// SOURCE_PARITY: Java `String` 与 LSP 均使用 UTF-16 code-unit
    /// 位置；非 BMP 字符占两个 offset/column。
    #[test]
    fn token_positions_use_java_utf16_units() {
        let tokens = lex("\"😀\" x");
        assert_eq!(tokens[0].text(), "\"");
        assert_eq!(
            (tokens[1].start_index(), tokens[1].stop_index()),
            (1, 2)
        );
        assert_eq!(
            (tokens[2].start_index(), tokens[2].char_position_in_line()),
            (3, 3)
        );
        assert_eq!(
            (tokens[3].start_index(), tokens[3].char_position_in_line()),
            (5, 5)
        );
    }

    /// SOURCE_PARITY: Java `Character.isLetter(char)` 不会把 surrogate pair
    /// 组合成补充平面字母；两个 UTF-16 单元分别成为 CATCH_ALL。
    #[test]
    fn supplementary_letter_is_not_a_java_identifier() {
        let tokens = lex("𐐀");
        assert_eq!(
            types(&tokens),
            vec![
                token::CATCH_ALL as i32,
                token::CATCH_ALL as i32,
                token::EOF
            ]
        );
        assert_eq!((tokens[0].start_index(), tokens[0].stop_index()), (0, 0));
        assert_eq!((tokens[1].start_index(), tokens[1].stop_index()), (1, 1));
        assert_eq!(tokens[2].start_index(), 2);
    }

    /// SOURCE_PARITY: Java `Character.isDigit` 与 `Character.digit` 接受
    /// BMP Unicode 十进制数字及全角十六进制字母。
    #[test]
    fn unicode_java_digits_are_scanned_as_numbers() {
        let tokens = lex("١٢ 0xＦＦ");
        assert_eq!(tokens[0].token_type(), token::INTEGER_OR_FLOATING_LITERAL as i32);
        assert_eq!(tokens[0].text(), "١٢");
        assert_eq!(tokens[1].token_type(), token::INTEGER_LITERAL as i32);
        assert_eq!(tokens[1].text(), "0xＦＦ");
    }
}
