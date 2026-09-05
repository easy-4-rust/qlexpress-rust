impl<'a> QLParser<'a> {
    fn parse_ql_if(&mut self) -> PResult<Node> {
        let if_token = self.consume_node();
        let lparen = self.expect(LPAREN, "'('")?;
        self.skip_newlines();
        let condition = self.parse_expression()?;
        self.skip_newlines();
        let rparen = self.expect(RPAREN, "')'")?;
        self.skip_newlines();
        let then_keyword = if self.la(THEN) {
            let kw = self.consume_node();
            self.skip_newlines();
            Some(kw)
        } else {
            None
        };
        let then_body = self.parse_then_body()?;
        let save = self.p;
        self.skip_newlines();
        let (else_keyword, else_body) = if self.la(ELSE) {
            let else_keyword = self.consume_node();
            self.skip_newlines();
            (Some(else_keyword), Some(Box::new(self.parse_else_body()?)))
        } else {
            self.p = save;
            (None, None)
        };
        Ok(Node::QlIf(QlIfContext {
            if_token,
            lparen,
            then_keyword,
            condition: Box::new(condition),
            rparen,
            then_body: Box::new(then_body),
            else_body,
            else_keyword,
        }))
    }

    fn parse_then_body(&mut self) -> PResult<Node> {
        if self.la(LBRACE) {
            let lbrace = self.expect(LBRACE, "'{'")?;
            self.skip_newlines();
            let block_statements = if !self.la(RBRACE) {
                Some(Box::new(self.parse_block_statements_until(RBRACE)?))
            } else {
                None
            };
            self.skip_newlines();
            let rbrace = self.expect(RBRACE, "'}'")?;
            return Ok(Node::ThenBody(ThenBodyContext {
                lbrace: Some(lbrace),
                block_statements,
                rbrace: Some(rbrace),
                non_expression_statement: None,
                expression: None,
            }));
        }
        if self.is_non_expression_statement_start()? {
            let non_expression_statement = self.parse_non_expression_statement()?;
            return Ok(Node::ThenBody(ThenBodyContext {
                lbrace: None,
                block_statements: None,
                rbrace: None,
                non_expression_statement: Some(Box::new(non_expression_statement)),
                expression: None,
            }));
        }
        let expression = self.parse_expression()?;
        Ok(Node::ThenBody(ThenBodyContext {
            lbrace: None,
            block_statements: None,
            rbrace: None,
            non_expression_statement: None,
            expression: Some(Box::new(expression)),
        }))
    }

    fn parse_else_body(&mut self) -> PResult<Node> {
        if self.la(LBRACE) {
            let lbrace = self.expect(LBRACE, "'{'")?;
            self.skip_newlines();
            let block_statements = if !self.la(RBRACE) {
                Some(Box::new(self.parse_block_statements_until(RBRACE)?))
            } else {
                None
            };
            self.skip_newlines();
            let rbrace = self.expect(RBRACE, "'}'")?;
            return Ok(Node::ElseBody(ElseBodyContext {
                lbrace: Some(lbrace),
                block_statements,
                rbrace: Some(rbrace),
                ql_if: None,
                non_expression_statement: None,
                expression: None,
            }));
        }
        if self.la(IF) {
            let ql_if = self.parse_ql_if()?;
            return Ok(Node::ElseBody(ElseBodyContext {
                lbrace: None,
                block_statements: None,
                rbrace: None,
                ql_if: Some(Box::new(ql_if)),
                non_expression_statement: None,
                expression: None,
            }));
        }
        if self.is_non_expression_statement_start()? {
            let non_expression_statement = self.parse_non_expression_statement()?;
            return Ok(Node::ElseBody(ElseBodyContext {
                lbrace: None,
                block_statements: None,
                rbrace: None,
                ql_if: None,
                non_expression_statement: Some(Box::new(non_expression_statement)),
                expression: None,
            }));
        }
        let expression = self.parse_expression()?;
        Ok(Node::ElseBody(ElseBodyContext {
            lbrace: None,
            block_statements: None,
            rbrace: None,
            ql_if: None,
            non_expression_statement: None,
            expression: Some(Box::new(expression)),
        }))
    }

    fn parse_switch_expr(&mut self) -> PResult<Node> {
        let switch_token = self.consume_node();
        let lparen = self.expect(LPAREN, "'('")?;
        self.skip_newlines();
        let expression = self.parse_expression()?;
        self.skip_newlines();
        let rparen = self.expect(RPAREN, "')'")?;
        let lbrace = self.expect(LBRACE, "'{'")?;
        self.skip_newlines();
        let groups = if !self.la(RBRACE) {
            Some(Box::new(self.parse_switch_case_groups()?))
        } else {
            None
        };
        self.skip_newlines();
        let rbrace = self.expect(RBRACE, "'}'")?;
        Ok(Node::SwitchExpr(SwitchExprContext {
            switch_token,
            lparen,
            expression: Box::new(expression),
            rparen,
            lbrace,
            groups,
            rbrace,
        }))
    }

    fn parse_switch_case_groups(&mut self) -> PResult<Node> {
        let mut ctx = SwitchCaseGroupsContext { groups: Vec::new() };
        while self.la(CASE) || self.la(DEFAULT) {
            ctx.groups.push(self.parse_switch_case_group()?);
            self.skip_newlines();
        }
        Ok(Node::SwitchCaseGroups(ctx))
    }

    fn parse_switch_case_group(&mut self) -> PResult<Node> {
        let save = self.p;
        if let Some(expr_label) = self.try_parse_switch_expression_label() {
            self.skip_newlines();
            let expression = self.parse_expression()?;
            self.skip_newlines();
            return Ok(Node::SwitchExprGroup(SwitchExprGroupContext {
                label: Box::new(expr_label),
                expression: Box::new(expression),
            }));
        }
        self.p = save;
        let labels = self.parse_switch_labels()?;
        self.skip_newlines();
        let block_statements = if !self.la(CASE) && !self.la(DEFAULT) && !self.la(RBRACE) {
            Some(Box::new(
                self.parse_block_statements_until_switch_group_end()?,
            ))
        } else {
            None
        };
        self.skip_newlines();
        Ok(Node::SwitchStatementGroup(SwitchStatementGroupContext {
            labels: Box::new(labels),
            block_statements,
        }))
    }

    fn parse_block_statements_until_switch_group_end(&mut self) -> PResult<Node> {
        let _guard = DepthGuard::enter()?;
        let mut ctx = BlockStatementsContext {
            statements: Vec::new(),
        };
        while !self.la(CASE) && !self.la(DEFAULT) && !self.la(RBRACE) && !self.la(EOF) {
            ctx.statements.push(self.parse_block_statement()?);
        }
        Ok(Node::BlockStatements(ctx))
    }

    /// Java `tryParseSwitchExpressionLabel`: catches all errors.
    fn try_parse_switch_expression_label(&mut self) -> Option<Node> {
        let save = self.p;
        let parsed = (|| -> PResult<Node> {
            let mut ctx = SwitchExpressionLabelContext {
                case_token: None,
                default_token: None,
                expression_list: None,
                arrow: TerminalNode::new(self.lt().clone()),
            };
            if self.la(CASE) {
                ctx.case_token = Some(self.consume_node());
                ctx.expression_list = Some(Box::new(self.parse_expression_list_until_arrow()?));
                self.skip_newlines();
                ctx.arrow = self.expect(ARROW, "'->'")?;
            } else if self.la(DEFAULT) {
                ctx.default_token = Some(self.consume_node());
                self.skip_newlines();
                ctx.arrow = self.expect(ARROW, "'->'")?;
            } else {
                return Err(ParseFail::Backtrack);
            }
            Ok(Node::SwitchExpressionLabel(ctx))
        })();
        match parsed {
            Ok(node) => {
                self.skip_newlines();
                Some(node)
            }
            Err(_) => {
                self.p = save;
                None
            }
        }
    }

    fn parse_switch_labels(&mut self) -> PResult<Node> {
        let mut ctx = SwitchLabelsContext { labels: Vec::new() };
        while self.la(CASE) || self.la(DEFAULT) {
            let mut label = SwitchLabelContext {
                case_token: None,
                default_token: None,
                expression: None,
                colon: TerminalNode::new(self.lt().clone()),
            };
            if self.la(CASE) {
                label.case_token = Some(self.consume_node());
                label.expression = Some(Box::new(self.parse_expression()?));
            } else {
                label.default_token = Some(self.consume_node());
            }
            label.colon = self.expect(COLON, "':'")?;
            ctx.labels.push(Node::SwitchLabel(label));
            self.skip_newlines();
        }
        Ok(Node::SwitchLabels(ctx))
    }

    fn parse_expression_list_until_arrow(&mut self) -> PResult<Node> {
        let _guard = DepthGuard::enter()?;
        let mut ctx = ExpressionListContext {
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
        Ok(Node::ExpressionList(ctx))
    }

    fn parse_try_catch_expr(&mut self) -> PResult<Node> {
        let try_token = self.consume_node();
        let (lbrace, block_statements, rbrace) = self.parse_braced_block()?;
        let save = self.p;
        self.skip_newlines();
        let try_catches = if self.la(CATCH) {
            let mut catches = Vec::new();
            while self.la(CATCH) {
                catches.push(self.parse_try_catch()?);
                let save = self.p;
                self.skip_newlines();
                if !self.la(CATCH) {
                    self.p = save;
                    break;
                }
            }
            Some(Box::new(Node::TryCatches(TryCatchesContext { catches })))
        } else {
            self.p = save;
            None
        };
        let save = self.p;
        self.skip_newlines();
        let try_finally = if self.la(FINALLY) {
            Some(Box::new(self.parse_try_finally()?))
        } else {
            self.p = save;
            None
        };
        Ok(Node::TryCatchExpr(TryCatchExprContext {
            try_token,
            lbrace,
            block_statements: block_statements.map(Box::new),
            rbrace,
            try_catches,
            try_finally,
        }))
    }

    fn parse_try_catch(&mut self) -> PResult<Node> {
        let catch_token = self.consume_node();
        let lparen = self.expect(LPAREN, "'('")?;
        let catch_params = self.parse_catch_params()?;
        let rparen = self.expect(RPAREN, "')'")?;
        let (lbrace, block_statements, rbrace) = self.parse_braced_block()?;
        Ok(Node::TryCatch(TryCatchContext {
            catch_token,
            lparen,
            catch_params: Box::new(catch_params),
            rparen,
            lbrace,
            block_statements: block_statements.map(Box::new),
            rbrace,
        }))
    }

    fn parse_catch_params(&mut self) -> PResult<Node> {
        if self.single_var_before(&[RPAREN]) {
            let var_id = self.parse_var_id()?;
            return Ok(Node::CatchParams(CatchParamsContext {
                decl_types: Vec::new(),
                bit_ors: Vec::new(),
                var_id: Box::new(var_id),
            }));
        }
        let mut decl_types = vec![self.parse_decl_type()?];
        let mut bit_ors = Vec::new();
        while self.la(BIT_OR) {
            bit_ors.push(self.consume_node());
            decl_types.push(self.parse_decl_type()?);
        }
        let var_id = self.parse_var_id()?;
        Ok(Node::CatchParams(CatchParamsContext {
            decl_types,
            bit_ors,
            var_id: Box::new(var_id),
        }))
    }

    fn parse_try_finally(&mut self) -> PResult<Node> {
        let finally_token = self.consume_node();
        let (lbrace, block_statements, rbrace) = self.parse_braced_block()?;
        Ok(Node::TryFinally(TryFinallyContext {
            finally_token,
            lbrace,
            block_statements: block_statements.map(Box::new),
            rbrace,
        }))
    }

    fn parse_lambda_expr(&mut self) -> PResult<Node> {
        let lambda_parameters = self.parse_lambda_parameters()?;
        let arrow = self.expect(ARROW, "'->'")?;
        self.skip_newlines();
        let mut ctx = LambdaExprContext {
            lambda_parameters: Box::new(lambda_parameters),
            arrow,
            lbrace: None,
            block_statements: None,
            rbrace: None,
            expression: None,
        };
        if self.la(LBRACE) && !self.is_map_expr_ahead() {
            ctx.lbrace = Some(self.expect(LBRACE, "'{'")?);
            self.skip_newlines();
            if !self.la(RBRACE) {
                ctx.block_statements = Some(Box::new(self.parse_block_statements_until(RBRACE)?));
            }
            self.skip_newlines();
            ctx.rbrace = Some(self.expect(RBRACE, "'}'")?);
        } else {
            ctx.expression = Some(Box::new(self.parse_expression()?));
        }
        Ok(Node::LambdaExpr(ctx))
    }

    fn parse_lambda_parameters(&mut self) -> PResult<Node> {
        if is_var_id_token(self.lt().token_type()) && self.la_at(1, ARROW) {
            let var_id = self.parse_var_id()?;
            return Ok(Node::LambdaParameters(LambdaParametersContext {
                var_id: Some(Box::new(var_id)),
                lparen: None,
                params: None,
                rparen: None,
            }));
        }
        let lparen = self.expect(LPAREN, "'('")?;
        let params = if !self.la(RPAREN) {
            Some(Box::new(self.parse_formal_or_inferred_parameter_list()?))
        } else {
            None
        };
        let rparen = self.expect(RPAREN, "')'")?;
        Ok(Node::LambdaParameters(LambdaParametersContext {
            var_id: None,
            lparen: Some(lparen),
            params,
            rparen: Some(rparen),
        }))
    }

    fn parse_formal_or_inferred_parameter_list(&mut self) -> PResult<Node> {
        let mut ctx = FormalOrInferredParameterListContext {
            params: Vec::new(),
            commas: Vec::new(),
        };
        ctx.params.push(self.parse_formal_or_inferred_parameter()?);
        self.skip_newlines();
        while self.la(COMMA) {
            ctx.commas.push(self.consume_node());
            self.skip_newlines();
            ctx.params.push(self.parse_formal_or_inferred_parameter()?);
            self.skip_newlines();
        }
        Ok(Node::FormalOrInferredParameterList(ctx))
    }

    fn parse_formal_or_inferred_parameter(&mut self) -> PResult<Node> {
        let mut ctx = FormalOrInferredParameterContext {
            decl_type: None,
            var_id: Box::new(Node::VarId(VarIdContext {
                token: TerminalNode::new(self.lt().clone()),
            })),
        };
        if !self.single_var_before(&[COMMA, RPAREN]) {
            let save = self.p;
            let decl_type = self.parse_decl_type();
            match decl_type {
                Ok(decl_type) if is_var_id_token(self.lt().token_type()) => {
                    ctx.decl_type = Some(Box::new(decl_type));
                }
                Ok(_) => {
                    self.p = save;
                }
                Err(_) => {
                    self.p = save;
                }
            }
        }
        ctx.var_id = Box::new(self.parse_var_id()?);
        Ok(Node::FormalOrInferredParameter(ctx))
    }
}

// ---------------------------------------------------------------------------
// Path parts / index / arguments / literals / import / types
// ---------------------------------------------------------------------------

