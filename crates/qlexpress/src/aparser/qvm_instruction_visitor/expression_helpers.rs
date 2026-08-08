// ---------------------------------------------------------------------------
// Expressions
// ---------------------------------------------------------------------------

impl<'a> QvmInstructionVisitor<'a> {
    /// Java `visitMethodInvokeInner`.
    fn visit_method_invoke_inner(
        &mut self,
        argument_list: Option<&Node>,
        method_name: &Node,
        optional: bool,
    ) {
        if let Some(argument_list) = argument_list {
            argument_list.accept(self);
        }
        let arg_num = argument_list.map_or(0, argument_count);
        let reporter = self.reporter_of(method_name);
        self.add_call_instruction(Box::new(MethodInvokeInstruction::new(
            reporter,
            method_name.text(),
            arg_num,
            optional,
        )));
    }

    /// Java `parseFieldId`.
    fn parse_field_id(field_id: &Node) -> String {
        if let Node::FieldId(ctx) = field_id {
            if let Some(quote) = ctx.quote_string_literal() {
                return QLStringUtils::parse_string_escape(quote.text())
                    .to_rust_string()
                    .expect("quoted Rust source must remain valid UTF-8");
            }
        }
        field_id
            .start_token()
            .map(Token::text)
            .unwrap_or_default()
            .to_string()
    }

    /// Java `parseDimParts`: count leading empty `[]` index parts.
    fn parse_dim_parts(&self, start: usize, path_parts: &[Node]) -> usize {
        let mut i = start;
        while i < path_parts.len() && is_empty_index(&path_parts[i]) {
            i += 1;
        }
        i - start
    }

    /// Java `parsePathHeadPart`.
    fn parse_path_head_part(&mut self, pathable: &Node, path_parts: &[Node]) -> usize {
        match pathable {
            Node::TypeExpr(_) => {
                let text = pathable.start_token().map(Token::text).unwrap_or_default();
                let cls = Self::built_in_cls(text).unwrap_or_else(object_cls);
                let dim_part_num = self.parse_dim_parts(0, path_parts);
                let cls = wrap_in_array(cls, dim_part_num);
                let reporter = self.reporter_of(pathable);
                self.add_instruction(Box::new(ConstInstruction::new(
                    reporter,
                    MetaClass::new(cls).into_data_value(),
                    None,
                )));
                dim_part_num
            }
            Node::VarIdExpr(id_context) => self.parse_id_head_part(
                &id_context.var_id,
                id_context.lparen.is_some(),
                id_context.argument_list.as_deref(),
                path_parts,
            ),
            _ => {
                pathable.accept(self);
                0
            }
        }
    }

    /// Java `parseIdHeadPart`.
    fn parse_id_head_part(
        &mut self,
        id_context: &Node,
        function_call: bool,
        argument_list: Option<&Node>,
        path_parts: &[Node],
    ) -> usize {
        if function_call {
            self.visit_call_function(id_context, argument_list);
            return 0;
        }
        let mut head_part_ids = vec![id_context.text()];
        for path_part in path_parts {
            match path_part {
                Node::FieldAccess(field_access) if field_access.chain == ChainKind::Plain => {
                    head_part_ids.push(Self::parse_field_id(&field_access.field_id));
                }
                _ => break,
            }
        }
        let result = self
            .import_manager
            .borrow()
            .load_part_qualified(&head_part_ids);
        match result.cls() {
            Some(cls) => {
                let cls = ClassRef::from_name(cls);
                let rest_index = result.rest_index() as i32 - 1;
                let report_token = if rest_index == 0 {
                    id_context.start_token()
                } else {
                    path_parts
                        .get((rest_index - 1).max(0) as usize)
                        .and_then(|p| p.stop_token())
                };
                let dim_part_num = self.parse_dim_parts(rest_index.max(0) as usize, path_parts);
                let cls = wrap_in_array(cls, dim_part_num);
                let reporter = report_token
                    .map(|t| self.new_reporter_with_token(t))
                    .unwrap_or_else(|| Rc::new(PureErrReporter::INSTANCE));
                self.add_instruction(Box::new(ConstInstruction::new(
                    reporter,
                    MetaClass::new(cls).into_data_value(),
                    None,
                )));
                rest_index.max(0) as usize + dim_part_num
            }
            None => {
                let reporter = self.reporter_of(id_context);
                let trace_key = id_context.start_token().map(Token::start_index);
                self.add_instruction(Box::new(LoadInstruction::new(
                    reporter,
                    id_context.text(),
                    trace_key,
                )));
                0
            }
        }
    }

    /// Java `visitCallFunction`.
    fn visit_call_function(&mut self, function_name_context: &Node, argument_list: Option<&Node>) {
        let function_name = function_name_context.text();
        let compile_time_function = self.compile_time_functions.get(&function_name).cloned();
        if let Some(compile_time_function) = compile_time_function {
            let function_token = function_name_context
                .start_token()
                .cloned()
                .unwrap_or_else(|| Token::new(0, "", 0, 0, 1, 0));
            let reporter = self.reporter_of(function_name_context);
            let arguments: Vec<&Node> = argument_list.map(argument_expressions).unwrap_or_default();
            let operator_factory = self.operator_factory;
            let mut code_generator = VisitorCodeGenerator {
                visitor: self,
                function_name: function_name.clone(),
                function_token,
                reporter,
            };
            compile_time_function.create_function_instruction(
                &function_name,
                &arguments,
                operator_factory,
                &mut code_generator,
            );
            return;
        }

        if let Some(arg_list) = argument_list {
            let lazy_flags: Option<Vec<bool>> = self
                .user_define_functions
                .get(&function_name)
                .and_then(|f| f.as_lazy_arg())
                .map(|lazy| {
                    (0..argument_count(arg_list))
                        .map(|i| lazy.is_lazy_arg(i))
                        .collect()
                });
            match lazy_flags {
                Some(flags) => {
                    let lazy_function_count = self.lazy_function_count();
                    for (i, expr) in argument_expressions(arg_list).iter().enumerate() {
                        if !flags[i] {
                            expr.accept(self);
                            continue;
                        }
                        let scope_name = self.child_scope_name(&format!(
                            "{LAZY_FUNCTION_PREFIX}{lazy_function_count}_{function_name}{i}"
                        ));
                        let scope = self.child_scope(scope_name.clone());
                        let lazy_visitor =
                            self.parse_expr_body_with_sub_visitor(expr, scope, Context::Block);
                        let max_stack_size = lazy_visitor.max_stack_size();
                        let instructions = lazy_visitor.take_instructions();
                        let lazy_lambda = QLambdaDefinitionInner::new(
                            scope_name,
                            instructions,
                            vec![],
                            max_stack_size,
                        );
                        let reporter = self.reporter_of(expr);
                        self.add_instruction(Box::new(LoadLambdaInstruction::new(
                            reporter,
                            Rc::new(lazy_lambda),
                        )));
                    }
                }
                None => arg_list.accept(self),
            }
        }
        let arg_size = argument_list.map_or(0, argument_count);
        let reporter = self.reporter_of(function_name_context);
        let trace_key = function_name_context.start_token().map(Token::start_index);
        self.add_call_instruction(Box::new(CallFunctionInstruction::new(
            reporter,
            function_name,
            arg_size,
            trace_key,
        )));
    }

    /// Java `visitListExprInner`.
    fn visit_list_expr_inner(
        &mut self,
        list_items: Option<&Node>,
        list_error_reporter: Rc<dyn ErrorReporter>,
    ) {
        let Some(Node::ListItems(ctx)) = list_items else {
            self.add_instruction(Box::new(NewListInstruction::new(list_error_reporter, 0)));
            return;
        };
        for expression in &ctx.expressions {
            expression.accept(self);
        }
        self.add_instruction(Box::new(NewListInstruction::new(
            list_error_reporter,
            ctx.expressions.len(),
        )));
    }

    /// Java `parseMapKey`.
    fn parse_map_key(&self, map_key: &Node) -> String {
        match map_key {
            Node::IdKey(_) => map_key.text(),
            Node::StringKey(_) | Node::QuoteStringKey(_) => {
                QLStringUtils::parse_string_escape(&map_key.text())
                    .to_rust_string()
                    .expect("quoted Rust source must remain valid UTF-8")
            }
            // shouldn't run here
            _ => panic!("unexpected map key node"),
        }
    }

    /// Java `visitSwitchStatement` (traditional `case X:` style).
    fn visit_switch_statement(&mut self, ctx: &SwitchExprContext) {
        // Evaluate switch expression once and store in temporary variable
        let switch_count = self.switch_count();
        let switch_var_name = format!("@switch_{switch_count}");
        let switch_key_token = ctx.switch_token.symbol();
        let switch_error_reporter = self.new_reporter_with_token(switch_key_token);

        // Create scope for switch
        let switch_scope_name = self.child_scope_name(&format!("SWITCH_{switch_count}"));
        self.add_instruction(Box::new(NewScopeInstruction::new(
            Rc::clone(&switch_error_reporter),
            switch_scope_name.clone(),
        )));

        // Evaluate and store switch expression
        ctx.expression.accept(self);
        self.add_instruction(Box::new(DefineLocalInstruction::new(
            Rc::clone(&switch_error_reporter),
            switch_var_name.clone(),
            Some(object_cls()),
        )));

        let groups: Vec<&SwitchStatementGroupContext> = switch_groups(ctx)
            .filter_map(|g| match g {
                Node::SwitchStatementGroup(group) => Some(group),
                _ => None,
            })
            .collect();

        // Collect case bodies and metadata
        let mut case_bodies: Vec<Vec<Instruction>> = Vec::new();
        let mut case_breaks: Vec<Vec<usize>> = Vec::new();
        let mut case_trace_keys: Vec<Option<i32>> = Vec::new();
        let mut case_conditions: Vec<Vec<&Node>> = Vec::new();
        let mut default_index: i32 = -1;

        for (i, group) in groups.iter().enumerate() {
            let mut conditions = Vec::new();
            if let Node::SwitchLabels(labels_ctx) = &*group.labels {
                for label in &labels_ctx.labels {
                    if let Node::SwitchLabel(label_ctx) = label {
                        if label_ctx.case_token.is_some() {
                            if let Some(expr) = &label_ctx.expression {
                                conditions.push(&**expr);
                            }
                        } else if label_ctx.default_token.is_some() {
                            default_index = i as i32;
                        }
                    }
                }
            }
            case_conditions.push(conditions);
            case_trace_keys.push(
                group
                    .block_statements
                    .as_deref()
                    .and_then(Node::start_token)
                    .map(Token::start_index),
            );

            // Generate case body instructions; record top-level `break`s
            // (Java finds them with `instanceof` afterwards).
            let (body_instructions, break_indices) = match &group.block_statements {
                None => (Vec::new(), Vec::new()),
                Some(body) => {
                    let mut body_visitor =
                        self.sub_visitor(Rc::clone(&self.generator_scope), Context::Macro);
                    body_visitor.collect_break_indices = Some(Vec::new());
                    body.accept(&mut body_visitor);
                    self.propagate_error(&body_visitor);
                    let breaks = body_visitor
                        .collect_break_indices
                        .take()
                        .unwrap_or_default();
                    (body_visitor.take_instructions(), breaks)
                }
            };
            case_bodies.push(body_instructions);
            case_breaks.push(break_indices);
        }

        // Generate comparison and jump logic
        let mut case_jumps: Vec<(Rc<JumpIfPopInstruction>, usize)> = Vec::new();

        for conditions in &case_conditions {
            for cond in conditions {
                // Load switch value
                self.add_instruction(Box::new(LoadInstruction::new(
                    Rc::clone(&switch_error_reporter),
                    switch_var_name.clone(),
                    None,
                )));
                // Evaluate case expression
                cond.accept(self);
                // Compare for equality using == operator
                let equals_op = self
                    .operator_factory
                    .get_binary_operator("==")
                    .expect("'==' operator must exist");
                self.add_instruction(Box::new(OperatorInstruction::new(
                    Rc::clone(&switch_error_reporter),
                    equals_op,
                    None,
                )));

                // If equal (result is true), jump to case body
                let jump_to_case = Rc::new(JumpIfPopInstruction::new(
                    Rc::clone(&switch_error_reporter),
                    true,
                    -1,
                ));
                self.pure_add_shared(Rc::clone(&jump_to_case) as SharedInstruction);
                case_jumps.push((jump_to_case, self.instruction_list.len() - 1));
            }
        }

        // No match, jump to default or end
        let jump_to_default_or_end =
            Rc::new(JumpInstruction::new(Rc::clone(&switch_error_reporter), -1));
        self.pure_add_shared(Rc::clone(&jump_to_default_or_end) as SharedInstruction);
        let jump_to_default_start_pos = self.instruction_list.len();

        // Generate case bodies
        let mut break_jumps: Vec<(Rc<JumpInstruction>, usize)> = Vec::new();
        let mut case_jump_index = 0;

        for (i, body) in case_bodies.into_iter().enumerate() {
            // Set jump targets for this case
            let num_conditions = case_conditions[i].len();
            let case_start_pos = self.instruction_list.len();

            for _ in 0..num_conditions {
                if case_jump_index < case_jumps.len() {
                    let (jump, jump_instr_pos) = &case_jumps[case_jump_index];
                    // Position should be relative to the instruction AFTER
                    // the JumpIfPop
                    let jump_start = jump_instr_pos + 1;
                    jump.set_position((case_start_pos - jump_start) as i32);
                    case_jump_index += 1;
                }
            }

            // Set default jump target
            if i as i32 == default_index {
                jump_to_default_or_end
                    .set_position((case_start_pos - jump_to_default_start_pos) as i32);
            }

            // Java 的 switch trace 将被选中的 statement group 记录为
            // `BLOCK ... null`，包括以 break 结束、没有产生表达式值的分支。
            // case 体本身不是 BlockExpr，不会生成 TracePeek，因此在进入
            // 分支时显式标记其 block trace 已执行。
            if self.init_options.is_trace_expression() {
                if let Some(trace_key) = case_trace_keys[i] {
                    self.pure_add_instruction(Box::new(TraceEvaluatedInstruction::new(
                        Rc::clone(&switch_error_reporter),
                        Some(trace_key),
                    )));
                }
            }

            // Add case body, replacing top-level `break` with a jump to
            // the end of the switch (Java behaviour).
            for (idx, instruction) in body.into_iter().enumerate() {
                if case_breaks[i].contains(&idx) {
                    let break_jump =
                        Rc::new(JumpInstruction::new(Rc::clone(&switch_error_reporter), -1));
                    self.pure_add_shared(Rc::clone(&break_jump) as SharedInstruction);
                    break_jumps.push((break_jump, self.instruction_list.len() - 1));
                } else {
                    self.pure_add_instruction(instruction);
                }
            }
        }

        // Set end position
        let end_position = self.instruction_list.len();

        // Fix up break jumps
        for (break_jump, break_jump_pos) in break_jumps {
            break_jump.set_position((end_position - break_jump_pos - 1) as i32);
        }

        // If no default, set jump to end
        if default_index == -1 {
            jump_to_default_or_end.set_position((end_position - jump_to_default_start_pos) as i32);
        }

        // If no case matched and no explicit return, push null
        let needs_default_value = if default_index >= 0 {
            let default_body = &groups[default_index as usize].block_statements;
            !last_stmt_is_return_or_break(default_body.as_deref())
        } else {
            true
        };
        if needs_default_value {
            self.add_instruction(Box::new(ConstInstruction::new(
                Rc::clone(&switch_error_reporter),
                DataValue::NULL_VALUE,
                None,
            )));
        }
        if self.init_options.is_trace_expression() {
            self.pure_add_instruction(Box::new(TracePeekInstruction::new(
                Rc::clone(&switch_error_reporter),
                Some(switch_key_token.start_index()),
            )));
        }

        self.add_instruction(Box::new(CloseScopeInstruction::new(
            switch_error_reporter,
            switch_scope_name,
        )));
    }

    /// Java `visitSwitchExpression` (`case X -> expr` style).
    fn visit_switch_expression(&mut self, ctx: &SwitchExprContext) {
        // Evaluate switch expression once and store in temporary variable
        let switch_count = self.switch_count();
        let switch_var_name = format!("@switch_expr_{switch_count}");
        let switch_key_token = ctx.switch_token.symbol();
        let switch_error_reporter = self.new_reporter_with_token(switch_key_token);

        // Create scope for switch expression
        let switch_scope_name = self.child_scope_name(&format!("SWITCH_EXPR_{switch_count}"));
        self.add_instruction(Box::new(NewScopeInstruction::new(
            Rc::clone(&switch_error_reporter),
            switch_scope_name.clone(),
        )));

        // Evaluate and store switch expression
        ctx.expression.accept(self);
        self.add_instruction(Box::new(DefineLocalInstruction::new(
            Rc::clone(&switch_error_reporter),
            switch_var_name.clone(),
            Some(object_cls()),
        )));

        let groups: Vec<&SwitchExprGroupContext> = switch_groups(ctx)
            .filter_map(|g| match g {
                Node::SwitchExprGroup(group) => Some(group),
                _ => None,
            })
            .collect();

        // Generate jump instructions for each case
        let mut case_jumps: Vec<(Rc<JumpIfPopInstruction>, usize)> = Vec::new();
        let mut default_index: i32 = -1;

        // First pass: generate comparisons and collect metadata
        for (i, group) in groups.iter().enumerate() {
            if let Node::SwitchExpressionLabel(label) = &*group.label {
                if label.case_token.is_some() {
                    if let Some(expr_list) = &label.expression_list {
                        if let Node::ExpressionList(list_ctx) = &**expr_list {
                            for case_value in &list_ctx.expressions {
                                // Load switch value
                                self.add_instruction(Box::new(LoadInstruction::new(
                                    Rc::clone(&switch_error_reporter),
                                    switch_var_name.clone(),
                                    None,
                                )));
                                // Evaluate case expression
                                case_value.accept(self);
                                // Compare for equality using == operator
                                let equals_op = self
                                    .operator_factory
                                    .get_binary_operator("==")
                                    .expect("'==' operator must exist");
                                self.add_instruction(Box::new(OperatorInstruction::new(
                                    Rc::clone(&switch_error_reporter),
                                    equals_op,
                                    None,
                                )));

                                // If equal (result is true), jump to case
                                // body
                                let jump_to_case = Rc::new(JumpIfPopInstruction::new(
                                    Rc::clone(&switch_error_reporter),
                                    true,
                                    -1,
                                ));
                                self.pure_add_shared(Rc::clone(&jump_to_case) as SharedInstruction);
                                case_jumps.push((jump_to_case, self.instruction_list.len() - 1));
                            }
                        }
                    }
                } else if label.default_token.is_some() {
                    default_index = i as i32;
                }
            }
        }

        // No match, jump to default or error
        let jump_to_default_or_error =
            Rc::new(JumpInstruction::new(Rc::clone(&switch_error_reporter), -1));
        self.pure_add_shared(Rc::clone(&jump_to_default_or_error) as SharedInstruction);
        let jump_to_default_start_pos = self.instruction_list.len();

        // Second pass: generate case bodies
        let mut end_jumps: Vec<(Rc<JumpInstruction>, usize)> = Vec::new();
        let mut case_jump_index = 0;

        for (i, group) in groups.iter().enumerate() {
            let Node::SwitchExpressionLabel(label) = &*group.label else {
                continue;
            };

            // Set jump targets for this case
            let case_start_pos = self.instruction_list.len();

            if label.case_token.is_some() {
                let num_case_values = match label.expression_list.as_deref() {
                    Some(Node::ExpressionList(list_ctx)) => list_ctx.expressions.len(),
                    _ => 0,
                };
                for _ in 0..num_case_values {
                    if case_jump_index < case_jumps.len() {
                        let (jump, jump_instr_pos) = &case_jumps[case_jump_index];
                        let jump_start = jump_instr_pos + 1;
                        jump.set_position((case_start_pos - jump_start) as i32);
                        case_jump_index += 1;
                    }
                }
            } else if label.default_token.is_some() && i as i32 == default_index {
                // Set default jump target
                jump_to_default_or_error
                    .set_position((case_start_pos - jump_to_default_start_pos) as i32);
            }

            // Evaluate result expression for this case
            group.expression.accept(self);

            // Jump to end after evaluating result
            let jump_to_end = Rc::new(JumpInstruction::new(Rc::clone(&switch_error_reporter), -1));
            self.pure_add_shared(Rc::clone(&jump_to_end) as SharedInstruction);
            end_jumps.push((jump_to_end, self.instruction_list.len() - 1));
        }

        // Set end position
        let end_position = self.instruction_list.len();

        // Fix up all end jumps
        for (end_jump, end_jump_pos) in end_jumps {
            end_jump.set_position((end_position - end_jump_pos - 1) as i32);
        }

        // If no default, jump to end with null (should not happen in
        // well-formed switch expressions)
        if default_index == -1 {
            jump_to_default_or_error
                .set_position((end_position - jump_to_default_start_pos) as i32);
            self.add_instruction(Box::new(ConstInstruction::new(
                Rc::clone(&switch_error_reporter),
                DataValue::NULL_VALUE,
                None,
            )));
        }

        if self.init_options.is_trace_expression() {
            self.pure_add_instruction(Box::new(TracePeekInstruction::new(
                Rc::clone(&switch_error_reporter),
                Some(switch_key_token.start_index()),
            )));
        }

        self.add_instruction(Box::new(CloseScopeInstruction::new(
            switch_error_reporter,
            switch_scope_name,
        )));
    }
}
