macro_rules! qvm_visit_statement_methods {
    () => {
    type T = ();

    /// Java `visitImportCls`.
    fn visit_import_cls(&mut self, ctx: &ImportClsContext) {
        if self.failed() {
            return;
        }
        let import_cls_path = ctx
            .var_ids
            .iter()
            .map(|id| id.text())
            .collect::<Vec<_>>()
            .join(".");
        self.import_manager
            .borrow_mut()
            .add_import(super::import_manager::QLImport::import_cls(import_cls_path));
    }

    /// Java `visitImportPack`.
    fn visit_import_pack(&mut self, ctx: &ImportPackContext) {
        if self.failed() {
            return;
        }
        let Some(last) = ctx.var_ids.last() else {
            return;
        };
        let last_text = last.text();
        let is_inner_cls = last_text.chars().next().is_some_and(|c| !c.is_lowercase());
        let import_path = ctx
            .var_ids
            .iter()
            .map(|id| id.text())
            .collect::<Vec<_>>()
            .join(".");
        let import = if is_inner_cls {
            super::import_manager::QLImport::import_inner_cls(import_path)
        } else {
            super::import_manager::QLImport::import_pack(import_path)
        };
        self.import_manager.borrow_mut().add_import(import);
    }

    /// Java `visitBlockStatements`: macros first, then function
    /// definitions, then the remaining statements.
    fn visit_block_statements(&mut self, ctx: &BlockStatementsContext) {
        if self.failed() {
            return;
        }
        let mut is_pre_express = false;
        let non_empty: Vec<&Node> = ctx
            .statements
            .iter()
            .filter(|bs| !matches!(bs, Node::EmptyStatement(_)))
            .collect();

        // First pass: process macro definitions to ensure they are
        // available for functions.
        for child in &non_empty {
            if let Node::MacroStatement(macro_ctx) = child {
                self.visit_macro_statement(macro_ctx);
            }
        }

        // Second pass: process all function definitions to support
        // forward references.
        for child in &non_empty {
            if let Node::FunctionStatement(function_ctx) = child {
                self.visit_function_statement(function_ctx);
            }
        }

        // Third pass: process all other statements.
        for child in &non_empty {
            if self.failed() {
                return;
            }
            if !matches!(child, Node::FunctionStatement(_) | Node::MacroStatement(_)) {
                if is_pre_express {
                    self.add_instruction(Box::new(PopInstruction::new(Rc::new(
                        PureErrReporter::INSTANCE,
                    ))));
                }
                is_pre_express = self.handle_stmt(child);
            }
        }

        if self.context == Context::Block && is_pre_express {
            self.add_instruction(Box::new(ReturnInstruction::new(
                Rc::new(PureErrReporter::INSTANCE),
                ReturnResultType::Continue,
                None,
            )));
        }
    }

    /// Java `visitTraditionalForStatement`.
    fn visit_traditional_for_statement(&mut self, ctx: &TraditionalForStatementContext) {
        if self.failed() {
            return;
        }
        let for_count = self.for_count();
        let for_err_reporter = self.new_reporter_with_token(ctx.for_token.symbol());

        // for init
        let for_init_lambda = self.generate_for_init_lambda(for_count, &ctx.for_init);

        // condition
        let for_condition_lambda = ctx
            .for_condition
            .as_ref()
            .and_then(|cond| self.generate_for_express_lambda(for_count, CONDITION_SUFFIX, cond));
        let condition_error_reporter = ctx
            .for_condition
            .as_ref()
            .and_then(|cond| cond.start_token())
            .map(|t| self.new_reporter_with_token(t))
            .unwrap_or_else(|| Rc::clone(&for_err_reporter));

        // for update
        let for_update_lambda = ctx
            .for_update
            .as_ref()
            .and_then(|upd| self.generate_for_express_lambda(for_count, UPDATE_SUFFIX, upd));

        // for body
        let body_scope_name =
            self.child_scope_name(&format!("{FOR_PREFIX}{for_count}{BODY_SUFFIX}"));
        let (for_body_lambda, _) = self.loop_body_visitor_definition(
            ctx.block_statements.as_deref(),
            body_scope_name,
            vec![],
            Rc::clone(&for_err_reporter),
        );

        let init_size = for_init_lambda.as_ref().map_or(0, |l| l.max_stack_size());
        let condition_size = for_condition_lambda
            .as_ref()
            .map_or(0, |l| l.max_stack_size());
        let update_size = for_update_lambda.as_ref().map_or(0, |l| l.max_stack_size());
        let for_scope_max_stack_size = init_size.max(condition_size).max(update_size);

        if self.init_options.is_trace_expression() {
            self.pure_add_instruction(Box::new(TraceEvaluatedInstruction::new(
                Rc::clone(&for_err_reporter),
                Some(ctx.for_token.symbol().start_index()),
            )));
        }

        self.add_instruction(Box::new(ForInstruction::new(
            for_err_reporter,
            for_init_lambda.map(|l| -> Rc<dyn QLambdaDefinition> { l }),
            for_condition_lambda.map(|l| -> Rc<dyn QLambdaDefinition> { l }),
            condition_error_reporter,
            for_update_lambda.map(|l| -> Rc<dyn QLambdaDefinition> { l }),
            for_scope_max_stack_size,
            for_body_lambda,
        )));
    }

    /// Java `visitForEachStatement`.
    fn visit_for_each_statement(&mut self, ctx: &ForEachStatementContext) {
        if self.failed() {
            return;
        }
        ctx.expression.accept(self);

        let it_var_cls = ctx
            .decl_type
            .as_ref()
            .map(|decl| self.parse_decl_type(decl))
            .unwrap_or_else(object_cls);

        let for_each_err_reporter = self.new_reporter_with_token(ctx.for_token.symbol());
        let for_count = self.for_count();
        let body_scope_name =
            self.child_scope_name(&format!("{FOR_PREFIX}{for_count}{BODY_SUFFIX}"));
        let (body_definition, _) = self.loop_body_visitor_definition(
            ctx.block_statements.as_deref(),
            body_scope_name,
            vec![Param::new(ctx.var_id.text(), Some(it_var_cls.clone()))],
            Rc::clone(&for_each_err_reporter),
        );

        if self.init_options.is_trace_expression() {
            self.pure_add_instruction(Box::new(TraceEvaluatedInstruction::new(
                Rc::clone(&for_each_err_reporter),
                Some(ctx.for_token.symbol().start_index()),
            )));
        }

        let target_reporter = self.reporter_of(&ctx.expression);
        self.add_instruction(Box::new(ForEachInstruction::new(
            for_each_err_reporter,
            body_definition,
            it_var_cls,
            target_reporter,
        )));
    }

    /// Java `visitWhileStatement`.
    fn visit_while_statement(&mut self, ctx: &WhileStatementContext) {
        if self.failed() {
            return;
        }
        let while_count = self.while_count();

        let while_condition_scope =
            self.child_scope_name(&format!("{WHILE_PREFIX}{while_count}{CONDITION_SUFFIX}"));
        let scope = self.child_scope(while_condition_scope.clone());
        let condition_sub =
            self.parse_expr_body_with_sub_visitor(&ctx.expression, scope, Context::Block);
        let condition_max_stack = condition_sub.max_stack_size();
        let condition_instructions = condition_sub.take_instructions();
        let condition_lambda: Rc<QLambdaDefinitionInner> = Rc::new(QLambdaDefinitionInner::new(
            while_condition_scope,
            condition_instructions,
            vec![],
            condition_max_stack,
        ));

        let while_err_reporter = self.new_reporter_with_token(ctx.while_token.symbol());
        let body_scope_name =
            self.child_scope_name(&format!("{WHILE_PREFIX}{while_count}{BODY_SUFFIX}"));
        let (while_body_lambda, body_max_stack) = self.loop_body_visitor_definition(
            ctx.block_statements.as_deref(),
            body_scope_name,
            vec![],
            Rc::clone(&while_err_reporter),
        );

        if self.init_options.is_trace_expression() {
            self.pure_add_instruction(Box::new(TraceEvaluatedInstruction::new(
                Rc::clone(&while_err_reporter),
                Some(ctx.while_token.symbol().start_index()),
            )));
        }

        // Java: max(condition, body) when the body is a
        // QLambdaDefinitionInner, else condition's.
        let while_scope_max_stack_size = match body_max_stack {
            Some(body_max) => condition_max_stack.max(body_max),
            None => condition_max_stack,
        };
        self.add_instruction(Box::new(WhileInstruction::new(
            while_err_reporter,
            condition_lambda,
            while_body_lambda,
            while_scope_max_stack_size,
        )));
    }

    /// Java `visitThrowStatement`.
    fn visit_throw_statement(&mut self, ctx: &ThrowStatementContext) {
        if self.failed() {
            return;
        }
        ctx.expression.accept(self);
        self.add_instruction(Box::new(ThrowInstruction::new(
            self.new_reporter_with_token(ctx.throw_token.symbol()),
        )));
    }

    /// Java `visitReturnStatement`.
    fn visit_return_statement(&mut self, ctx: &ReturnStatementContext) {
        if self.failed() {
            return;
        }
        let error_reporter = self.new_reporter_with_token(ctx.return_token.symbol());
        match &ctx.expression {
            None => {
                self.add_instruction(Box::new(ConstInstruction::new(
                    Rc::clone(&error_reporter),
                    DataValue::NULL_VALUE,
                    None,
                )));
            }
            Some(expression) => expression.accept(self),
        }
        self.add_instruction(Box::new(ReturnInstruction::new(
            error_reporter,
            ReturnResultType::Return,
            Some(ctx.return_token.symbol().start_index()),
        )));
    }

    /// Java `visitFunctionStatement`.
    fn visit_function_statement(&mut self, ctx: &FunctionStatementContext) {
        if self.failed() {
            return;
        }
        let params = ctx
            .params
            .as_ref()
            .map(|p| self.parse_formal_or_inferred_parameter_list(p))
            .unwrap_or_default();
        let function_name = ctx.var_id.text();
        let function_definition =
            self.parse_function_definition(&function_name, ctx.block_statements.as_deref(), params);

        let error_reporter = ctx
            .var_id
            .start_token()
            .map(|t| self.new_reporter_with_token(t))
            .unwrap_or_else(|| Rc::new(PureErrReporter::INSTANCE));

        if self.init_options.is_trace_expression() {
            let trace_key = ctx.var_id.start_token().map(Token::start_index);
            self.pure_add_instruction(Box::new(TraceEvaluatedInstruction::new(
                Rc::clone(&error_reporter),
                trace_key,
            )));
        }

        self.add_instruction(Box::new(DefineFunctionInstruction::new(
            error_reporter,
            function_definition.name().to_string(),
            function_definition,
        )));
    }

    /// Java `visitBreakContinueStatement`.
    fn visit_break_continue_statement(&mut self, ctx: &BreakContinueStatementContext) {
        if self.failed() {
            return;
        }
        let is_break = ctx.is_break();

        if self.init_options.is_trace_expression() {
            let trace_key = ctx.token.symbol().start_index();
            self.pure_add_instruction(Box::new(TraceEvaluatedInstruction::new(
                self.new_reporter_with_token(ctx.token.symbol()),
                Some(trace_key),
            )));
        }

        self.add_instruction(Box::new(BreakContinueInstruction::new(
            self.new_reporter_with_token(ctx.token.symbol()),
            is_break,
        )));
        if is_break {
            if let Some(indices) = &mut self.collect_break_indices {
                indices.push(self.instruction_list.len() - 1);
            }
        }
    }

    /// Java `visitQlIf`.
    fn visit_ql_if(&mut self, ctx: &QlIfContext) {
        if self.failed() {
            return;
        }
        ctx.condition.accept(self);

        let if_count = self.if_count();
        let if_error_reporter = self.new_reporter_with_token(ctx.if_token.symbol());
        let if_scope_name = self.child_scope_name(&format!("{IF_PREFIX}{if_count}"));
        self.add_instruction(Box::new(NewScopeInstruction::new(
            Rc::clone(&if_error_reporter),
            if_scope_name.clone(),
        )));

        // then branch
        let then_scope_name = self.child_scope_name(&format!("{IF_PREFIX}{if_count}{THEN_SUFFIX}"));
        let then_scope = self.child_scope(then_scope_name);
        let then_visitor = self.parse_with_sub_visitor(&ctx.then_body, then_scope, Context::Macro);
        let mut then_instructions = then_visitor.take_instructions();
        if let Node::ThenBody(then_body) = &*ctx.then_body {
            if Self::if_body_fill_const(
                then_body.expression.as_deref(),
                then_body.non_expression_statement.as_deref(),
                then_body.block_statements.as_deref(),
            ) {
                then_instructions.push(Box::new(ConstInstruction::new(
                    Rc::clone(&if_error_reporter),
                    DataValue::NULL_VALUE,
                    None,
                )));
            }
        }
        let then_trace_key = match &*ctx.then_body {
            Node::ThenBody(then_body) if then_body.lbrace.is_some() => {
                ctx.then_body.start_token().map(Token::start_index)
            }
            _ => None,
        };

        // else branch
        let else_scope_name = self.child_scope_name(&format!("{IF_PREFIX}{if_count}{ELSE_SUFFIX}"));
        let else_instructions = match &ctx.else_body {
            None => vec![Box::new(ConstInstruction::new(
                Rc::clone(&if_error_reporter),
                DataValue::NULL_VALUE,
                None,
            )) as Instruction],
            Some(else_body) => {
                let else_scope = self.child_scope(else_scope_name);
                let else_visitor =
                    self.parse_with_sub_visitor(else_body, else_scope, Context::Macro);
                let mut instructions = else_visitor.take_instructions();
                if let Node::ElseBody(else_ctx) = &**else_body {
                    if else_ctx.ql_if.is_none()
                        && Self::if_body_fill_const(
                            else_ctx.expression.as_deref(),
                            else_ctx.non_expression_statement.as_deref(),
                            else_ctx.block_statements.as_deref(),
                        )
                    {
                        instructions.push(Box::new(ConstInstruction::new(
                            Rc::clone(&if_error_reporter),
                            DataValue::NULL_VALUE,
                            None,
                        )));
                    }
                }
                instructions
            }
        };
        let else_trace_key = match ctx.else_body.as_deref() {
            Some(Node::ElseBody(else_ctx)) if else_ctx.lbrace.is_some() => ctx
                .else_body
                .as_ref()
                .and_then(|b| b.start_token())
                .map(Token::start_index),
            _ => None,
        };
        let trace_key = ctx.if_token.symbol().start_index();
        self.if_else_instructions(
            Rc::clone(&if_error_reporter),
            then_instructions,
            then_trace_key,
            else_instructions,
            else_trace_key,
            Some(trace_key),
        );

        self.add_instruction(Box::new(CloseScopeInstruction::new(
            if_error_reporter,
            if_scope_name,
        )));
    }

    /// Java `visitMacroStatement`.
    fn visit_macro_statement(&mut self, ctx: &MacroStatementContext) {
        if self.failed() {
            return;
        }
        let macro_id = ctx.var_id.text();
        let macro_block = ctx.block_statements.as_deref();
        let last_stmt_express = Self::macro_last_stmt_is_expression(macro_block);
        let macro_instructions = self.get_macro_instructions(macro_block);
        self.generator_scope.define_macro(
            macro_id,
            MacroDefine::new(macro_instructions, last_stmt_express),
        );

        if self.init_options.is_trace_expression() {
            let trace_key = ctx.var_id.start_token().map(Token::start_index);
            let reporter = ctx
                .var_id
                .start_token()
                .map(|t| self.new_reporter_with_token(t))
                .unwrap_or_else(|| Rc::new(PureErrReporter::INSTANCE));
            self.pure_add_instruction(Box::new(TraceEvaluatedInstruction::new(
                reporter, trace_key,
            )));
        }
    }

    /// Java `visitLocalVariableDeclaration`.
    fn visit_local_variable_declaration(&mut self, ctx: &LocalVariableDeclarationContext) {
        if self.failed() {
            return;
        }
        if self.init_options.is_trace_expression() {
            let trace_key = ctx.decl_type.start_token().map(Token::start_index);
            let reporter = self.reporter_of(&ctx.decl_type);
            self.add_instruction(Box::new(TraceEvaluatedInstruction::new(
                reporter, trace_key,
            )));
        }

        let decl_cls = self.parse_decl_type(&ctx.decl_type);
        let Node::VariableDeclaratorList(list) = &*ctx.variable_declarator_list else {
            return;
        };
        for variable_declarator in &list.variables {
            if self.failed() {
                return;
            }
            let Node::VariableDeclarator(declarator) = variable_declarator else {
                continue;
            };
            match &declarator.initializer {
                None => {
                    // Java `visitLocalVariableDeclaration` 无论声明类型为何，都
                    // 为缺省初始化器压入 `null`，再由 DefineLocal 执行转换。
                    let reporter = variable_declarator
                        .stop_token()
                        .map(|t| self.new_reporter_with_token(t))
                        .unwrap_or_else(|| Rc::new(PureErrReporter::INSTANCE));
                    self.add_instruction(Box::new(ConstInstruction::new(
                        reporter,
                        DataValue::Null,
                        None,
                    )));
                }
                Some(initializer) => self.parse_initializer(initializer, &decl_cls),
            }
            // Java `variableDeclaratorId().getStart()`: the variable id
            // token (not any trailing `[]` dims).
            let id_token = declarator.id.start_token().cloned();
            let reporter = id_token
                .as_ref()
                .map(|t| self.new_reporter_with_token(t))
                .unwrap_or_else(|| Rc::new(PureErrReporter::INSTANCE));
            let variable_name = id_token
                .as_ref()
                .map(|t| t.text().to_string())
                .unwrap_or_default();
            self.add_instruction(Box::new(DefineLocalInstruction::new(
                reporter,
                variable_name,
                Some(decl_cls.clone()),
            )));
        }
    }

    /// Java `visitTryCatchExpr`.
    fn visit_try_catch_expr(&mut self, ctx: &TryCatchExprContext) {
        if self.failed() {
            return;
        }
        let Some(block_statements) = &ctx.block_statements else {
            self.add_instruction(Box::new(ConstInstruction::new(
                self.new_reporter_with_token(ctx.try_token.symbol()),
                DataValue::NULL_VALUE,
                Some(ctx.try_token.symbol().start_index()),
            )));
            return;
        };

        let try_count = self.try_count();
        let try_scope_name = self.child_scope_name(&format!("{TRY_PREFIX}{try_count}"));
        let scope = self.child_scope(try_scope_name.clone());
        let body_sub = self.parse_with_sub_visitor(block_statements, scope, Context::Block);
        let body_max_stack = body_sub.max_stack_size();
        let body_instructions = body_sub.take_instructions();
        let body_lambda_definition: Rc<dyn QLambdaDefinition> = Rc::new(
            QLambdaDefinitionInner::new(try_scope_name, body_instructions, vec![], body_max_stack),
        );
        let exception_table = self.parse_exception_table(try_count, ctx.try_catches.as_deref());
        let final_body_definition =
            self.parse_final_body_definition(try_count, ctx.try_finally.as_deref());

        // Java 行为:try-catch 始终作为表达式。控制信号(Return/Break/
        // Continue(null))由 should_exit_try_catch 判断是否透传,
        // 不依赖 is_expression_form。catch body 含控制信号时,
        // 仍为表达式(is_expression_form=true),由 execute 内部
        // should_exit_try_catch 处理传播。
        self.add_instruction(Box::new(
            TryCatchInstruction::new(
                self.new_reporter_with_token(ctx.try_token.symbol()),
                body_lambda_definition,
                exception_table,
                final_body_definition,
            )
            .with_expression_form(true),
        ));
    }


    };
}
