impl<'a> QLexer<'a> {
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
}
