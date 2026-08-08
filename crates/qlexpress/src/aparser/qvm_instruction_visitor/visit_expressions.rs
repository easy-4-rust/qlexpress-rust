macro_rules! qvm_visit_expression_methods {
    () => {
    /// Java `visitExpression` (assignment).
    fn visit_expression(&mut self, ctx: &ExpressionContext) {
        if self.failed() {
            return;
        }
        if let Some(ternary) = &ctx.ternary {
            ternary.accept(self);
            return;
        }

        if let Some(left) = &ctx.left {
            left.accept(self);
        }
        if let Some(expression) = &ctx.expression {
            expression.accept(self);
        }

        let Some(assign_operator) = &ctx.assign_operator else {
            return;
        };
        let operator_id = assign_operator.text();
        let Some(binary_operator) = self.operator_factory.get_binary_operator(&operator_id) else {
            return;
        };
        let reporter = self.reporter_of(assign_operator);
        let trace_key = assign_operator.start_token().map(Token::start_index);
        self.add_instruction(Box::new(OperatorInstruction::new(
            reporter,
            binary_operator,
            trace_key,
        )));
    }

    /// Java `visitTernaryExpr`.
    fn visit_ternary_expr(&mut self, ctx: &TernaryExprContext) {
        if self.failed() {
            return;
        }
        ctx.condition.accept(self);

        if let Some(question) = &ctx.question {
            let then_visitor = self.parse_with_sub_visitor(
                ctx.then_expr.as_ref().expect("ternary then expr"),
                Rc::clone(&self.generator_scope),
                Context::Macro,
            );
            let else_visitor = self.parse_with_sub_visitor(
                ctx.else_expr.as_ref().expect("ternary else expr"),
                Rc::clone(&self.generator_scope),
                Context::Macro,
            );
            let then_instructions = then_visitor.take_instructions();
            let else_instructions = else_visitor.take_instructions();
            self.if_else_instructions(
                self.new_reporter_with_token(question.symbol()),
                then_instructions,
                None,
                else_instructions,
                None,
                Some(question.symbol().start_index()),
            );
        }
    }

    /// Java `visitBlockExpr`.
    fn visit_block_expr(&mut self, ctx: &BlockExprContext) {
        if self.failed() {
            return;
        }
        let block_err_reporter = self.new_reporter_with_token(ctx.lbrace.symbol());
        let Some(block_statements) = &ctx.block_statements else {
            self.add_instruction(Box::new(ConstInstruction::new(
                block_err_reporter,
                DataValue::NULL_VALUE,
                None,
            )));
            return;
        };

        let block_scope_name = self.block_scope_name();
        let scope = self.child_scope(block_scope_name.clone());
        let block_sub_visitor =
            self.parse_with_sub_visitor(block_statements, scope, Context::Macro);
        let block_instructions = block_sub_visitor.take_instructions();

        self.add_instruction(Box::new(NewScopeInstruction::new(
            Rc::clone(&block_err_reporter),
            block_scope_name.clone(),
        )));
        for instruction in block_instructions {
            self.pure_add_instruction(instruction);
        }
        self.add_instruction(Box::new(CloseScopeInstruction::new(
            Rc::clone(&block_err_reporter),
            block_scope_name,
        )));
        if self.init_options.is_trace_expression() {
            self.pure_add_instruction(Box::new(TracePeekInstruction::new(
                block_err_reporter,
                Some(ctx.lbrace.symbol().start_index()),
            )));
        }
    }

    /// Java `visitCastExpr`.
    fn visit_cast_expr(&mut self, ctx: &CastExprContext) {
        if self.failed() {
            return;
        }
        let cast_cls = self.parse_decl_type(&ctx.decl_type);
        let error_reporter = self.reporter_of(&ctx.decl_type);
        self.add_instruction(Box::new(ConstInstruction::new(
            Rc::clone(&error_reporter),
            MetaClass::new(cast_cls).into_data_value(),
            None,
        )));
        ctx.primary.accept(self);
        self.add_instruction(Box::new(CastInstruction::new(error_reporter)));
    }

    /// Java `visitSwitchExpr`.
    fn visit_switch_expr(&mut self, ctx: &SwitchExprContext) {
        if self.failed() {
            return;
        }
        let mut groups = switch_groups(ctx);
        let Some(first_group) = groups.next() else {
            // Empty switch, push null as result
            self.add_instruction(Box::new(ConstInstruction::new(
                self.new_reporter_with_token(ctx.switch_token.symbol()),
                DataValue::NULL_VALUE,
                None,
            )));
            return;
        };

        // Check the type of first group to determine switch style
        let is_statement_style = matches!(first_group, Node::SwitchStatementGroup(_));
        let is_expression_style = matches!(first_group, Node::SwitchExprGroup(_));

        // Validate that all groups have the same type
        for group in groups {
            let current_is_statement = matches!(group, Node::SwitchStatementGroup(_));
            let current_is_expression = matches!(group, Node::SwitchExprGroup(_));
            if (is_statement_style && !current_is_statement)
                || (is_expression_style && !current_is_expression)
            {
                let error_token = group.start_token().cloned();
                if let Some(error_token) = error_token {
                    self.report_parse_err(
                        &error_token,
                        "SWITCH_STYLE_MISMATCH",
                        "Cannot mix traditional switch syntax (case X:) with switch expression syntax (case X ->) in the same switch block",
                    );
                }
                return;
            }
        }

        if is_statement_style {
            self.visit_switch_statement(ctx);
        } else if is_expression_style {
            self.visit_switch_expression(ctx);
        }
    }

    /// Java `visitListExpr`.
    fn visit_list_expr(&mut self, ctx: &ListExprContext) {
        if self.failed() {
            return;
        }
        let reporter = self.new_reporter_with_token(ctx.lbrack.symbol());
        self.visit_list_expr_inner(ctx.list_items.as_deref(), reporter);
    }

    /// Java `visitMapExpr`.
    fn visit_map_expr(&mut self, ctx: &MapExprContext) {
        if self.failed() {
            return;
        }
        let Node::MapEntries(entries_ctx) = &*ctx.map_entries else {
            return;
        };
        let mut keys: Vec<String> = Vec::with_capacity(entries_ctx.entries.len());
        let mut cls: Option<ClassRef> = None;
        for map_entry in &entries_ctx.entries {
            if self.failed() {
                return;
            }
            let Node::MapEntry(entry_ctx) = map_entry else {
                continue;
            };
            match &*entry_ctx.map_value {
                Node::EValue(e_value) => {
                    keys.push(self.parse_map_key(&entry_ctx.map_key));
                    e_value.expression.accept(self);
                }
                Node::ClsValue(cls_value) => {
                    let cls_text = cls_value.quote.text();
                    let cls_name = strip_quotes(cls_text);
                    let may_be_cls = self.import_manager.borrow().load_qualified(&cls_name);
                    match may_be_cls {
                        None => {
                            let key_text = entry_ctx.map_key.text();
                            keys.push(strip_quotes(&key_text));
                            let reporter = self.new_reporter_with_token(cls_value.quote.symbol());
                            self.add_instruction(Box::new(ConstInstruction::new(
                                reporter,
                                DataValue::string(QLStringUtils::parse_string_escape(cls_text)),
                                None,
                            )));
                            // @class override
                            cls = None;
                        }
                        Some(loaded) => {
                            cls = Some(ClassRef::from_name(&loaded));
                        }
                    }
                }
                _ => {}
            }
        }
        let reporter = self.new_reporter_with_token(ctx.lbrace.symbol());
        match cls {
            None => {
                self.add_instruction(Box::new(NewMapInstruction::new(reporter, keys)));
            }
            Some(cls) => {
                self.add_instruction(Box::new(NewFilledInstanceInstruction::new(
                    reporter, cls, keys,
                )));
            }
        }
    }

    /// Java `visitNewObjExpr`.
    fn visit_new_obj_expr(&mut self, ctx: &NewObjExprContext) {
        if self.failed() {
            return;
        }
        let new_cls = self.parse_cls_ids(&ctx.var_ids);
        if let Some(argument_list) = &ctx.argument_list {
            argument_list.accept(self);
        }
        let arg_num = ctx.argument_list.as_deref().map_or(0, argument_count);
        self.add_instruction(Box::new(NewInstanceInstruction::new(
            self.new_reporter_with_token(ctx.new_token.symbol()),
            new_cls,
            arg_num,
        )));
    }

    /// Java `visitNewEmptyArrExpr`.
    fn visit_new_empty_arr_expr(&mut self, ctx: &NewEmptyArrExprContext) {
        if self.failed() {
            return;
        }
        ctx.dim_exprs.accept(self);
        let dims = match &*ctx.dim_exprs {
            Node::DimExprs(dim_exprs) => dim_exprs.expressions.len(),
            _ => 0,
        };
        let arr_cls = self.parse_decl_type_no_arr(&ctx.decl_type_no_arr);
        self.add_instruction(Box::new(MultiNewArrayInstruction::new(
            self.new_reporter_with_token(ctx.new_token.symbol()),
            arr_cls,
            dims,
        )));
    }

    /// Java `visitNewInitArrExpr`.
    fn visit_new_init_arr_expr(&mut self, ctx: &NewInitArrExprContext) {
        if self.failed() {
            return;
        }
        let cls = self.parse_decl_type_no_arr(&ctx.decl_type_no_arr);
        // Java `embedClsInDims(cls, dims - 1)`: array-of-component class.
        let dimensions = dims_dim_count(&ctx.dims);
        self.new_arr_with_initializers(
            wrap_in_array(cls, dimensions.saturating_sub(1)),
            &ctx.array_initializer,
        );
    }

    /// Java `visitLambdaExpr`.
    fn visit_lambda_expr(&mut self, ctx: &LambdaExprContext) {
        if self.failed() {
            return;
        }
        let lambda_params = self.parse_lambda_params(&ctx.lambda_parameters);
        let lambda_scope_name = self.lambda_scope_name();

        let arrow_error_reporter = self.new_reporter_with_token(ctx.arrow.symbol());
        let sub_visitor = if let Some(expression) = &ctx.expression {
            let scope = self.child_scope(lambda_scope_name.clone());
            Some(self.parse_expr_body_with_sub_visitor(expression, scope, Context::Block))
        } else {
            ctx.block_statements.as_ref().map(|block_statements| {
                let scope = self.child_scope(lambda_scope_name.clone());
                self.parse_with_sub_visitor(block_statements, scope, Context::Block)
            })
        };

        match sub_visitor {
            None => {
                self.add_instruction(Box::new(LoadLambdaInstruction::new(
                    arrow_error_reporter,
                    Rc::new(QLambdaDefinitionEmpty::INSTANCE),
                )));
            }
            Some(sub_visitor) => {
                let max_stack_size = sub_visitor.max_stack_size();
                let instructions = sub_visitor.take_instructions();
                let lambda_definition = QLambdaDefinitionInner::new(
                    lambda_scope_name,
                    instructions,
                    lambda_params,
                    max_stack_size,
                );
                self.add_instruction(Box::new(LoadLambdaInstruction::new(
                    arrow_error_reporter,
                    Rc::new(lambda_definition),
                )));
            }
        }
    }


    };
}
