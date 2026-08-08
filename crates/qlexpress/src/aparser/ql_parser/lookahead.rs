impl<'a> QLParser<'a> {
    fn consume_next_statement(&mut self) -> PResult<()> {
        if self.la(EOF) || self.la(RBRACE) {
            return Ok(());
        }
        if self.la(SEMI) {
            self.consume();
            return Ok(());
        }
        if self.strict_new_lines {
            self.expect(NEWLINE, "NEWLINE")?;
            while self.la(NEWLINE) {
                self.consume();
            }
        } else if self.la(NEWLINE) {
            while self.la(NEWLINE) {
                self.consume();
            }
        }
        Ok(())
    }

    fn is_next_statement_start(&self) -> bool {
        self.la(EOF)
            || self.la(RBRACE)
            || self.la(SEMI)
            || (self.strict_new_lines && self.la(NEWLINE))
    }

    fn skip_newlines(&mut self) {
        while self.la(NEWLINE) {
            self.consume();
        }
    }

    fn is_lambda_start(&self) -> bool {
        if is_var_id_token(self.lt().token_type()) && self.la_at(1, ARROW) {
            return true;
        }
        if !self.la(LPAREN) {
            return false;
        }
        let close = self.find_matching_paren(self.p);
        close >= 0 && self.token(close as usize + 1).token_type() == ARROW
    }

    fn find_matching_paren(&self, start: usize) -> i64 {
        let mut depth = 0i64;
        for (i, tok) in self.tokens.iter().enumerate().skip(start) {
            let ty = tok.token_type();
            if ty == LPAREN {
                depth += 1;
            } else if ty == RPAREN {
                depth -= 1;
                if depth == 0 {
                    return i as i64;
                }
            }
        }
        -1
    }

    fn is_primary_start(&self, tok: &Token) -> bool {
        let ty = tok.token_type();
        is_literal_start(ty)
            || ty == LPAREN
            || ty == NEW
            || is_var_id_token(ty)
            || is_primitive_type(ty)
            || ty == LBRACK
            || ty == LBRACE
            || ty == SELECTOR_START
            || ty == IF
            || ty == SWITCH
            || ty == TRY
            || self.is_prefix_operator(tok)
    }

    fn is_non_expression_statement_start(&mut self) -> PResult<bool> {
        if self.la(THROW)
            || self.la(WHILE)
            || self.la(FOR)
            || self.la(FUNCTION)
            || self.la(MACRO)
            || self.la(BREAK)
            || self.la(CONTINUE)
            || self.la(RETURN)
            || self.la(SEMI)
            || self.la(NEWLINE)
        {
            return Ok(true);
        }
        Ok(self.try_parse_local_variable_declaration()?.is_some())
    }

    fn single_var_before(&self, end_types: &[i32]) -> bool {
        if !is_var_id_token(self.lt().token_type()) {
            return false;
        }
        end_types.iter().any(|&end| self.la_at(1, end))
    }

    fn single_for_each_var_before_colon(&self) -> bool {
        if !is_var_id_token(self.lt().token_type()) {
            return false;
        }
        let mut i = self.p + 1;
        while self.token(i).token_type() == NEWLINE {
            i += 1;
        }
        self.token(i).token_type() == COLON
    }

    /// Java `isMapExprAhead`: speculative, catches all errors.
    fn is_map_expr_ahead(&mut self) -> bool {
        let save = self.p;
        let result = (|| -> PResult<bool> {
            self.expect(LBRACE, "'{'")?;
            self.skip_newlines();
            if self.la(COLON) {
                return Ok(true);
            }
            if !is_id_map_key(self.lt().token_type())
                && !self.la(QUOTE_STRING_LITERAL)
                && !self.la(DOUBLE_QUOTE)
            {
                return Ok(false);
            }
            if self.la(DOUBLE_QUOTE) {
                self.parse_double_quote_string_literal()?;
            } else {
                self.consume();
            }
            self.skip_newlines();
            Ok(self.la(COLON))
        })();
        self.p = save;
        matches!(result, Ok(true))
    }

    fn has_top_level_assign_operator_ahead(&self) -> bool {
        let mut depth = 0i32;
        for i in self.p..self.tokens.len() {
            let ty = self.tokens[i].token_type();
            if depth == 0 {
                if is_assign_operator(ty) {
                    return true;
                }
                if ty == EOF
                    || ty == SEMI
                    || ty == COMMA
                    || ty == RPAREN
                    || ty == RBRACK
                    || ty == RBRACE
                    || (self.strict_new_lines && ty == NEWLINE)
                {
                    return false;
                }
            }
            if ty == LPAREN || ty == LBRACK || ty == LBRACE {
                depth += 1;
            } else if ty == RPAREN || ty == RBRACK || ty == RBRACE {
                if depth == 0 {
                    return false;
                }
                depth -= 1;
            }
        }
        false
    }

    // ------------------------------------------------------------------
    // Operator manager queries (Java isMiddleOperator etc.)
    // ------------------------------------------------------------------

    fn is_middle_operator(&self, tok: &Token) -> bool {
        self.operator_manager
            .map(|m| m.is_op_type(tok.text(), OpType::Middle))
            .unwrap_or(false)
    }

    fn is_prefix_operator(&self, tok: &Token) -> bool {
        self.operator_manager
            .map(|m| m.is_op_type(tok.text(), OpType::Prefix))
            .unwrap_or(false)
    }

    fn is_suffix_operator(&self, tok: &Token) -> bool {
        self.operator_manager
            .map(|m| m.is_op_type(tok.text(), OpType::Suffix))
            .unwrap_or(false)
    }

    fn is_group_operator(&self, tok: &Token) -> bool {
        is_op_id_token(tok.token_type())
            && self.is_middle_operator(tok)
            && self.precedence(tok) == ql_precedences::GROUP
    }

    fn precedence(&self, tok: &Token) -> i32 {
        self.operator_manager
            .and_then(|m| m.precedence(tok.text()))
            .unwrap_or(-1)
    }
}
