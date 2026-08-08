impl<'a> QvmInstructionVisitor<'a> {
    /// 创建对象实例。
    /// 参数：`script`、`import_manager`、`global_scope`、`operator_factory`、`compile_time_functions`、`user_define_functions`、`init_options`；返回：`Self`。
    /// 对应或承接 Java 源文件：`com/alibaba/qlexpress4/aparser/QvmInstructionVisitor.java`，构造器 `<init>`；Rust 侧按所有权与 `Result` 语义适配。
    /// Java main constructor: `new QvmInstructionVisitor(script,
    /// importManager, globalScope, operatorFactory, compileTimeFunctions,
    /// userDefineFunctions, initOptions)`.
    /// 对应 Java: com.alibaba.qlexpress4.aparser.QvmInstructionVisitor#new。
    pub fn new(
        script: &'a str,
        import_manager: &'a RefCell<ImportManager<'a>>,
        global_scope: Option<Rc<InstructionScope>>,
        operator_factory: &'a dyn OperatorFactory,
        compile_time_functions: &'a CompileTimeFunctions,
        user_define_functions: &'a UserDefineFunctions,
        init_options: &'a InitOptions,
    ) -> Self {
        Self::for_recursion(
            script,
            import_manager,
            Rc::new(GeneratorScope::new("main", global_scope)),
            operator_factory,
            Context::Block,
            compile_time_functions,
            user_define_functions,
            init_options,
        )
    }

    /// 附加 context 配置并返回新值。
    /// 参数：`script`、`import_manager`、`generator_scope`、`operator_factory`、`context`、`compile_time_functions`、`user_define_functions`、`init_options`；返回：`Self`。
    /// 对应或承接 Java 源文件：`com/alibaba/qlexpress4/aparser/QvmInstructionVisitor.java`，方法 `withContext`；Rust 侧按所有权与 `Result` 语义适配。
    /// Java macro constructor: `new QvmInstructionVisitor(script,
    /// importManager, generatorScope, operatorFactory, context,
    /// compileTimeFunctions, userDefineFunctions, initOptions)` —
    /// used by `Express4Runner.parseMacroDefine` with `Context.MACRO`.
    #[allow(clippy::too_many_arguments)]
    /// 对应 Java: com.alibaba.qlexpress4.aparser.QvmInstructionVisitor#withContext。
    pub fn with_context(
        script: &'a str,
        import_manager: &'a RefCell<ImportManager<'a>>,
        generator_scope: Rc<InstructionScope>,
        operator_factory: &'a dyn OperatorFactory,
        context: Context,
        compile_time_functions: &'a CompileTimeFunctions,
        user_define_functions: &'a UserDefineFunctions,
        init_options: &'a InitOptions,
    ) -> Self {
        Self::for_recursion(
            script,
            import_manager,
            generator_scope,
            operator_factory,
            context,
            compile_time_functions,
            user_define_functions,
            init_options,
        )
    }

    /// Java recursion constructor.
    #[expect(
        clippy::too_many_arguments,
        reason = "对应 Java QvmInstructionVisitor 递归构造器的参数契约"
    )]
    fn for_recursion(
        script: &'a str,
        import_manager: &'a RefCell<ImportManager<'a>>,
        generator_scope: Rc<InstructionScope>,
        operator_factory: &'a dyn OperatorFactory,
        context: Context,
        compile_time_functions: &'a CompileTimeFunctions,
        user_define_functions: &'a UserDefineFunctions,
        init_options: &'a InitOptions,
    ) -> Self {
        QvmInstructionVisitor {
            script,
            import_manager,
            generator_scope,
            operator_factory,
            compile_time_functions,
            user_define_functions,
            init_options,
            context,
            instruction_list: Vec::new(),
            stack_size: 0,
            max_stack_size: 0,
            if_counter: 0,
            switch_counter: 0,
            block_counter: 0,
            macro_counter: 0,
            lambda_counter: 0,
            lazy_function_counter: 0,
            try_counter: 0,
            for_counter: 0,
            while_counter: 0,
            timeout_check_point: -1,
            syntax_error: None,
            last_is_timeout_check: false,
            collect_break_indices: None,
        }
    }

    /// 返回已经生成的 QVM 指令只读切片。
    /// 无显式参数；返回：`&[Instruction]`。
    /// 对应或承接 Java 源文件：`com/alibaba/qlexpress4/runtime/instruction/QLInstruction.java`，方法 `instructions`；Rust 侧按所有权与 `Result` 语义适配。
    /// Java `getInstructions`.
    /// 对应 Java: com.alibaba.qlexpress4.aparser.QvmInstructionVisitor#instructions。
    pub fn instructions(&self) -> &[Instruction] {
        &self.instruction_list
    }

    /// 移出并返回已经生成的全部 QVM 指令。
    /// 无显式参数；返回：`Vec<Instruction>`。
    /// 对应或承接 Java 源文件：`com/alibaba/qlexpress4/aparser/QvmInstructionVisitor.java`，方法 `takeInstructions`；Rust 侧按所有权与 `Result` 语义适配。
    /// Take the compiled instruction list (sub-visitor handover).
    /// 对应 Java: com.alibaba.qlexpress4.aparser.QvmInstructionVisitor#takeInstructions。
    pub fn take_instructions(self) -> Vec<Instruction> {
        self.instruction_list
    }

    /// 返回编译结果所需的最大操作数栈深。
    /// 无显式参数；返回：`usize`。
    /// 对应或承接 Java 源文件：`com/alibaba/qlexpress4/aparser/QvmInstructionVisitor.java`，方法 `maxStackSize`；Rust 侧按所有权与 `Result` 语义适配。
    /// Java `getMaxStackSize`.
    /// 对应 Java: com.alibaba.qlexpress4.aparser.QvmInstructionVisitor#maxStackSize。
    pub fn max_stack_size(&self) -> usize {
        self.max_stack_size as usize
    }

    /// 按当前源码位置构造编译期语法错误。
    /// 无显式参数；返回：`Option<&QLSyntaxException>`。
    /// 对应或承接 Java 源文件：`com/alibaba/qlexpress4/aparser/QvmInstructionVisitor.java`，方法 `syntaxError`；Rust 侧按所有权与 `Result` 语义适配。
    /// The first recorded syntax error, if any (Java: thrown).
    /// 对应 Java: com.alibaba.qlexpress4.aparser.QvmInstructionVisitor#syntaxError。
    pub fn syntax_error(&self) -> Option<&QLSyntaxException> {
        self.syntax_error.as_ref()
    }

    /// 把语法树编译为可执行 QVM 指令。
    /// 参数：`tree`；返回：`Result<(Vec<Instruction>, usize), QLSyntaxException>`。
    /// 对应或承接 Java 源文件：`com/alibaba/qlexpress4/aparser/QvmInstructionVisitor.java`，方法 `compile`；Rust 侧按所有权与 `Result` 语义适配。
    /// Compile `tree` and return the instructions plus max stack size, or
    /// the first syntax error (Java: the exception unwinds `accept`).
    /// 对应 Java: com.alibaba.qlexpress4.aparser.QvmInstructionVisitor#compile。
    pub fn compile(mut self, tree: &Node) -> Result<(Vec<Instruction>, usize), QLSyntaxException> {
        tree.accept(&mut self);
        match self.syntax_error {
            Some(err) => Err(err),
            None => Ok((self.instruction_list, self.max_stack_size as usize)),
        }
    }

    // ------------------------------------------------------------------
    // Error plumbing (Java throws QLSyntaxException)
    // ------------------------------------------------------------------

    /// Stop emitting once an error was recorded.
    fn failed(&self) -> bool {
        self.syntax_error.is_some()
    }

    /// Java `reportParseErr`.
    fn report_parse_err(
        &mut self,
        token: &Token,
        err_code: &str,
        err_reason: &str,
    ) -> QLSyntaxException {
        let err = QLException::report_scanner_err(
            self.script,
            token.start_index(),
            token.line(),
            token.char_position_in_line() + 1,
            token.text(),
            err_code,
            err_reason,
        );
        if self.syntax_error.is_none() {
            self.syntax_error = Some(err.clone());
        }
        err
    }

    /// Absorb a sub-visitor's error (Java: the exception propagates out of
    /// the recursive `accept`).
    fn propagate_error(&mut self, sub: &QvmInstructionVisitor<'a>) {
        if self.syntax_error.is_none() {
            if let Some(err) = sub.syntax_error() {
                self.syntax_error = Some(err.clone());
            }
        }
    }

    // ------------------------------------------------------------------
    // Sub-visitor plumbing (Java `parseWithSubVisitor` etc.)
    // ------------------------------------------------------------------

    /// Java `parseWithSubVisitor`.
    fn parse_with_sub_visitor(
        &mut self,
        node: &Node,
        generator_scope: Rc<InstructionScope>,
        context: Context,
    ) -> QvmInstructionVisitor<'a> {
        let mut sub = self.sub_visitor(generator_scope, context);
        node.accept(&mut sub);
        self.propagate_error(&sub);
        sub
    }

    /// Java `parseExprBodyWithSubVisitor`.
    fn parse_expr_body_with_sub_visitor(
        &mut self,
        expression: &Node,
        generator_scope: Rc<InstructionScope>,
        context: Context,
    ) -> QvmInstructionVisitor<'a> {
        let mut sub = self.sub_visitor(generator_scope, context);
        // reduce the level of syntax tree when expression is a block
        sub.visit_body_expression(expression);
        self.propagate_error(&sub);
        sub
    }

    fn sub_visitor(
        &self,
        generator_scope: Rc<InstructionScope>,
        context: Context,
    ) -> QvmInstructionVisitor<'a> {
        QvmInstructionVisitor::for_recursion(
            self.script,
            self.import_manager,
            generator_scope,
            self.operator_factory,
            context,
            self.compile_time_functions,
            self.user_define_functions,
            self.init_options,
        )
    }

    /// Java `visitBodyExpression`: when the body is a bare block `{ ... }`
    /// compile the block inline; otherwise compile the expression and add
    /// a `RETURN` instruction.
    fn visit_body_expression(&mut self, expression: &Node) {
        if self.failed() {
            return;
        }
        if let Some(block_expr) = block_expr_of(expression) {
            if let Some(block_statements) = &block_expr.block_statements {
                block_statements.accept(self);
            }
            return;
        }
        expression.accept(self);
        let reporter = expression
            .start_token()
            .map(|t| self.new_reporter_with_token(t))
            .unwrap_or_else(|| Rc::new(PureErrReporter::INSTANCE));
        self.add_instruction(Box::new(ReturnInstruction::new(
            reporter,
            ReturnResultType::Return,
            None,
        )));
    }

    // ------------------------------------------------------------------
    // Instruction emission (Java pureAddInstruction/addInstruction/
    // addTimeoutInstruction/expandStackSize)
    // ------------------------------------------------------------------

    /// Java `pureAddInstruction`.
    fn pure_add_instruction(&mut self, instruction: Instruction) {
        let stack_expand_size = instruction.stack_output() - instruction.stack_input();
        self.expand_stack_size(stack_expand_size);
        self.last_is_timeout_check = false;
        self.instruction_list.push(instruction);
    }

    /// Push a shared instruction (Java shares the same object, e.g. macro
    /// bodies and back-patched jumps).
    fn pure_add_shared(&mut self, instruction: SharedInstruction) {
        let stack_expand_size = instruction.stack_output() - instruction.stack_input();
        self.expand_stack_size(stack_expand_size);
        self.last_is_timeout_check = false;
        self.instruction_list.push(Box::new(instruction));
    }

    /// Java `addInstruction` for a regular (non-call) instruction.
    fn add_instruction(&mut self, instruction: Instruction) {
        self.add_instruction_inner(instruction, false);
    }

    /// Java `addInstruction` for `MethodInvokeInstruction` /
    /// `CallFunctionInstruction` / `CallConstInstruction` /
    /// `CallInstruction` (a timeout check follows).
    fn add_call_instruction(&mut self, instruction: Instruction) {
        self.add_instruction_inner(instruction, true);
    }

    fn add_instruction_inner(&mut self, instruction: Instruction, is_call: bool) {
        if self.instruction_list.len() as i32 - self.timeout_check_point > TIMEOUT_CHECK_GAP {
            self.add_timeout_instruction();
        }
        self.pure_add_instruction(instruction);
        if is_call {
            self.add_timeout_instruction();
        }
    }

    /// Java `addTimeoutInstruction`.
    fn add_timeout_instruction(&mut self) {
        if self.last_is_timeout_check {
            return;
        }
        let Some(last_instruction) = self.instruction_list.last() else {
            return;
        };
        let reporter = Rc::clone(last_instruction.error_reporter());
        self.timeout_check_point = self.instruction_list.len() as i32;
        self.last_is_timeout_check = true;
        self.instruction_list
            .push(Box::new(CheckTimeOutInstruction::new(reporter)));
    }

    /// Java `expandStackSize`.
    fn expand_stack_size(&mut self, stack_expand_size: i32) {
        self.stack_size += stack_expand_size;
        if self.stack_size > self.max_stack_size {
            self.max_stack_size = self.stack_size;
        }
    }

    // ------------------------------------------------------------------
    // Reporters and counters
    // ------------------------------------------------------------------

    /// Java `newReporterWithToken`.
    fn new_reporter_with_token(&self, token: &Token) -> Rc<dyn ErrorReporter> {
        Rc::new(DefaultErrReporter::new(
            self.script,
            token.start_index(),
            token.line(),
            token.char_position_in_line() + 1,
            token.text(),
        ))
    }

    fn reporter_of(&self, node: &Node) -> Rc<dyn ErrorReporter> {
        node.start_token()
            .map(|t| self.new_reporter_with_token(t))
            .unwrap_or_else(|| Rc::new(PureErrReporter::INSTANCE))
    }

    fn while_count(&mut self) -> i32 {
        let count = self.while_counter;
        self.while_counter += 1;
        count
    }

    fn for_count(&mut self) -> i32 {
        let count = self.for_counter;
        self.for_counter += 1;
        count
    }

    fn if_count(&mut self) -> i32 {
        let count = self.if_counter;
        self.if_counter += 1;
        count
    }

    fn switch_count(&mut self) -> i32 {
        let count = self.switch_counter;
        self.switch_counter += 1;
        count
    }

    fn lazy_function_count(&mut self) -> i32 {
        let count = self.lazy_function_counter;
        self.lazy_function_counter += 1;
        count
    }

    fn try_count(&mut self) -> i32 {
        let count = self.try_counter;
        self.try_counter += 1;
        count
    }

    fn block_scope_name(&mut self) -> String {
        let name = format!(
            "{}{}{}{}",
            self.generator_scope.name(),
            SCOPE_SEPARATOR,
            BLOCK_LAMBDA_NAME_PREFIX,
            self.block_counter
        );
        self.block_counter += 1;
        name
    }

    fn macro_scope_name(&mut self) -> String {
        let name = format!(
            "{}{}{}{}",
            self.generator_scope.name(),
            SCOPE_SEPARATOR,
            MACRO_PREFIX,
            self.macro_counter
        );
        self.macro_counter += 1;
        name
    }

    fn lambda_scope_name(&mut self) -> String {
        let name = format!(
            "{}{}{}{}",
            self.generator_scope.name(),
            SCOPE_SEPARATOR,
            LAMBDA_PREFIX,
            self.lambda_counter
        );
        self.lambda_counter += 1;
        name
    }

    /// Java `generatorScope.getName() + SCOPE_SEPARATOR + ...`.
    fn child_scope_name(&self, stem: &str) -> String {
        format!("{}{}{}", self.generator_scope.name(), SCOPE_SEPARATOR, stem)
    }

    fn child_scope(&self, name: impl Into<String>) -> Rc<InstructionScope> {
        Rc::new(GeneratorScope::new(
            name,
            Some(Rc::clone(&self.generator_scope)),
        ))
    }

    // ------------------------------------------------------------------
    // Shared compile helpers
    // ------------------------------------------------------------------

    /// Java `ifElseInstructions`.
    fn if_else_instructions(
        &mut self,
        condition_reporter: Rc<dyn ErrorReporter>,
        then_instructions: Vec<Instruction>,
        then_trace_key: Option<i32>,
        else_instructions: Vec<Instruction>,
        else_trace_key: Option<i32>,
        trace_key: Option<i32>,
    ) {
        let jump_if = Rc::new(JumpIfPopInstruction::new(
            Rc::clone(&condition_reporter),
            false,
            -1,
        ));
        self.pure_add_shared(Rc::clone(&jump_if) as SharedInstruction);
        let mut jump_start = self.instruction_list.len();
        for instruction in then_instructions {
            self.pure_add_instruction(instruction);
        }
        if self.init_options.is_trace_expression() {
            if then_trace_key.is_some() {
                self.pure_add_instruction(Box::new(TracePeekInstruction::new(
                    Rc::clone(&condition_reporter),
                    then_trace_key,
                )));
            }
            self.pure_add_instruction(Box::new(TracePeekInstruction::new(
                Rc::clone(&condition_reporter),
                trace_key,
            )));
        }
        self.add_timeout_instruction();

        let jump = Rc::new(JumpInstruction::new(Rc::clone(&condition_reporter), -1));
        self.pure_add_shared(Rc::clone(&jump) as SharedInstruction);

        jump_if.set_position((self.instruction_list.len() - jump_start) as i32);

        jump_start = self.instruction_list.len();
        for instruction in else_instructions {
            self.pure_add_instruction(instruction);
        }
        if self.init_options.is_trace_expression() {
            if else_trace_key.is_some() {
                self.pure_add_instruction(Box::new(TracePeekInstruction::new(
                    Rc::clone(&condition_reporter),
                    else_trace_key,
                )));
            }
            self.pure_add_instruction(Box::new(TracePeekInstruction::new(
                Rc::clone(&condition_reporter),
                trace_key,
            )));
        }
        self.add_timeout_instruction();
        jump.set_position((self.instruction_list.len() - jump_start) as i32);
    }

    /// Java `jumpRightIfExpect` (short-circuit `&&` / `||`).
    fn jump_right_if_expect(
        &mut self,
        expect: bool,
        op_err_reporter: Rc<dyn ErrorReporter>,
        right: &Node,
        operator_id: &str,
        trace_key: Option<i32>,
    ) {
        let right_visitor =
            self.parse_with_sub_visitor(right, Rc::clone(&self.generator_scope), Context::Macro);
        let right_instructions = right_visitor.take_instructions();

        let jump_if = Rc::new(JumpIfInstruction::new(
            Rc::clone(&op_err_reporter),
            expect,
            -1,
            trace_key,
        ));
        self.pure_add_shared(Rc::clone(&jump_if) as SharedInstruction);

        let jump_start = self.instruction_list.len();

        for instruction in right_instructions {
            self.pure_add_instruction(instruction);
        }
        let binary_operator = self
            .operator_factory
            .get_binary_operator(operator_id)
            .expect("short-circuit operator must exist");
        self.add_instruction(Box::new(OperatorInstruction::new(
            op_err_reporter,
            binary_operator,
            trace_key,
        )));
        self.add_timeout_instruction();

        jump_if.set_position((self.instruction_list.len() - jump_start) as i32);
    }

    /// Java `loopBodyVisitorDefinition`.
    fn loop_body_visitor_definition(
        &mut self,
        body: Option<&Node>,
        scope_name: String,
        params_type: Vec<Param>,
        error_reporter: Rc<dyn ErrorReporter>,
    ) -> (Rc<dyn QLambdaDefinition>, Option<usize>) {
        let Some(body) = body else {
            return (Rc::new(QLambdaDefinitionEmpty::INSTANCE), None);
        };
        let body_scope = self.child_scope(scope_name.clone());
        let body_visitor = self.parse_with_sub_visitor(body, body_scope, Context::Macro);
        let max_stack_size = body_visitor.max_stack_size();
        let body_instructions = body_visitor.take_instructions();

        let mut result_instructions: Vec<Instruction> = Vec::new();
        result_instructions.push(Box::new(CheckTimeOutInstruction::new(error_reporter)));
        result_instructions.extend(body_instructions);

        (
            Rc::new(QLambdaDefinitionInner::new(
                scope_name,
                result_instructions,
                params_type,
                max_stack_size,
            )),
            Some(max_stack_size),
        )
    }

    // ------------------------------------------------------------------
    // Types (Java parseDeclType/parseClsIds/BuiltInTypesSet/wrapInArray)
    // ------------------------------------------------------------------

    /// Java `BuiltInTypesSet.getCls`.
    fn built_in_cls(lexeme: &str) -> Option<ClassRef> {
        let target = match lexeme {
            "byte" => TargetType::Byte,
            "short" => TargetType::Short,
            "int" => TargetType::Int,
            "long" => TargetType::Long,
            "float" => TargetType::Float,
            "double" => TargetType::Double,
            "boolean" => TargetType::Boolean,
            "char" => TargetType::Character,
            _ => return None,
        };
        // Java `BuiltInTypesSet` 返回包装类，而不是 `int.class` 等原语类。
        Some(ClassRef::Boxed(target))
    }

    /// Java `parseDeclTypeNoArr`.
    fn parse_decl_type_no_arr(&mut self, node: &Node) -> ClassRef {
        let Node::DeclTypeNoArr(ctx) = node else {
            return object_cls();
        };
        if let Some(primitive) = &ctx.primitive_type {
            let text = primitive.text();
            if let Some(cls) = Self::built_in_cls(&text) {
                return cls;
            }
        }
        match &ctx.cls_type {
            Some(cls_type) => self.parse_cls_ids(cls_type_children(cls_type)),
            None => object_cls(),
        }
    }

    /// Java `parseDeclType` (base type wrapped in `dims` array layers).
    fn parse_decl_type(&mut self, node: &Node) -> ClassRef {
        let Node::DeclType(ctx) = node else {
            return object_cls();
        };
        let base_cls = if let Some(primitive) = &ctx.primitive_type {
            Self::built_in_cls(&primitive.text()).unwrap_or_else(object_cls)
        } else if let Some(cls_type) = &ctx.cls_type {
            self.parse_cls_ids(cls_type_children(cls_type))
        } else {
            object_cls()
        };
        let layers = ctx.dims.as_ref().map_or(0, |d| dims_dim_count(d));
        wrap_in_array(base_cls, layers)
    }

    /// Java `parseClsIds`: resolve a dotted class name through the import
    /// manager, reporting `CLASS_NOT_FOUND` when unresolvable.
    fn parse_cls_ids(&mut self, var_ids: &[Node]) -> ClassRef {
        let field_ids: Vec<String> = var_ids.iter().map(|id| id.text()).collect();
        let result = self.import_manager.borrow().load_part_qualified(&field_ids);
        match result.cls() {
            Some(cls) if result.rest_index() == field_ids.len() => ClassRef::from_name(cls),
            _ => {
                let last_id = var_ids.last().expect("class ids non-empty");
                let reason = error_codes::format_msg(
                    error_codes::error_msg(error_codes::CLASS_NOT_FOUND),
                    &[field_ids.join(".")],
                );
                if let Some(token) = last_id.start_token() {
                    let token = token.clone();
                    self.report_parse_err(&token, error_codes::CLASS_NOT_FOUND, &reason);
                }
                ClassRef::Named(field_ids.join("."))
            }
        }
    }
}
