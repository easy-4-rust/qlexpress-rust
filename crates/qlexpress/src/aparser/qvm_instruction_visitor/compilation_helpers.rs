/// Java `ClsTypeContext.varId()` children.
fn cls_type_children(cls_type: &Node) -> &[Node] {
    match cls_type {
        Node::ClsType(ctx) => &ctx.var_ids,
        _ => &[],
    }
}

/// Java `DimsContext.LBRACK().size()`.
fn dims_dim_count(dims: &Node) -> usize {
    match dims {
        Node::Dims(ctx) => ctx.dim_count(),
        _ => 0,
    }
}

/// Java `wrapInArray`: array class literals are represented by appending
/// `[]` to the Java-style name (Rust has no array class object; only used
/// for `MetaClass` constants and error messages).
fn wrap_in_array(base_type: ClassRef, layers: usize) -> ClassRef {
    let mut result = base_type;
    for _ in 0..layers {
        result = ClassRef::array_of(result);
    }
    result
}

/// Java `Object.class` (untyped declaration target).
fn object_cls() -> ClassRef {
    ClassRef::Named("java.lang.Object".to_string())
}

/// Java `blockExpr`: reduce `{ ... }` used as an expression body.
fn block_expr_of(expression: &Node) -> Option<&BlockExprContext> {
    let Node::Expression(ctx) = expression else {
        return None;
    };
    if ctx.is_assign() {
        return None;
    }
    // Java fast fail: start token `{` and stop token `}`.
    if expression.start_token().map(Token::text) != Some("{")
        || expression.stop_token().map(Token::text) != Some("}")
    {
        return None;
    }
    let ternary = ctx.ternary.as_deref()?;
    let Node::TernaryExpr(ternary_ctx) = ternary else {
        return None;
    };
    if ternary_ctx.question.is_some() {
        return None;
    }
    let Node::BaseExpr(base_expr) = &*ternary_ctx.condition else {
        return None;
    };
    if !base_expr.left_assos.is_empty() {
        return None;
    }
    let Node::Primary(primary) = &*base_expr.primary else {
        return None;
    };
    if primary.non_pathable.is_some() {
        return None;
    }
    // Java checks neither prefix/suffix nor path parts here.
    match primary.pathable.as_deref() {
        Some(Node::BlockExpr(block_expr)) => Some(block_expr),
        _ => None,
    }
}

// ---------------------------------------------------------------------------
// Statements
// ---------------------------------------------------------------------------

impl<'a> QvmInstructionVisitor<'a> {
    /// Java `handleStmt`: inline a macro call or compile the statement;
    /// returns whether the statement is an expression statement (its value
    /// remains on the stack).
    fn handle_stmt(&mut self, statement: &Node) -> bool {
        if self.maybe_macro_call(statement) {
            let macro_name = statement
                .start_token()
                .map(Token::text)
                .unwrap_or_default()
                .to_string();
            if let Some(macro_define) = self.generator_scope.get_macro_instructions(&macro_name) {
                if let Some(instructions) = macro_define.macro_instructions() {
                    for instruction in instructions.iter() {
                        self.pure_add_shared(Rc::clone(instruction));
                    }
                }
                self.add_timeout_instruction();
                return macro_define.is_last_stmt_express();
            }
        }
        statement.accept(self);
        matches!(statement, Node::ExpressionStatement(_))
    }

    /// Java `maybeMacroCall`: an expression statement consisting of a
    /// single `ID` token.
    fn maybe_macro_call(&self, statement: &Node) -> bool {
        let Node::ExpressionStatement(expr_stmt) = statement else {
            return false;
        };
        let expression = &expr_stmt.expression;
        let (Some(start), Some(stop)) = (expression.start_token(), expression.stop_token()) else {
            return false;
        };
        std::ptr::eq(start, stop) && start.token_type() == token::ID as i32
    }

    /// Java `generateForInitLambda`.
    fn generate_for_init_lambda(
        &mut self,
        for_count: i32,
        for_init: &Node,
    ) -> Option<Rc<QLambdaDefinitionInner>> {
        let Node::ForInit(ctx) = for_init else {
            return None;
        };
        if let Some(local_variable_declaration) = &ctx.local_variable_declaration {
            let scope_name =
                self.child_scope_name(&format!("{FOR_PREFIX}{for_count}{INIT_SUFFIX}"));
            let scope = self.child_scope(scope_name.clone());
            let sub =
                self.parse_with_sub_visitor(local_variable_declaration, scope, Context::Macro);
            let max_stack_size = sub.max_stack_size();
            let instructions = sub.take_instructions();
            Some(Rc::new(QLambdaDefinitionInner::new(
                scope_name,
                instructions,
                vec![],
                max_stack_size,
            )))
        } else {
            ctx.expression
                .as_ref()
                .and_then(|expr| self.generate_for_express_lambda(for_count, INIT_SUFFIX, expr))
        }
    }

    /// Java `generateForExpressLambda`.
    fn generate_for_express_lambda(
        &mut self,
        for_count: i32,
        scope_suffix: &str,
        expression: &Node,
    ) -> Option<Rc<QLambdaDefinitionInner>> {
        let scope_name = self.child_scope_name(&format!("{FOR_PREFIX}{for_count}{scope_suffix}"));
        let scope = self.child_scope(scope_name.clone());
        let sub = self.parse_expr_body_with_sub_visitor(expression, scope, Context::Block);
        let max_stack_size = sub.max_stack_size();
        let instructions = sub.take_instructions();
        Some(Rc::new(QLambdaDefinitionInner::new(
            scope_name,
            instructions,
            vec![],
            max_stack_size,
        )))
    }

    /// Java `parseFunctionDefinition`.
    fn parse_function_definition(
        &mut self,
        function_name: &str,
        block_statements: Option<&Node>,
        params: Vec<Param>,
    ) -> Rc<dyn QLambdaDefinition> {
        let Some(block_statements) = block_statements else {
            return Rc::new(QLambdaDefinitionInner::new(
                function_name,
                vec![],
                params,
                0,
            ));
        };
        let scope = self.child_scope(function_name);
        let sub = self.parse_with_sub_visitor(block_statements, scope, Context::Block);
        let max_stack_size = sub.max_stack_size();
        let instructions = sub.take_instructions();
        Rc::new(QLambdaDefinitionInner::new(
            function_name,
            instructions,
            params,
            max_stack_size,
        ))
    }

    /// Java `parseFormalOrInferredParameterList`.
    fn parse_formal_or_inferred_parameter_list(&mut self, params_node: &Node) -> Vec<Param> {
        let Node::FormalOrInferredParameterList(ctx) = params_node else {
            return vec![];
        };
        ctx.params
            .iter()
            .map(|param| self.formal_or_inferred_parameter_to_param(param))
            .collect()
    }

    /// Java `formalOrInferredParameter2Param`.
    fn formal_or_inferred_parameter_to_param(&mut self, param_node: &Node) -> Param {
        let Node::FormalOrInferredParameter(ctx) = param_node else {
            return Param::new("", None);
        };
        let param_name = ctx.var_id.text();
        let param_cls = ctx
            .decl_type
            .as_ref()
            .map(|decl| self.parse_decl_type(decl));
        Param::new(param_name, param_cls)
    }

    /// Java `parseLambdaParams`.
    fn parse_lambda_params(&mut self, lambda_parameters: &Node) -> Vec<Param> {
        let Node::LambdaParameters(ctx) = lambda_parameters else {
            return vec![];
        };
        if let Some(var_id) = &ctx.var_id {
            return vec![Param::new(var_id.text(), Some(object_cls()))];
        }
        match &ctx.params {
            Some(params) => self.parse_formal_or_inferred_parameter_list(params),
            None => vec![],
        }
    }

    /// Java `parseExceptionTable`.
    fn parse_exception_table(
        &mut self,
        try_count: i32,
        try_catches: Option<&Node>,
    ) -> Vec<(ClassRef, Rc<dyn QLambdaDefinition>)> {
        let mut exception_table = Vec::new();
        let Some(Node::TryCatches(catches_ctx)) = try_catches else {
            return exception_table;
        };
        for try_catch in &catches_ctx.catches {
            let Node::TryCatch(try_catch_ctx) = try_catch else {
                continue;
            };
            let Node::CatchParams(catch_params) = &*try_catch_ctx.catch_params else {
                continue;
            };
            let e_name = catch_params.var_id.text();
            let catch_body_name =
                self.child_scope_name(&format!("{TRY_PREFIX}{try_count}{CATCH_SUFFIX}"));
            let catch_sub = try_catch_ctx.block_statements.as_ref().map(|block| {
                let scope = self.child_scope(catch_body_name.clone());
                self.parse_with_sub_visitor(block, scope, Context::Block)
            });
            // Java compiles the body once and shares the instruction list
            // across the catch's declared types. The port moves the
            // compiled instructions into the first handler and recompiles
            // the body for each additional declared type (identical
            // instruction sequences, fresh objects).
            let mut compiled = catch_sub.map(|sub| (sub.max_stack_size(), sub.take_instructions()));
            let mut handler_for = |visitor: &mut Self, param: Param| -> Rc<dyn QLambdaDefinition> {
                match compiled.take() {
                    Some((max_stack, instructions)) => Rc::new(QLambdaDefinitionInner::new(
                        catch_body_name.clone(),
                        instructions,
                        vec![param],
                        max_stack,
                    )),
                    None => match &try_catch_ctx.block_statements {
                        None => Rc::new(QLambdaDefinitionEmpty::INSTANCE),
                        Some(block) => {
                            let scope = visitor.child_scope(catch_body_name.clone());
                            let sub = visitor.parse_with_sub_visitor(block, scope, Context::Block);
                            let max_stack = sub.max_stack_size();
                            Rc::new(QLambdaDefinitionInner::new(
                                catch_body_name.clone(),
                                sub.take_instructions(),
                                vec![param],
                                max_stack,
                            ))
                        }
                    },
                }
            };
            if catch_params.decl_types.is_empty() {
                let handler = handler_for(self, Param::new(e_name.clone(), Some(object_cls())));
                exception_table.push((object_cls(), handler));
            }
            for decl_type in &catch_params.decl_types {
                let exception_type = self.parse_decl_type(decl_type);
                let param = Param::new(e_name.clone(), Some(exception_type.clone()));
                let handler = handler_for(self, param);
                exception_table.push((exception_type, handler));
            }
        }
        exception_table
    }

    /// Java `parseFinalBodyDefinition`.
    fn parse_final_body_definition(
        &mut self,
        try_count: i32,
        try_finally: Option<&Node>,
    ) -> Option<Rc<dyn QLambdaDefinition>> {
        let Node::TryFinally(finally_ctx) = try_finally? else {
            return None;
        };
        let block_statements = finally_ctx.block_statements.as_ref()?;
        let final_scope_name =
            self.child_scope_name(&format!("{TRY_PREFIX}{try_count}{FINAL_SUFFIX}"));
        let scope = self.child_scope(final_scope_name.clone());
        let sub = self.parse_with_sub_visitor(block_statements, scope, Context::Block);
        let max_stack_size = sub.max_stack_size();
        let instructions = sub.take_instructions();
        Some(Rc::new(QLambdaDefinitionInner::new(
            final_scope_name,
            instructions,
            vec![],
            max_stack_size,
        )))
    }

    /// Java `ifBodyFillConst`.
    fn if_body_fill_const(
        expression: Option<&Node>,
        non_expression_statement: Option<&Node>,
        block_statements: Option<&Node>,
    ) -> bool {
        if expression.is_some() {
            return false;
        }
        if let Some(non_expression_statement) = non_expression_statement {
            return Self::non_expression_stmt_fill_const(non_expression_statement);
        }
        if let Some(block_statements) = block_statements {
            if let Node::BlockStatements(ctx) = block_statements {
                let statements: Vec<&Node> = ctx
                    .statements
                    .iter()
                    .filter(|bs| !matches!(bs, Node::EmptyStatement(_)))
                    .collect();
                return statements
                    .last()
                    .is_none_or(|last| Self::block_stmt_fill_const(last));
            }
            return true;
        }
        true
    }

    /// Java `stmtFillConst(NonExpressionStatementContext)`: fill unless the
    /// statement is a `return`.
    fn non_expression_stmt_fill_const(non_expression_statement: &Node) -> bool {
        let Node::NonExpressionStatement(ctx) = non_expression_statement else {
            return true;
        };
        !matches!(&*ctx.statement, Node::ReturnStatement(_))
    }

    /// Java `stmtFillConst(BlockStatementContext)`.
    fn block_stmt_fill_const(block_statement: &Node) -> bool {
        !matches!(
            block_statement,
            Node::ExpressionStatement(_) | Node::ReturnStatement(_)
        )
    }

    /// Java `getMacroLastStmt`: whether the macro's last statement is an
    /// expression statement.
    fn macro_last_stmt_is_expression(macro_block_statements: Option<&Node>) -> bool {
        let Some(Node::BlockStatements(ctx)) = macro_block_statements else {
            return false;
        };
        ctx.statements
            .iter()
            .rfind(|bs| !matches!(bs, Node::EmptyStatement(_)))
            .is_some_and(|last| matches!(last, Node::ExpressionStatement(_)))
    }

    /// Java `getMacroInstructions`.
    fn get_macro_instructions(
        &mut self,
        macro_block_statements: Option<&Node>,
    ) -> Vec<SharedInstruction> {
        let Some(block_statements) = macro_block_statements else {
            return vec![];
        };
        let scope_name = self.macro_scope_name();
        let scope = self.child_scope(scope_name);
        let sub = self.parse_with_sub_visitor(block_statements, scope, Context::Macro);
        sub.take_instructions().into_iter().map(Rc::from).collect()
    }

    /// Java `parseInitializer`.
    fn parse_initializer(&mut self, variable_initializer: &Node, decl_cls: &ClassRef) {
        let Node::VariableInitializer(ctx) = variable_initializer else {
            return;
        };
        if let Some(expression) = &ctx.expression {
            expression.accept(self);
            return;
        }
        if let Some(array_initializer) = &ctx.array_initializer {
            let component_cls = decl_cls
                .component_type()
                .unwrap_or_else(|| decl_cls.clone());
            self.new_arr_with_initializers(component_cls, array_initializer);
        }
    }

    /// Java `newArrWithInitializers`.
    fn new_arr_with_initializers(&mut self, component_cls: ClassRef, array_initializer: &Node) {
        let Node::ArrayInitializer(ctx) = array_initializer else {
            return;
        };
        let initializers: &[Node] = match ctx.initializers.as_deref() {
            Some(Node::VariableInitializerList(list)) => &list.initializers,
            _ => &[],
        };
        for initializer in initializers {
            initializer.accept(self);
        }
        let reporter = self.reporter_of(array_initializer);
        self.add_instruction(Box::new(NewArrayInstruction::new(
            reporter,
            component_cls,
            initializers.len(),
        )));
    }
}
