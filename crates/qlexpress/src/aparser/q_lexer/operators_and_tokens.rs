impl<'a> QLexer<'a> {
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
