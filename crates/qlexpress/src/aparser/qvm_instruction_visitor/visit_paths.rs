macro_rules! qvm_visit_path_methods {
    () => {
    /// Java `visitVarIdExpr`.
    fn visit_var_id_expr(&mut self, ctx: &VarIdExprContext) {
        if self.failed() {
            return;
        }
        let reporter = self.reporter_of(&ctx.var_id);
        let trace_key = ctx.var_id.start_token().map(Token::start_index);
        self.add_instruction(Box::new(LoadInstruction::new(
            reporter,
            ctx.var_id.text(),
            trace_key,
        )));
    }

    /// Java `visitMethodInvoke` / `visitOptionalMethodInvoke` /
    /// `visitSpreadMethodInvoke` (merged via `ChainKind`).
    fn visit_method_invoke(&mut self, ctx: &MethodInvokeContext) {
        if self.failed() {
            return;
        }
        match ctx.chain {
            ChainKind::Plain => {
                self.visit_method_invoke_inner(ctx.argument_list.as_deref(), &ctx.var_id, false)
            }
            ChainKind::Optional => {
                self.visit_method_invoke_inner(ctx.argument_list.as_deref(), &ctx.var_id, true)
            }
            ChainKind::Spread => {
                if let Some(argument_list) = &ctx.argument_list {
                    argument_list.accept(self);
                }
                let arg_num = ctx.argument_list.as_deref().map_or(0, argument_count);
                let reporter = self.reporter_of(&ctx.var_id);
                self.add_instruction(Box::new(SpreadMethodInvokeInstruction::new(
                    reporter,
                    ctx.var_id.text(),
                    arg_num,
                )));
            }
        }
    }

    /// Java `visitFieldAccess` / `visitOptionalFieldAccess` /
    /// `visitSpreadFieldAccess` (merged via `ChainKind`).
    fn visit_field_access(&mut self, ctx: &FieldAccessContext) {
        if self.failed() {
            return;
        }
        let field_name = Self::parse_field_id(&ctx.field_id);
        let reporter = ctx
            .field_id
            .stop_token()
            .map(|t| self.new_reporter_with_token(t))
            .unwrap_or_else(|| self.new_reporter_with_token(ctx.dot.symbol()));
        match ctx.chain {
            ChainKind::Plain => self.add_instruction(Box::new(GetFieldInstruction::new(
                reporter, field_name, false,
            ))),
            ChainKind::Optional => self.add_instruction(Box::new(GetFieldInstruction::new(
                reporter, field_name, true,
            ))),
            ChainKind::Spread => self.add_instruction(Box::new(SpreadGetFieldInstruction::new(
                reporter, field_name,
            ))),
        }
    }

    /// Java `visitMethodAccess` (`Cls::method`).
    fn visit_method_access(&mut self, ctx: &MethodAccessContext) {
        if self.failed() {
            return;
        }
        self.add_instruction(Box::new(GetMethodInstruction::new(
            self.new_reporter_with_token(ctx.dcolon.symbol()),
            ctx.var_id.text(),
        )));
    }

    /// Java `visitIndexExpr`.
    fn visit_index_expr(&mut self, ctx: &IndexExprContext) {
        if self.failed() {
            return;
        }
        let Some(index_value_expr) = &ctx.index_value_expr else {
            // Java `ctx.getStop()` 指向 `]`，不能用起始 `[`，否则列号少 1。
            let stop = ctx.rbrack.symbol().clone();
            self.report_parse_err(
                &stop,
                error_codes::MISSING_INDEX,
                error_codes::error_msg(error_codes::MISSING_INDEX),
            );
            return;
        };
        let error_reporter = self.new_reporter_with_token(ctx.lbrack.symbol());
        match &**index_value_expr {
            Node::SingleIndex(single) => {
                single.expression.accept(self);
                self.add_instruction(Box::new(IndexInstruction::new(error_reporter)));
            }
            Node::SliceIndex(slice) => match (&slice.start, &slice.end) {
                (None, None) => self.add_instruction(Box::new(SliceInstruction::new(
                    error_reporter,
                    SliceMode::Copy,
                ))),
                (None, Some(end)) => {
                    end.accept(self);
                    self.add_instruction(Box::new(SliceInstruction::new(
                        error_reporter,
                        SliceMode::Left,
                    )));
                }
                (Some(start), None) => {
                    start.accept(self);
                    self.add_instruction(Box::new(SliceInstruction::new(
                        error_reporter,
                        SliceMode::Right,
                    )));
                }
                (Some(start), Some(end)) => {
                    start.accept(self);
                    end.accept(self);
                    self.add_instruction(Box::new(SliceInstruction::new(
                        error_reporter,
                        SliceMode::Both,
                    )));
                }
            },
            _ => {}
        }
    }

    /// Java `visitCustomPath`.
    fn visit_custom_path(&mut self, ctx: &CustomPathContext) {
        if self.failed() {
            return;
        }
        let error_reporter = self.reporter_of(&ctx.op_id);
        self.add_instruction(Box::new(ConstInstruction::new(
            Rc::clone(&error_reporter),
            DataValue::string(ctx.path_text.clone()),
            None,
        )));

        let operator_id = ctx.op_id.text();
        let Some(binary_operator) = self.operator_factory.get_binary_operator(&operator_id) else {
            return;
        };
        let trace_key = ctx.op_id.start_token().map(Token::start_index);
        self.add_instruction(Box::new(OperatorInstruction::new(
            error_reporter,
            binary_operator,
            trace_key,
        )));
    }

    /// Java `visitLeftAsso` (binary chain step, with `&&`/`||`
    /// short-circuit).
    fn visit_left_asso(&mut self, ctx: &LeftAssoContext) {
        if self.failed() {
            return;
        }
        let operator_id = ctx.binaryop.text();
        let op_err_reporter = self.reporter_of(&ctx.binaryop);
        let trace_key = ctx.binaryop.start_token().map(Token::start_index);
        // short circuit operator
        if operator_id == "&&" {
            self.jump_right_if_expect(false, op_err_reporter, &ctx.right, &operator_id, trace_key);
        } else if operator_id == "||" {
            self.jump_right_if_expect(true, op_err_reporter, &ctx.right, &operator_id, trace_key);
        } else {
            ctx.right.accept(self);
            let Some(binary_operator) = self.operator_factory.get_binary_operator(&operator_id)
            else {
                return;
            };
            self.add_instruction(Box::new(OperatorInstruction::new(
                op_err_reporter,
                binary_operator,
                trace_key,
            )));
        }
    }

    /// Java `visitLeftHandSide`.
    fn visit_left_hand_side(&mut self, ctx: &LeftHandSideContext) {
        if self.failed() {
            return;
        }
        let tail_part_start = self.parse_id_head_part(
            &ctx.var_id,
            ctx.lparen.is_some(),
            ctx.argument_list.as_deref(),
            &ctx.path_parts,
        );
        for path_part in &ctx.path_parts[tail_part_start..] {
            if self.failed() {
                return;
            }
            path_part.accept(self);
        }
    }

    /// Java `visitPrimary`.
    fn visit_primary(&mut self, ctx: &PrimaryContext) {
        if self.failed() {
            return;
        }
        if let Some(non_pathable) = &ctx.non_pathable {
            non_pathable.accept(self);
            return;
        }
        let Some(pathable) = &ctx.pathable else {
            return;
        };

        // path: head part
        let tail_part_start = self.parse_path_head_part(pathable, &ctx.path_parts);

        // tail part
        for path_part in &ctx.path_parts[tail_part_start..] {
            if self.failed() {
                return;
            }
            path_part.accept(self);
        }

        if let Some(suffix_express) = &ctx.suffix {
            let suffix_operator = suffix_express.text();
            let suffix_unary_operator = self
                .operator_factory
                .get_suffix_unary_operator(&suffix_operator)
                .expect("suffix unary operator must exist");
            let reporter = self.reporter_of(suffix_express);
            let trace_key = suffix_express.start_token().map(Token::start_index);
            self.add_instruction(Box::new(UnaryInstruction::new(
                reporter,
                suffix_unary_operator,
                trace_key,
            )));
        }

        if let Some(prefix_express) = &ctx.prefix {
            let prefix_operator = prefix_express.text();
            let prefix_unary_operator = self
                .operator_factory
                .get_prefix_unary_operator(&prefix_operator)
                .expect("prefix unary operator must exist");
            let reporter = self.reporter_of(prefix_express);
            let trace_key = prefix_express.start_token().map(Token::start_index);
            self.add_instruction(Box::new(UnaryInstruction::new(
                reporter,
                prefix_unary_operator,
                trace_key,
            )));
        }
    }

    /// Java `visitTypeExpr`.
    fn visit_type_expr(&mut self, ctx: &TypeExprContext) {
        if self.failed() {
            return;
        }
        let cls = self.parse_decl_type(&ctx.decl_type);
        let reporter = self.reporter_of(&ctx.decl_type);
        self.add_instruction(Box::new(ConstInstruction::new(
            reporter,
            MetaClass::new(cls).into_data_value(),
            None,
        )));
    }

    /// Java `visitContextSelectExpr`.
    fn visit_context_select_expr(&mut self, ctx: &ContextSelectExprContext) {
        if self.failed() {
            return;
        }
        let variable_name = ctx.selector_variable.text().trim().to_string();
        let reporter = self.new_reporter_with_token(ctx.selector_start.symbol());
        let trace_key = Some(ctx.selector_start.symbol().start_index());
        self.add_instruction(Box::new(LoadInstruction::new(
            reporter,
            variable_name,
            trace_key,
        )));
    }

    /// Java `visitLiteral`.
    fn visit_literal(&mut self, ctx: &LiteralContext) {
        if self.failed() {
            return;
        }
        if let Some(terminal) = &ctx.token {
            let symbol = terminal.symbol();
            let token_type = symbol.token_type();
            let text = symbol.text();
            let reporter = self.new_reporter_with_token(symbol);
            let trace_key = Some(symbol.start_index());
            let value: Option<DataValue> = match token_type as u16 {
                token::INTEGER_LITERAL => parse_integer_literal(&remove_char(text, '_')),
                token::FLOATING_POINT_LITERAL => parse_floating_literal(&remove_char(text, '_')),
                token::INTEGER_OR_FLOATING_LITERAL => {
                    let cleaned = remove_char(text, '_');
                    if cleaned.contains('.') {
                        parse_floating_literal(&cleaned)
                    } else {
                        parse_integer_literal(&cleaned)
                    }
                }
                token::QUOTE_STRING_LITERAL => {
                    Some(DataValue::string(QLStringUtils::parse_string_escape(text)))
                }
                token::NULL => Some(DataValue::NULL_VALUE),
                _ => None,
            };
            match value {
                Some(value) => {
                    self.add_instruction(Box::new(ConstInstruction::new(
                        reporter, value, trace_key,
                    )));
                }
                None if matches!(
                    token_type as u16,
                    token::INTEGER_LITERAL
                        | token::FLOATING_POINT_LITERAL
                        | token::INTEGER_OR_FLOATING_LITERAL
                ) =>
                {
                    let symbol = symbol.clone();
                    self.report_parse_err(
                        &symbol,
                        error_codes::INVALID_NUMBER,
                        error_codes::error_msg(error_codes::INVALID_NUMBER),
                    );
                }
                _ => {}
            }
            return;
        }
        if let Some(boolen) = &ctx.boolen {
            if let Node::BoolenLiteral(boolen_ctx) = &**boolen {
                let symbol = boolen_ctx.token.symbol();
                let bool_value = symbol.text() == "true";
                let reporter = self.new_reporter_with_token(symbol);
                let trace_key = Some(symbol.start_index());
                self.add_instruction(Box::new(ConstInstruction::new(
                    reporter,
                    DataValue::Bool(bool_value),
                    trace_key,
                )));
            }
            return;
        }
        if let Some(double_quote) = &ctx.double_quote_string {
            double_quote.accept(self);
        }
    }

    /// Java `visitDoubleQuoteStringLiteral` (string interpolation).
    fn visit_double_quote_string_literal(&mut self, ctx: &DoubleQuoteStringLiteralContext) {
        if self.failed() {
            return;
        }
        if self.init_options.interpolation_mode() == InterpolationMode::Disable {
            match &ctx.static_characters {
                None => {
                    let reporter = self.new_reporter_with_token(ctx.open_quote.symbol());
                    self.add_instruction(Box::new(ConstInstruction::new(
                        reporter,
                        DataValue::string(String::new()),
                        None,
                    )));
                }
                Some(characters) => {
                    let text = characters.text();
                    let reporter = self.new_reporter_with_token(ctx.open_quote.symbol());
                    self.add_instruction(Box::new(ConstInstruction::new(
                        reporter,
                        DataValue::string(QLStringUtils::parse_string_escape_start_end(
                            text,
                            0,
                            i32::try_from(text.encode_utf16().count())
                                .expect("Java String length exceeds i32"),
                        )),
                        None,
                    )));
                }
            }
            return;
        }
        // Children between the quotes (Java iterates children 1..count-1).
        let mut part_count = 0usize;
        if let Some(characters) = &ctx.static_characters {
            let text = characters.text().to_string();
            let reporter = self.new_reporter_with_token(characters.symbol());
            let trace_key = Some(ctx.open_quote.symbol().start_index());
            self.add_instruction(Box::new(ConstInstruction::new(
                reporter,
                DataValue::string(QLStringUtils::parse_string_escape_start_end(
                    &text,
                    0,
                    i32::try_from(text.encode_utf16().count())
                        .expect("Java String length exceeds i32"),
                )),
                trace_key,
            )));
            part_count += 1;
        } else {
            for part in &ctx.parts {
                if self.failed() {
                    return;
                }
                match part {
                    DyStrPart::Expr(node) => {
                        if let Node::StringExpression(string_expression) = &**node {
                            if let Some(expression) = &string_expression.expression {
                                // SCRIPT
                                expression.accept(self);
                            } else if let Some(var_terminal) = &string_expression.selector_variable
                            {
                                // VARIABLE
                                let var_name = var_terminal.text().trim().to_string();
                                let reporter = self.new_reporter_with_token(var_terminal.symbol());
                                self.add_instruction(Box::new(LoadInstruction::new(
                                    reporter, var_name, None,
                                )));
                            }
                        }
                    }
                    DyStrPart::Text(terminal) => {
                        let origin_str = terminal.text();
                        let reporter = self.new_reporter_with_token(terminal.symbol());
                        let trace_key = Some(ctx.open_quote.symbol().start_index());
                        self.add_instruction(Box::new(ConstInstruction::new(
                            reporter,
                            DataValue::string(QLStringUtils::parse_string_escape_start_end(
                                origin_str,
                                0,
                                i32::try_from(origin_str.encode_utf16().count())
                                    .expect("Java String length exceeds i32"),
                            )),
                            trace_key,
                        )));
                    }
                }
                part_count += 1;
            }
        }
        let reporter = self.new_reporter_with_token(ctx.open_quote.symbol());
        self.add_instruction(Box::new(StringJoinInstruction::new(reporter, part_count)));
    }

    };
}
