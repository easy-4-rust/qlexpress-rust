impl<'a> QLParser<'a> {
    /// 创建对象实例。
    /// 参数：`script`、`tokens`、`operator_manager`、`strict_new_lines`；返回：`Self`。
    /// 对应或承接 Java 源文件：`com/alibaba/qlexpress4/aparser/QLParser.java`，构造器 `<init>`；Rust 侧按所有权与 `Result` 语义适配。
    /// Java `new QLParser(script, tokens, operatorManager, strictNewLines)`.
    /// 对应 Java: com.alibaba.qlexpress4.aparser.QLParser#new。
    pub fn new(
        script: &'a str,
        tokens: &'a [Token],
        operator_manager: Option<&'a dyn ParserOperatorManager>,
        strict_new_lines: bool,
    ) -> Self {
        QLParser {
            script,
            tokens,
            operator_manager,
            strict_new_lines,
            p: 0,
        }
    }

    /// 解析完整脚本并返回程序根节点。
    /// 无显式参数；返回：`Result<Node, QLSyntaxException>`。
    /// 对应或承接 Java 源文件：`com/alibaba/qlexpress4/aparser/QLParser.java`，方法 `program`；Rust 侧按所有权与 `Result` 语义适配。
    /// Java `program()`.
    /// 对应 Java: com.alibaba.qlexpress4.aparser.QLParser#program。
    pub fn program(&mut self) -> Result<Node, QLSyntaxException> {
        self.program_internal().map_err(QLSyntaxException::from)
    }

    fn program_internal(&mut self) -> PResult<Node> {
        let mut ctx = ProgramContext {
            imports: Vec::new(),
            block_statements: None,
        };
        self.skip_newlines();
        while self.la(IMPORT) {
            let import = self.parse_import_declaration()?;
            ctx.imports.push(import);
            self.skip_newlines();
        }
        if !self.la(EOF) {
            ctx.block_statements = Some(Box::new(self.parse_block_statements_until(EOF)?));
        }
        self.expect(EOF, "<EOF>")?;
        Ok(Node::Program(ctx))
    }

    // ------------------------------------------------------------------
    // Token stream plumbing (Java lt/token/la/consume/expect/syntax).
    // ------------------------------------------------------------------

    fn lt(&self) -> &'a Token {
        self.token(self.p)
    }

    fn token(&self, index: usize) -> &'a Token {
        if index >= self.tokens.len() {
            return &self.tokens[self.tokens.len() - 1];
        }
        &self.tokens[index]
    }

    fn la(&self, token_type: i32) -> bool {
        self.lt().token_type() == token_type
    }

    fn la_at(&self, offset: usize, token_type: i32) -> bool {
        self.token(self.p + offset).token_type() == token_type
    }

    fn consume(&mut self) -> &'a Token {
        let tok = self.token(self.p);
        self.p += 1;
        tok
    }

    fn consume_node(&mut self) -> TerminalNode {
        TerminalNode::new(self.consume().clone())
    }

    fn expect(&mut self, token_type: i32, display: &str) -> PResult<TerminalNode> {
        if !self.la(token_type) {
            let tok = self.lt();
            return Err(self.syntax(
                tok,
                format!(
                    "mismatched input '{}' expecting {}",
                    token_text(tok),
                    display
                ),
            ));
        }
        Ok(self.consume_node())
    }

    /// Java `syntax(token, reason)` — fatal syntax error.
    fn syntax(&self, tok: &Token, reason: String) -> ParseFail {
        let report_script = if tok.token_type() == EOF {
            format!("{}<EOF>", self.script)
        } else {
            self.script.to_string()
        };
        ParseFail::Syntax(QLException::report_scanner_err(
            &report_script,
            tok.start_index(),
            tok.line(),
            tok.char_position_in_line() + 1,
            &token_text(tok),
            error_codes::SYNTAX_ERROR,
            &reason,
        ))
    }

    fn backtrack<T>() -> PResult<T> {
        Err(ParseFail::Backtrack)
    }

    // ------------------------------------------------------------------
    // Statements
    // ------------------------------------------------------------------

    fn parse_block_statements_until(&mut self, end_type: i32) -> PResult<Node> {
        let _guard = DepthGuard::enter()?;
        let mut ctx = BlockStatementsContext {
            statements: Vec::new(),
        };
        while !self.la(end_type) && !self.la(EOF) {
            if end_type == RBRACE && self.la(RBRACE) {
                break;
            }
            let statement = self.parse_block_statement()?;
            ctx.statements.push(statement);
        }
        Ok(Node::BlockStatements(ctx))
    }

    fn parse_block_statement(&mut self) -> PResult<Node> {
        let _guard = DepthGuard::enter()?;
        if self.la(NEWLINE) || self.la(SEMI) {
            return Ok(Node::EmptyStatement(EmptyStatementContext {
                token: self.consume_node(),
            }));
        }
        if self.la(IMPORT) {
            let tok = self.lt();
            return Err(self.syntax(
                tok,
                "Import statement is not at the beginning of the file.".to_string(),
            ));
        }
        if self.la(THROW) {
            let throw_token = self.consume_node();
            let expression = self.parse_expression()?;
            self.consume_next_statement()?;
            return Ok(Node::ThrowStatement(ThrowStatementContext {
                throw_token,
                expression: Box::new(expression),
            }));
        }
        if self.la(WHILE) {
            return self.parse_while_statement();
        }
        if self.la(FOR) {
            return self.parse_for_statement();
        }
        if self.la(FUNCTION) {
            return self.parse_function_statement();
        }
        if self.la(MACRO) {
            return self.parse_macro_statement();
        }
        if let Some(local) = self.try_parse_local_variable_declaration()? {
            let semi = self.expect(SEMI, "';'")?;
            return Ok(Node::LocalVariableDeclarationStatement(
                LocalVariableDeclarationStatementContext {
                    local_variable_declaration: Box::new(local),
                    semi,
                },
            ));
        }
        if self.la(BREAK) || self.la(CONTINUE) {
            let token = self.consume_node();
            self.consume_next_statement()?;
            return Ok(Node::BreakContinueStatement(
                BreakContinueStatementContext { token },
            ));
        }
        if self.la(RETURN) {
            let return_token = self.consume_node();
            let expression = if !self.is_next_statement_start() {
                Some(Box::new(self.parse_expression()?))
            } else {
                None
            };
            self.consume_next_statement()?;
            return Ok(Node::ReturnStatement(ReturnStatementContext {
                return_token,
                expression,
            }));
        }
        let expression = self.parse_expression()?;
        self.consume_next_statement()?;
        Ok(Node::ExpressionStatement(ExpressionStatementContext {
            expression: Box::new(expression),
        }))
    }

    fn parse_while_statement(&mut self) -> PResult<Node> {
        let while_token = self.consume_node();
        let lparen = self.expect(LPAREN, "'('")?;
        self.skip_newlines();
        let expression = self.parse_expression()?;
        self.skip_newlines();
        let rparen = self.expect(RPAREN, "')'")?;
        let (lbrace, block_statements, rbrace) = self.parse_braced_block()?;
        Ok(Node::WhileStatement(WhileStatementContext {
            while_token,
            lparen,
            expression: Box::new(expression),
            rparen,
            lbrace,
            block_statements: block_statements.map(Box::new),
            rbrace,
        }))
    }

    /// Java `parseBracedBlock`: `{ ... }`, `Ok(None)` for an empty block.
    fn parse_braced_block(&mut self) -> PResult<(TerminalNode, Option<Node>, TerminalNode)> {
        let lbrace = self.expect(LBRACE, "'{'")?;
        self.skip_newlines();
        let mut block = None;
        if !self.la(RBRACE) {
            block = Some(self.parse_block_statements_until(RBRACE)?);
        }
        self.skip_newlines();
        let rbrace = self.expect(RBRACE, "'}'")?;
        Ok((lbrace, block, rbrace))
    }

    fn parse_for_statement(&mut self) -> PResult<Node> {
        let save = self.p;
        self.consume();
        self.expect(LPAREN, "'('")?;
        let for_each = self.scan_for_each_header();
        self.p = save;
        if for_each {
            return self.parse_for_each_statement();
        }
        self.parse_traditional_for_statement()
    }

    fn scan_for_each_header(&self) -> bool {
        let mut depth = 0i32;
        let mut i = self.p;
        while i < self.tokens.len() {
            let ty = self.tokens[i].token_type();
            if ty == LPAREN || ty == LBRACK || ty == LBRACE {
                depth += 1;
            } else if ty == RPAREN || ty == RBRACK || ty == RBRACE {
                depth -= 1;
                if depth <= 0 && ty == RPAREN {
                    return false;
                }
            } else if depth == 0 && ty == SEMI {
                return false;
            } else if depth == 0 && ty == COLON {
                return true;
            }
            i += 1;
        }
        false
    }

    fn parse_traditional_for_statement(&mut self) -> PResult<Node> {
        let for_token = self.consume_node();
        let lparen = self.expect(LPAREN, "'('")?;
        self.skip_newlines();
        let for_init = self.parse_for_init()?;
        self.skip_newlines();
        let for_condition = if !self.la(SEMI) {
            Some(Box::new(self.parse_expression()?))
        } else {
            None
        };
        let condition_semi = self.expect(SEMI, "';'")?;
        self.skip_newlines();
        let for_update = if !self.la(RPAREN) {
            Some(Box::new(self.parse_expression()?))
        } else {
            None
        };
        self.skip_newlines();
        let rparen = self.expect(RPAREN, "')'")?;
        let (lbrace, block_statements, rbrace) = self.parse_braced_block()?;
        Ok(Node::TraditionalForStatement(
            TraditionalForStatementContext {
                for_token,
                lparen,
                for_init: Box::new(for_init),
                for_condition,
                condition_semi,
                for_update,
                rparen,
                lbrace,
                block_statements: block_statements.map(Box::new),
                rbrace,
            },
        ))
    }

    fn parse_for_init(&mut self) -> PResult<Node> {
        if self.la(SEMI) {
            let semi = self.consume_node();
            return Ok(Node::ForInit(ForInitContext {
                local_variable_declaration: None,
                expression: None,
                semi,
            }));
        }
        if let Some(local) = self.try_parse_local_variable_declaration()? {
            let semi = self.expect(SEMI, "';'")?;
            return Ok(Node::ForInit(ForInitContext {
                local_variable_declaration: Some(Box::new(local)),
                expression: None,
                semi,
            }));
        }
        let expression = self.parse_expression()?;
        let semi = self.expect(SEMI, "';'")?;
        Ok(Node::ForInit(ForInitContext {
            local_variable_declaration: None,
            expression: Some(Box::new(expression)),
            semi,
        }))
    }

    fn parse_for_each_statement(&mut self) -> PResult<Node> {
        let for_token = self.consume_node();
        let lparen = self.expect(LPAREN, "'('")?;
        self.skip_newlines();
        let decl_type = if !self.single_for_each_var_before_colon() {
            Some(Box::new(self.parse_decl_type()?))
        } else {
            None
        };
        let var_id = self.parse_var_id()?;
        let colon = self.expect(COLON, "':'")?;
        let expression = self.parse_expression()?;
        self.skip_newlines();
        let rparen = self.expect(RPAREN, "')'")?;
        let (lbrace, block_statements, rbrace) = self.parse_braced_block()?;
        Ok(Node::ForEachStatement(ForEachStatementContext {
            for_token,
            lparen,
            decl_type,
            var_id: Box::new(var_id),
            colon,
            expression: Box::new(expression),
            rparen,
            lbrace,
            block_statements: block_statements.map(Box::new),
            rbrace,
        }))
    }

    fn parse_function_statement(&mut self) -> PResult<Node> {
        let function_token = self.consume_node();
        let var_id = self.parse_var_id()?;
        let lparen = self.expect(LPAREN, "'('")?;
        self.skip_newlines();
        let params = if !self.la(RPAREN) {
            Some(Box::new(self.parse_formal_or_inferred_parameter_list()?))
        } else {
            None
        };
        self.skip_newlines();
        let rparen = self.expect(RPAREN, "')'")?;
        let (lbrace, block_statements, rbrace) = self.parse_braced_block()?;
        Ok(Node::FunctionStatement(FunctionStatementContext {
            function_token,
            var_id: Box::new(var_id),
            lparen,
            params,
            rparen,
            lbrace,
            block_statements: block_statements.map(Box::new),
            rbrace,
        }))
    }

    fn parse_macro_statement(&mut self) -> PResult<Node> {
        let macro_token = self.consume_node();
        let var_id = self.parse_var_id()?;
        let (lbrace, block_statements, rbrace) = self.parse_braced_block()?;
        Ok(Node::MacroStatement(MacroStatementContext {
            macro_token,
            var_id: Box::new(var_id),
            lbrace,
            block_statements: block_statements.map(Box::new),
            rbrace,
        }))
    }

    fn parse_non_expression_statement(&mut self) -> PResult<Node> {
        let statement = if self.la(NEWLINE)
            || self.la(SEMI)
            || self.la(THROW)
            || self.la(WHILE)
            || self.la(FOR)
            || self.la(FUNCTION)
            || self.la(MACRO)
            || self.la(BREAK)
            || self.la(CONTINUE)
            || self.la(RETURN)
        {
            self.parse_block_statement()?
        } else {
            match self.try_parse_local_variable_declaration()? {
                None => {
                    let tok = self.lt();
                    return Err(self.syntax(
                        tok,
                        format!("mismatched input '{}' expecting statement", token_text(tok)),
                    ));
                }
                Some(local) => {
                    let semi = self.expect(SEMI, "';'")?;
                    Node::LocalVariableDeclarationStatement(
                        LocalVariableDeclarationStatementContext {
                            local_variable_declaration: Box::new(local),
                            semi,
                        },
                    )
                }
            }
        };
        Ok(Node::NonExpressionStatement(
            NonExpressionStatementContext {
                statement: Box::new(statement),
            },
        ))
    }

    // ------------------------------------------------------------------
    // Local variable declarations
    // ------------------------------------------------------------------

    /// Java `tryParseLocalVariableDeclaration`: catches `QLParseBacktrack`.
    fn try_parse_local_variable_declaration(&mut self) -> PResult<Option<Node>> {
        let save = self.p;
        let decl_type = match self.parse_decl_type() {
            Ok(decl_type) => decl_type,
            Err(ParseFail::Backtrack) => {
                self.p = save;
                return Ok(None);
            }
            Err(fatal) => return Err(fatal),
        };
        if !is_var_id_token(self.lt().token_type()) {
            self.p = save;
            return Ok(None);
        }
        if self.is_middle_operator(self.lt()) {
            self.p = save;
            return Ok(None);
        }
        let variable_declarator_list = self.parse_variable_declarator_list()?;
        Ok(Some(Node::LocalVariableDeclaration(
            LocalVariableDeclarationContext {
                decl_type: Box::new(decl_type),
                variable_declarator_list: Box::new(variable_declarator_list),
            },
        )))
    }

    fn parse_variable_declarator_list(&mut self) -> PResult<Node> {
        let mut ctx = VariableDeclaratorListContext {
            variables: Vec::new(),
            commas: Vec::new(),
        };
        ctx.variables.push(self.parse_variable_declarator()?);
        self.skip_newlines();
        while self.la(COMMA) {
            ctx.commas.push(self.consume_node());
            self.skip_newlines();
            ctx.variables.push(self.parse_variable_declarator()?);
            self.skip_newlines();
        }
        Ok(Node::VariableDeclaratorList(ctx))
    }

    fn parse_variable_declarator(&mut self) -> PResult<Node> {
        let id = self.parse_variable_declarator_id()?;
        let (equals, initializer) = if self.la(EQ) {
            let equals = self.consume_node();
            self.skip_newlines();
            (
                Some(equals),
                Some(Box::new(self.parse_variable_initializer()?)),
            )
        } else {
            (None, None)
        };
        Ok(Node::VariableDeclarator(VariableDeclaratorContext {
            id: Box::new(id),
            equals,
            initializer,
        }))
    }

    fn parse_variable_declarator_id(&mut self) -> PResult<Node> {
        let var_id = self.parse_var_id()?;
        let dims = if self.la(LBRACK) && self.la_at(1, RBRACK) {
            Some(Box::new(self.parse_dims()?))
        } else {
            None
        };
        Ok(Node::VariableDeclaratorId(VariableDeclaratorIdContext {
            var_id: Box::new(var_id),
            dims,
        }))
    }

    fn parse_variable_initializer(&mut self) -> PResult<Node> {
        if self.la(LBRACE) && self.is_array_initializer_ahead() {
            let array_initializer = self.parse_array_initializer()?;
            return Ok(Node::VariableInitializer(VariableInitializerContext {
                expression: None,
                array_initializer: Some(Box::new(array_initializer)),
            }));
        }
        let expression = self.parse_expression()?;
        Ok(Node::VariableInitializer(VariableInitializerContext {
            expression: Some(Box::new(expression)),
            array_initializer: None,
        }))
    }

    fn is_array_initializer_ahead(&mut self) -> bool {
        !self.is_map_expr_ahead()
    }

    fn parse_array_initializer(&mut self) -> PResult<Node> {
        let lbrace = self.expect(LBRACE, "'{'")?;
        self.skip_newlines();
        let initializers = if !self.la(RBRACE) {
            Some(Box::new(self.parse_variable_initializer_list()?))
        } else {
            None
        };
        self.skip_newlines();
        let rbrace = self.expect(RBRACE, "'}'")?;
        Ok(Node::ArrayInitializer(ArrayInitializerContext {
            lbrace,
            initializers,
            rbrace,
        }))
    }

    fn parse_variable_initializer_list(&mut self) -> PResult<Node> {
        let mut ctx = VariableInitializerListContext {
            initializers: Vec::new(),
            commas: Vec::new(),
        };
        ctx.initializers.push(self.parse_variable_initializer()?);
        self.skip_newlines();
        while self.la(COMMA) {
            ctx.commas.push(self.consume_node());
            self.skip_newlines();
            if self.la(RBRACE) {
                break;
            }
            ctx.initializers.push(self.parse_variable_initializer()?);
            self.skip_newlines();
        }
        Ok(Node::VariableInitializerList(ctx))
    }
}

// ---------------------------------------------------------------------------
// Expressions
// ---------------------------------------------------------------------------

