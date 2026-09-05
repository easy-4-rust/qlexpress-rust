impl<'a> QLParser<'a> {
    fn parse_expression(&mut self) -> PResult<Node> {
        let _guard = DepthGuard::enter(self.script, self.lt())?;
        let save = self.p;
        if self.has_top_level_assign_operator_ahead() {
            let left = self.try_parse_left_hand_side()?;
            if left.is_some() && is_assign_operator(self.lt().token_type()) {
                let assign_operator = self.parse_assign_operator()?;
                self.skip_newlines();
                let expression = self.parse_expression()?;
                return Ok(Node::Expression(ExpressionContext {
                    left: left.map(Box::new),
                    assign_operator: Some(Box::new(assign_operator)),
                    expression: Some(Box::new(expression)),
                    ternary: None,
                }));
            }
            self.p = save;
        }
        let ternary = self.parse_ternary_expr()?;
        Ok(Node::Expression(ExpressionContext {
            left: None,
            assign_operator: None,
            expression: None,
            ternary: Some(Box::new(ternary)),
        }))
    }

    fn try_parse_left_hand_side(&mut self) -> PResult<Option<Node>> {
        if !is_var_id_token(self.lt().token_type()) {
            return Ok(None);
        }
        let var_id = self.parse_var_id()?;
        let mut ctx = LeftHandSideContext {
            var_id: Box::new(var_id),
            lparen: None,
            argument_list: None,
            rparen: None,
            path_parts: Vec::new(),
        };
        if self.la(LPAREN) {
            ctx.lparen = Some(self.consume_node());
            self.skip_newlines();
            if !self.la(RPAREN) {
                ctx.argument_list = Some(Box::new(self.parse_argument_list()?));
            }
            self.skip_newlines();
            ctx.rparen = Some(self.expect(RPAREN, "')'")?);
        }
        loop {
            let before_newlines = self.p;
            self.skip_newlines();
            match self.try_parse_path_part()? {
                None => {
                    self.p = before_newlines;
                    break;
                }
                Some(part) => ctx.path_parts.push(part),
            }
        }
        Ok(Some(Node::LeftHandSide(ctx)))
    }

    fn parse_assign_operator(&mut self) -> PResult<Node> {
        if !is_assign_operator(self.lt().token_type()) {
            let tok = self.lt();
            return Err(self.syntax(
                tok,
                format!(
                    "mismatched input '{}' expecting assignment operator",
                    token_text(tok)
                ),
            ));
        }
        Ok(Node::AssignOperator(AssignOperatorContext {
            token: self.consume_node(),
        }))
    }

    fn parse_ternary_expr(&mut self) -> PResult<Node> {
        let condition = self.parse_base_expr(1)?;
        let mut ctx = TernaryExprContext {
            condition: Box::new(condition),
            question: None,
            then_expr: None,
            colon: None,
            else_expr: None,
        };
        if self.la(QUESTION) {
            ctx.question = Some(self.consume_node());
            self.skip_newlines();
            ctx.then_expr = Some(Box::new(self.parse_base_expr(0)?));
            ctx.colon = Some(self.expect(COLON, "':'")?);
            self.skip_newlines();
            ctx.else_expr = Some(Box::new(self.parse_expression()?));
        }
        Ok(Node::TernaryExpr(ctx))
    }

    fn parse_base_expr(&mut self, min_precedence: i32) -> PResult<Node> {
        let primary = self.parse_primary()?;
        let mut ctx = BaseExprContext {
            primary: Box::new(primary),
            left_assos: Vec::new(),
        };
        while !self.la(EOF)
            && (!self.strict_new_lines || !self.la(NEWLINE))
            && self.is_middle_operator(self.lt())
            && self.precedence(self.lt()) >= min_precedence
        {
            let binaryop = self.parse_binaryop()?;
            let is_instanceof = binaryop.text() == "instanceof";
            let next_min = {
                match &binaryop {
                    Node::Binaryop(op) => self.precedence(op.token.symbol()) + 1,
                    _ => unreachable!("parse_binaryop returns Binaryop"),
                }
            };
            self.skip_newlines();
            let right = if is_instanceof {
                let decl_type = self.parse_decl_type()?;
                Node::Primary(PrimaryContext {
                    prefix: None,
                    pathable: None,
                    path_parts: Vec::new(),
                    suffix: None,
                    non_pathable: Some(Box::new(Node::TypeExpr(TypeExprContext {
                        decl_type: Box::new(decl_type),
                    }))),
                })
            } else {
                self.parse_base_expr(next_min)?
            };
            ctx.left_assos.push(Node::LeftAsso(LeftAssoContext {
                binaryop: Box::new(binaryop),
                right: Box::new(right),
            }));
        }
        Ok(Node::BaseExpr(ctx))
    }

    fn parse_binaryop(&mut self) -> PResult<Node> {
        if !self.is_middle_operator(self.lt()) {
            let tok = self.lt();
            return Err(self.syntax(
                tok,
                format!("mismatched input '{}' expecting operator", token_text(tok)),
            ));
        }
        Ok(Node::Binaryop(BinaryopContext {
            token: self.consume_node(),
        }))
    }

    fn parse_primary(&mut self) -> PResult<Node> {
        if self.is_lambda_start() {
            let non_pathable = self.parse_lambda_expr()?;
            return Ok(Node::Primary(PrimaryContext {
                prefix: None,
                pathable: None,
                path_parts: Vec::new(),
                suffix: None,
                non_pathable: Some(Box::new(non_pathable)),
            }));
        }
        if self.la(IF) {
            let non_pathable = self.parse_ql_if()?;
            return Ok(Node::Primary(PrimaryContext {
                prefix: None,
                pathable: None,
                path_parts: Vec::new(),
                suffix: None,
                non_pathable: Some(Box::new(non_pathable)),
            }));
        }
        if self.la(SWITCH) && self.la_at(1, LPAREN) {
            let non_pathable = self.parse_switch_expr()?;
            return Ok(Node::Primary(PrimaryContext {
                prefix: None,
                pathable: None,
                path_parts: Vec::new(),
                suffix: None,
                non_pathable: Some(Box::new(non_pathable)),
            }));
        }
        if self.la(TRY) {
            let non_pathable = self.parse_try_catch_expr()?;
            return Ok(Node::Primary(PrimaryContext {
                prefix: None,
                pathable: None,
                path_parts: Vec::new(),
                suffix: None,
                non_pathable: Some(Box::new(non_pathable)),
            }));
        }
        let mut ctx = PrimaryContext {
            prefix: None,
            pathable: None,
            path_parts: Vec::new(),
            suffix: None,
            non_pathable: None,
        };
        if self.is_prefix_operator(self.lt()) {
            let op_id = self.parse_op_id()?;
            ctx.prefix = Some(Box::new(Node::PrefixExpress(PrefixExpressContext {
                op_id: Box::new(op_id),
            })));
        }
        ctx.pathable = Some(Box::new(self.parse_primary_no_fix_pathable()?));
        loop {
            let before_newlines = self.p;
            self.skip_newlines();
            match self.try_parse_path_part()? {
                None => {
                    self.p = before_newlines;
                    break;
                }
                Some(part) => ctx.path_parts.push(part),
            }
        }
        if self.is_suffix_operator(self.lt()) {
            let op_id = self.parse_op_id()?;
            ctx.suffix = Some(Box::new(Node::SuffixExpress(SuffixExpressContext {
                op_id: Box::new(op_id),
            })));
        }
        Ok(Node::Primary(ctx))
    }

    fn parse_primary_no_fix_pathable(&mut self) -> PResult<Node> {
        if is_literal_start(self.lt().token_type()) {
            let literal = self.parse_literal()?;
            return Ok(Node::ConstExpr(ConstExprContext {
                literal: Box::new(literal),
            }));
        }
        if self.la(LPAREN) {
            if self.is_cast_start() {
                let lparen = self.expect(LPAREN, "'('")?;
                self.skip_newlines();
                let decl_type = self.parse_decl_type()?;
                self.skip_newlines();
                let rparen = self.expect(RPAREN, "')'")?;
                let primary = self.parse_primary()?;
                return Ok(Node::CastExpr(CastExprContext {
                    lparen,
                    decl_type: Box::new(decl_type),
                    rparen,
                    primary: Box::new(primary),
                }));
            }
            let lparen = self.expect(LPAREN, "'('")?;
            self.skip_newlines();
            let expression = self.parse_expression()?;
            self.skip_newlines();
            let rparen = self.expect(RPAREN, "')'")?;
            return Ok(Node::GroupExpr(GroupExprContext {
                lparen,
                expression: Box::new(expression),
                rparen,
            }));
        }
        if self.la(NEW) {
            return self.parse_new_expr();
        }
        if is_primitive_type(self.lt().token_type()) {
            let decl_type = self.parse_decl_type()?;
            return Ok(Node::TypeExpr(TypeExprContext {
                decl_type: Box::new(decl_type),
            }));
        }
        if self.la(LBRACK) {
            return self.parse_list_expr();
        }
        if self.la(LBRACE) {
            return if self.is_map_expr_ahead() {
                self.parse_map_expr()
            } else {
                self.parse_block_expr()
            };
        }
        if self.la(SELECTOR_START) {
            let selector_start = self.consume_node();
            let selector_variable = self.expect(SELECTOR_VARIABLE_VANME, "selector variable")?;
            return Ok(Node::ContextSelectExpr(ContextSelectExprContext {
                selector_start,
                selector_variable,
            }));
        }
        if is_var_id_token(self.lt().token_type()) {
            let var_id = self.parse_var_id()?;
            let mut ctx = VarIdExprContext {
                var_id: Box::new(var_id),
                lparen: None,
                argument_list: None,
                rparen: None,
            };
            if self.la(LPAREN) {
                ctx.lparen = Some(self.consume_node());
                self.skip_newlines();
                if !self.la(RPAREN) {
                    ctx.argument_list = Some(Box::new(self.parse_argument_list()?));
                }
                self.skip_newlines();
                ctx.rparen = Some(self.expect(RPAREN, "')'")?);
            }
            return Ok(Node::VarIdExpr(ctx));
        }
        let tok = self.lt();
        Err(self.syntax(
            tok,
            format!(
                "mismatched input '{}' expecting expression",
                token_text(tok)
            ),
        ))
    }

    /// Java `isCastStart`: speculative, catches all errors.
    fn is_cast_start(&mut self) -> bool {
        let save = self.p;
        let result = (|| -> PResult<bool> {
            self.expect(LPAREN, "'('")?;
            self.skip_newlines();
            self.parse_decl_type()?;
            self.skip_newlines();
            if !self.la(RPAREN) {
                return Ok(false);
            }
            self.consume();
            Ok(self.is_primary_start(self.lt()))
        })();
        self.p = save;
        matches!(result, Ok(true))
    }

    fn parse_new_expr(&mut self) -> PResult<Node> {
        let new_token = self.consume_node();
        let decl_type = self.parse_decl_type_no_arr()?;
        if self.la(LPAREN) {
            let var_ids = match &decl_type {
                Node::DeclTypeNoArr(d) => match &d.cls_type {
                    None => {
                        return Err(self.syntax(
                            new_token.symbol(),
                            "primitive type can not be constructed".to_string(),
                        ));
                    }
                    Some(cls_type) => match cls_type.as_ref() {
                        Node::ClsType(c) => c.var_ids.clone(),
                        _ => unreachable!("cls_type holds ClsType"),
                    },
                },
                _ => unreachable!("parse_decl_type_no_arr returns DeclTypeNoArr"),
            };
            let lparen = self.expect(LPAREN, "'('")?;
            self.skip_newlines();
            let argument_list = if !self.la(RPAREN) {
                Some(Box::new(self.parse_argument_list()?))
            } else {
                None
            };
            self.skip_newlines();
            let rparen = self.expect(RPAREN, "')'")?;
            return Ok(Node::NewObjExpr(NewObjExprContext {
                new_token,
                var_ids,
                lparen,
                argument_list,
                rparen,
            }));
        }
        if self.la(LBRACK) && !self.la_at(1, RBRACK) {
            let dim_exprs = self.parse_dim_exprs()?;
            return Ok(Node::NewEmptyArrExpr(NewEmptyArrExprContext {
                new_token,
                decl_type_no_arr: Box::new(decl_type),
                dim_exprs: Box::new(dim_exprs),
            }));
        }
        let dims = self.parse_dims()?;
        let array_initializer = self.parse_array_initializer()?;
        Ok(Node::NewInitArrExpr(NewInitArrExprContext {
            new_token,
            decl_type_no_arr: Box::new(decl_type),
            dims: Box::new(dims),
            array_initializer: Box::new(array_initializer),
        }))
    }

    fn parse_list_expr(&mut self) -> PResult<Node> {
        let lbrack = self.expect(LBRACK, "'['")?;
        self.skip_newlines();
        let list_items = if !self.la(RBRACK) {
            Some(Box::new(self.parse_list_items()?))
        } else {
            None
        };
        self.skip_newlines();
        let rbrack = self.expect(RBRACK, "']'")?;
        Ok(Node::ListExpr(ListExprContext {
            lbrack,
            list_items,
            rbrack,
        }))
    }

    fn parse_list_items(&mut self) -> PResult<Node> {
        let mut expressions = vec![self.parse_expression()?];
        let mut commas = Vec::new();
        self.skip_newlines();
        while self.la(COMMA) {
            commas.push(self.consume_node());
            self.skip_newlines();
            if self.la(RBRACK) {
                break;
            }
            expressions.push(self.parse_expression()?);
            self.skip_newlines();
        }
        Ok(Node::ListItems(ListItemsContext {
            expressions,
            commas,
        }))
    }

    fn parse_map_expr(&mut self) -> PResult<Node> {
        let lbrace = self.expect(LBRACE, "'{'")?;
        self.skip_newlines();
        let mut map_entries = MapEntriesContext {
            empty_colon: None,
            entries: Vec::new(),
            commas: Vec::new(),
        };
        if self.la(COLON) {
            map_entries.empty_colon = Some(self.consume_node());
        } else {
            map_entries.entries.push(self.parse_map_entry()?);
            self.skip_newlines();
            while self.la(COMMA) {
                map_entries.commas.push(self.consume_node());
                self.skip_newlines();
                if self.la(RBRACE) {
                    break;
                }
                map_entries.entries.push(self.parse_map_entry()?);
                self.skip_newlines();
            }
        }
        self.skip_newlines();
        let rbrace = self.expect(RBRACE, "'}'")?;
        Ok(Node::MapExpr(MapExprContext {
            lbrace,
            map_entries: Box::new(Node::MapEntries(map_entries)),
            rbrace,
        }))
    }

    fn parse_map_entry(&mut self) -> PResult<Node> {
        let map_key = self.parse_map_key()?;
        self.skip_newlines();
        let colon = self.expect(COLON, "':'")?;
        self.skip_newlines();
        let map_value = if map_key.text() == "'@class'" && self.la(QUOTE_STRING_LITERAL) {
            Node::ClsValue(ClsValueContext {
                quote: self.consume_node(),
            })
        } else {
            Node::EValue(EValueContext {
                expression: Box::new(self.parse_expression()?),
            })
        };
        Ok(Node::MapEntry(MapEntryContext {
            map_key: Box::new(map_key),
            colon,
            map_value: Box::new(map_value),
        }))
    }

    fn parse_map_key(&mut self) -> PResult<Node> {
        if self.la(QUOTE_STRING_LITERAL) {
            return Ok(Node::QuoteStringKey(QuoteStringKeyContext {
                token: self.consume_node(),
            }));
        }
        if self.la(DOUBLE_QUOTE) {
            let double_quote_string = self.parse_double_quote_string_literal()?;
            return Ok(Node::StringKey(StringKeyContext {
                double_quote_string: Box::new(double_quote_string),
            }));
        }
        if is_id_map_key(self.lt().token_type()) {
            return Ok(Node::IdKey(IdKeyContext {
                token: self.consume_node(),
            }));
        }
        let tok = self.lt();
        Err(self.syntax(
            tok,
            format!("mismatched input '{}' expecting map key", token_text(tok)),
        ))
    }

    fn parse_block_expr(&mut self) -> PResult<Node> {
        let _guard = DepthGuard::enter(self.script, self.lt())?;
        let lbrace = self.expect(LBRACE, "'{'")?;
        self.skip_newlines();
        let block_statements = if !self.la(RBRACE) {
            Some(Box::new(self.parse_block_statements_until(RBRACE)?))
        } else {
            None
        };
        self.skip_newlines();
        let rbrace = self.expect(RBRACE, "'}'")?;
        Ok(Node::BlockExpr(BlockExprContext {
            lbrace,
            block_statements,
            rbrace,
        }))
    }
}

// ---------------------------------------------------------------------------
// if / switch / try / lambda
// ---------------------------------------------------------------------------

