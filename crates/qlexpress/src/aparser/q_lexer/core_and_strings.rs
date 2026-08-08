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
}
