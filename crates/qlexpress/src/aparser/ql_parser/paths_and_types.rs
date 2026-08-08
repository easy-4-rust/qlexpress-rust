impl<'a> QLParser<'a> {
    fn try_parse_path_part(&mut self) -> PResult<Option<Node>> {
        if self.la(DOT) {
            let dot = self.consume_node();
            if is_var_id_token(self.lt().token_type()) && self.la_at(1, LPAREN) {
                let var_id = self.parse_var_id()?;
                let mut ctx = MethodInvokeContext {
                    dot,
                    var_id: Box::new(var_id),
                    lparen: TerminalNode::new(self.lt().clone()),
                    argument_list: None,
                    rparen: TerminalNode::new(self.lt().clone()),
                    chain: ChainKind::Plain,
                };
                self.parse_method_arguments(&mut ctx)?;
                return Ok(Some(Node::MethodInvoke(ctx)));
            }
            let field_id = self.parse_field_id()?;
            return Ok(Some(Node::FieldAccess(FieldAccessContext {
                dot,
                field_id: Box::new(field_id),
                chain: ChainKind::Plain,
            })));
        }
        if self.la(OPTIONAL_CHAINING) || self.la(SPREAD_CHAINING) {
            let optional = self.la(OPTIONAL_CHAINING);
            let chain = if optional {
                ChainKind::Optional
            } else {
                ChainKind::Spread
            };
            let token = self.consume_node();
            if is_var_id_token(self.lt().token_type()) && self.la_at(1, LPAREN) {
                let var_id = self.parse_var_id()?;
                let mut ctx = MethodInvokeContext {
                    dot: token,
                    var_id: Box::new(var_id),
                    lparen: TerminalNode::new(self.lt().clone()),
                    argument_list: None,
                    rparen: TerminalNode::new(self.lt().clone()),
                    chain,
                };
                self.parse_method_arguments(&mut ctx)?;
                return Ok(Some(Node::MethodInvoke(ctx)));
            }
            let field_id = self.parse_field_id()?;
            return Ok(Some(Node::FieldAccess(FieldAccessContext {
                dot: token,
                field_id: Box::new(field_id),
                chain,
            })));
        }
        if self.la(DCOLON) {
            let dcolon = self.consume_node();
            let var_id = self.parse_var_id()?;
            return Ok(Some(Node::MethodAccess(MethodAccessContext {
                dcolon,
                var_id: Box::new(var_id),
            })));
        }
        if self.la(LBRACK) {
            let lbrack = self.expect(LBRACK, "'['")?;
            self.skip_newlines();
            let index_value_expr = if !self.la(RBRACK) {
                Some(Box::new(self.parse_index_value_expr()?))
            } else {
                None
            };
            self.skip_newlines();
            let rbrack = self.expect(RBRACK, "']'")?;
            return Ok(Some(Node::IndexExpr(IndexExprContext {
                lbrack,
                index_value_expr,
                rbrack,
            })));
        }
        if self.is_group_operator(self.lt()) {
            let op_id = self.parse_op_id()?;
            self.skip_newlines();
            let mut ctx = CustomPathContext {
                op_id: Box::new(op_id),
                var_id: None,
                quote: None,
                path_text: String::new(),
            };
            if is_var_id_token(self.lt().token_type()) {
                let var_id = self.parse_var_id()?;
                ctx.path_text = var_id.text();
                ctx.var_id = Some(Box::new(var_id));
            } else if self.la(QUOTE_STRING_LITERAL) {
                let quote = self.consume_node();
                let text = quote.text();
                ctx.path_text = text[1..text.len() - 1].to_string();
                ctx.quote = Some(quote);
            } else {
                let tok = self.lt();
                return Err(self.syntax(
                    tok,
                    format!(
                        "mismatched input '{}' expecting custom path",
                        token_text(tok)
                    ),
                ));
            }
            return Ok(Some(Node::CustomPath(ctx)));
        }
        Ok(None)
    }

    fn parse_method_arguments(&mut self, ctx: &mut MethodInvokeContext) -> PResult<()> {
        ctx.lparen = self.expect(LPAREN, "'('")?;
        self.skip_newlines();
        if !self.la(RPAREN) {
            ctx.argument_list = Some(Box::new(self.parse_argument_list()?));
        }
        self.skip_newlines();
        ctx.rparen = self.expect(RPAREN, "')'")?;
        Ok(())
    }

    fn parse_field_id(&mut self) -> PResult<Node> {
        if is_var_id_token(self.lt().token_type()) || self.la(CLASS) {
            return Ok(Node::FieldId(FieldIdContext {
                token: Some(self.consume_node()),
                quote: None,
            }));
        }
        if self.la(QUOTE_STRING_LITERAL) {
            return Ok(Node::FieldId(FieldIdContext {
                token: None,
                quote: Some(self.consume_node()),
            }));
        }
        let tok = self.lt();
        Err(self.syntax(
            tok,
            format!("mismatched input '{}' expecting field", token_text(tok)),
        ))
    }

    fn parse_index_value_expr(&mut self) -> PResult<Node> {
        if self.la(COLON) {
            let colon = self.consume_node();
            self.skip_newlines();
            let end = if !self.la(RBRACK) {
                Some(Box::new(self.parse_expression()?))
            } else {
                None
            };
            return Ok(Node::SliceIndex(SliceIndexContext {
                start: None,
                colon,
                end,
            }));
        }
        let first = self.parse_expression()?;
        self.skip_newlines();
        if self.la(COLON) {
            let colon = self.consume_node();
            self.skip_newlines();
            let end = if !self.la(RBRACK) {
                Some(Box::new(self.parse_expression()?))
            } else {
                None
            };
            return Ok(Node::SliceIndex(SliceIndexContext {
                start: Some(Box::new(first)),
                colon,
                end,
            }));
        }
        Ok(Node::SingleIndex(SingleIndexContext {
            expression: Box::new(first),
        }))
    }

    fn parse_argument_list(&mut self) -> PResult<Node> {
        let mut ctx = ArgumentListContext {
            expressions: Vec::new(),
            commas: Vec::new(),
        };
        ctx.expressions.push(self.parse_expression()?);
        self.skip_newlines();
        while self.la(COMMA) {
            ctx.commas.push(self.consume_node());
            self.skip_newlines();
            ctx.expressions.push(self.parse_expression()?);
            self.skip_newlines();
        }
        Ok(Node::ArgumentList(ctx))
    }

    fn parse_literal(&mut self) -> PResult<Node> {
        let ty = self.lt().token_type();
        if ty == INTEGER_LITERAL
            || ty == FLOATING_POINT_LITERAL
            || ty == INTEGER_OR_FLOATING_LITERAL
            || ty == QUOTE_STRING_LITERAL
            || ty == NULL
        {
            return Ok(Node::Literal(LiteralContext {
                token: Some(self.consume_node()),
                boolen: None,
                double_quote_string: None,
            }));
        }
        if ty == TRUE || ty == FALSE {
            return Ok(Node::Literal(LiteralContext {
                token: None,
                boolen: Some(Box::new(Node::BoolenLiteral(BoolenLiteralContext {
                    token: self.consume_node(),
                }))),
                double_quote_string: None,
            }));
        }
        if self.la(DOUBLE_QUOTE) {
            let double_quote_string = self.parse_double_quote_string_literal()?;
            return Ok(Node::Literal(LiteralContext {
                token: None,
                boolen: None,
                double_quote_string: Some(Box::new(double_quote_string)),
            }));
        }
        let tok = self.lt();
        Err(self.syntax(
            tok,
            format!("mismatched input '{}' expecting literal", token_text(tok)),
        ))
    }

    fn parse_double_quote_string_literal(&mut self) -> PResult<Node> {
        let open_quote = self.expect(DOUBLE_QUOTE, "'\"'")?;
        let mut ctx = DoubleQuoteStringLiteralContext {
            open_quote,
            static_characters: None,
            parts: Vec::new(),
            close_quote: TerminalNode::new(self.lt().clone()),
        };
        if self.la(STATIC_STRING_CHARACTERS) {
            ctx.static_characters = Some(self.consume_node());
        }
        while !self.la(DOUBLE_QUOTE) && !self.la(EOF) {
            if self.la(DY_STR_TEXT) {
                ctx.parts.push(DyStrPart::Text(self.consume_node()));
            } else if self.la(DY_STR_EXPR_START) {
                let start = self.consume_node();
                let expr = if self.la(SELECTOR_VARIABLE_VANME) {
                    StringExpressionContext {
                        start,
                        selector_variable: Some(self.consume_node()),
                        expression: None,
                        rbrace: None,
                    }
                } else {
                    self.skip_newlines();
                    let expression = self.parse_expression()?;
                    self.skip_newlines();
                    let rbrace = self.expect(RBRACE, "'}'")?;
                    StringExpressionContext {
                        start,
                        selector_variable: None,
                        expression: Some(Box::new(expression)),
                        rbrace: Some(rbrace),
                    }
                };
                ctx.parts
                    .push(DyStrPart::Expr(Box::new(Node::StringExpression(expr))));
            } else {
                let tok = self.lt();
                return Err(self.syntax(
                    tok,
                    format!(
                        "mismatched input '{}' expecting string content",
                        token_text(tok)
                    ),
                ));
            }
        }
        ctx.close_quote = self.expect(DOUBLE_QUOTE, "'\"'")?;
        Ok(Node::DoubleQuoteStringLiteral(ctx))
    }

    fn parse_import_declaration(&mut self) -> PResult<Node> {
        let import_token = self.expect(IMPORT, "'import'")?;
        let mut ids = vec![self.parse_var_id()?];
        while self.la(DOT) && !self.la_at(1, MUL) && !self.la_at(1, EOF) {
            self.consume();
            ids.push(self.parse_var_id()?);
        }
        let is_pack = self.la(DOTMUL) || (self.la(DOT) && self.la_at(1, MUL));
        if is_pack {
            if self.la(DOTMUL) {
                self.consume();
            } else {
                self.consume();
                self.consume();
            }
        }
        let semi = self.expect(SEMI, "';'")?;
        if is_pack {
            Ok(Node::ImportPack(ImportPackContext {
                import_token,
                var_ids: ids,
                semi,
            }))
        } else {
            Ok(Node::ImportCls(ImportClsContext {
                import_token,
                var_ids: ids,
                semi,
            }))
        }
    }

    // ------------------------------------------------------------------
    // Types
    // ------------------------------------------------------------------

    /// Java `parseDeclType` (throws `QLParseBacktrack` on non-type start).
    fn parse_decl_type(&mut self) -> PResult<Node> {
        let mut ctx = DeclTypeContext {
            primitive_type: None,
            cls_type: None,
            dims: None,
        };
        if is_primitive_type(self.lt().token_type()) {
            ctx.primitive_type = Some(Box::new(self.parse_primitive_type()?));
        } else if is_var_id_token(self.lt().token_type()) {
            ctx.cls_type = Some(Box::new(self.parse_cls_type()?));
        } else {
            return Self::backtrack();
        }
        if self.la(LBRACK) && self.la_at(1, RBRACK) {
            ctx.dims = Some(Box::new(self.parse_dims()?));
        }
        Ok(Node::DeclType(ctx))
    }

    fn parse_decl_type_no_arr(&mut self) -> PResult<Node> {
        let mut ctx = DeclTypeNoArrContext {
            primitive_type: None,
            cls_type: None,
        };
        if is_primitive_type(self.lt().token_type()) {
            ctx.primitive_type = Some(Box::new(self.parse_primitive_type()?));
        } else if is_var_id_token(self.lt().token_type()) {
            ctx.cls_type = Some(Box::new(self.parse_cls_type()?));
        } else {
            let tok = self.lt();
            return Err(self.syntax(
                tok,
                format!("mismatched input '{}' expecting type", token_text(tok)),
            ));
        }
        Ok(Node::DeclTypeNoArr(ctx))
    }

    fn parse_primitive_type(&mut self) -> PResult<Node> {
        if !is_primitive_type(self.lt().token_type()) {
            return Self::backtrack();
        }
        Ok(Node::PrimitiveType(PrimitiveTypeContext {
            token: self.consume_node(),
        }))
    }

    fn parse_cls_type(&mut self) -> PResult<Node> {
        let mut var_ids = vec![self.parse_var_id()?];
        while self.la(DOT) && is_var_id_token(self.token(self.p + 1).token_type()) {
            self.consume();
            var_ids.push(self.parse_var_id()?);
        }
        if self.la(LT) || self.la(NOEQ) {
            self.parse_type_arguments()?;
        }
        Ok(Node::ClsType(ClsTypeContext { var_ids }))
    }

    fn parse_type_arguments(&mut self) -> PResult<()> {
        if self.la(NOEQ) {
            self.consume();
            return Ok(());
        }
        self.expect(LT, "'<'")?;
        self.skip_newlines();
        if !self.is_type_argument_end() && !self.la(EOF) {
            self.parse_type_argument_list()?;
            self.skip_newlines();
        }
        if self.is_type_argument_end() {
            self.consume();
        }
        Ok(())
    }

    fn parse_type_argument_list(&mut self) -> PResult<()> {
        self.parse_type_argument()?;
        self.skip_newlines();
        while self.la(COMMA) {
            self.consume();
            self.skip_newlines();
            self.parse_type_argument()?;
            self.skip_newlines();
        }
        Ok(())
    }

    fn parse_type_argument(&mut self) -> PResult<()> {
        if self.la(QUESTION) {
            self.consume();
            self.skip_newlines();
            if self.la(EXTENDS) || self.la(SUPER) {
                self.consume();
                self.skip_newlines();
                self.parse_reference_type()?;
            }
            return Ok(());
        }
        self.parse_reference_type()
    }

    fn parse_reference_type(&mut self) -> PResult<()> {
        if is_var_id_token(self.lt().token_type()) {
            self.parse_cls_type()?;
            if self.la(LBRACK) && self.la_at(1, RBRACK) {
                self.parse_dims()?;
            }
            return Ok(());
        }
        if is_primitive_type(self.lt().token_type()) {
            self.parse_primitive_type()?;
            if self.la(LBRACK) && self.la_at(1, RBRACK) {
                self.parse_dims()?;
                return Ok(());
            }
        }
        let tok = self.lt();
        Err(self.syntax(
            tok,
            format!(
                "mismatched input '{}' expecting reference type",
                token_text(tok)
            ),
        ))
    }

    fn is_type_argument_end(&self) -> bool {
        self.la(GT) || self.la(RIGHSHIFT) || self.la(URSHIFT)
    }

    fn parse_dims(&mut self) -> PResult<Node> {
        let mut brackets = Vec::new();
        loop {
            brackets.push(self.expect(LBRACK, "'['")?);
            brackets.push(self.expect(RBRACK, "']'")?);
            if !(self.la(LBRACK) && self.la_at(1, RBRACK)) {
                break;
            }
        }
        Ok(Node::Dims(DimsContext { brackets }))
    }

    fn parse_dim_exprs(&mut self) -> PResult<Node> {
        let mut expressions = Vec::new();
        let mut brackets = Vec::new();
        loop {
            brackets.push(self.expect(LBRACK, "'['")?);
            self.skip_newlines();
            expressions.push(self.parse_expression()?);
            self.skip_newlines();
            brackets.push(self.expect(RBRACK, "']'")?);
            if !(self.la(LBRACK) && !self.la_at(1, RBRACK)) {
                break;
            }
        }
        Ok(Node::DimExprs(DimExprsContext {
            expressions,
            brackets,
        }))
    }

    fn parse_op_id(&mut self) -> PResult<Node> {
        if !is_op_id_token(self.lt().token_type()) {
            let tok = self.lt();
            return Err(self.syntax(
                tok,
                format!("mismatched input '{}' expecting operator", token_text(tok)),
            ));
        }
        Ok(Node::OpId(OpIdContext {
            token: self.consume_node(),
        }))
    }

    fn parse_var_id(&mut self) -> PResult<Node> {
        if !is_var_id_token(self.lt().token_type()) {
            let tok = self.lt();
            return Err(self.syntax(
                tok,
                format!(
                    "mismatched input '{}' expecting identifier",
                    token_text(tok)
                ),
            ));
        }
        Ok(Node::VarId(VarIdContext {
            token: self.consume_node(),
        }))
    }
}

// ---------------------------------------------------------------------------
// Newline handling, lookaheads and token classification (Java private helpers)
// ---------------------------------------------------------------------------

