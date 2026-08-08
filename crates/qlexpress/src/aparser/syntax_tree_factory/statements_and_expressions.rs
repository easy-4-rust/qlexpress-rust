impl BreakContinueStatementContext {
    /// 判断 break 条件。
    /// 无显式参数；返回：`bool`。
    /// 对应或承接 Java 源文件：`com/alibaba/qlexpress4/aparser/SyntaxTreeFactory.java`，方法 `isBreak`；Rust 侧按所有权与 `Result` 语义适配。
    /// Java `BREAK() != null`.
    /// 对应 Java: com.alibaba.qlexpress4.aparser.SyntaxTreeFactory#isBreak。
    pub fn is_break(&self) -> bool {
        self.token.symbol().token_type() == super::token::BREAK as i32
    }
}

impl DimsContext {
    /// 返回数组类型声明包含的维度数量。
    /// 无显式参数；返回：`usize`。
    /// 对应或承接 Java 源文件：`com/alibaba/qlexpress4/aparser/SyntaxTreeFactory.java`，方法 `dimCount`；Rust 侧按所有权与 `Result` 语义适配。
    /// Number of `[]` dimensions (Java `LBRACK().size()`).
    /// 对应 Java: com.alibaba.qlexpress4.aparser.SyntaxTreeFactory#dimCount。
    pub fn dim_count(&self) -> usize {
        self.brackets.len() / 2
    }
}

impl ExpressionContext {
    /// 判断 assign 条件。
    /// 无显式参数；返回：`bool`。
    /// 对应或承接 Java 源文件：`com/alibaba/qlexpress4/aparser/SyntaxTreeFactory.java`，方法 `isAssign`；Rust 侧按所有权与 `Result` 语义适配。
    /// Java `leftHandSide()`.
    /// 对应 Java: com.alibaba.qlexpress4.aparser.SyntaxTreeFactory#isAssign。
    pub fn is_assign(&self) -> bool {
        self.left.is_some()
    }
}

// ---------------------------------------------------------------------------
// Node enum: one variant per Java QLParser *Context class.
// ---------------------------------------------------------------------------

// ---------------------------------------------------------------------------
// HasChildren implementations (child order mirrors Java addChild calls).
// ---------------------------------------------------------------------------

impl HasChildren for ProgramContext {
    fn children(&self) -> Vec<ChildRef<'_>> {
        let mut out = Vec::new();
        push_all(&mut out, &self.imports);
        push_opt(&mut out, &self.block_statements);
        out
    }
}

impl HasChildren for BlockStatementsContext {
    fn children(&self) -> Vec<ChildRef<'_>> {
        self.statements.iter().map(n).collect()
    }
}

impl HasChildren for LocalVariableDeclarationStatementContext {
    fn children(&self) -> Vec<ChildRef<'_>> {
        vec![n(&self.local_variable_declaration), t(&self.semi)]
    }
}

impl HasChildren for ThrowStatementContext {
    fn children(&self) -> Vec<ChildRef<'_>> {
        vec![t(&self.throw_token), n(&self.expression)]
    }
}

impl HasChildren for WhileStatementContext {
    fn children(&self) -> Vec<ChildRef<'_>> {
        let mut out = vec![
            t(&self.while_token),
            t(&self.lparen),
            n(&self.expression),
            t(&self.rparen),
            t(&self.lbrace),
        ];
        push_opt(&mut out, &self.block_statements);
        out.push(t(&self.rbrace));
        out
    }
}

impl HasChildren for TraditionalForStatementContext {
    fn children(&self) -> Vec<ChildRef<'_>> {
        let mut out = vec![t(&self.for_token), t(&self.lparen), n(&self.for_init)];
        push_opt(&mut out, &self.for_condition);
        out.push(t(&self.condition_semi));
        push_opt(&mut out, &self.for_update);
        out.push(t(&self.rparen));
        out.push(t(&self.lbrace));
        push_opt(&mut out, &self.block_statements);
        out.push(t(&self.rbrace));
        out
    }
}

impl HasChildren for ForInitContext {
    fn children(&self) -> Vec<ChildRef<'_>> {
        let mut out = Vec::new();
        push_opt(&mut out, &self.local_variable_declaration);
        push_opt(&mut out, &self.expression);
        out.push(t(&self.semi));
        out
    }
}

impl HasChildren for ForEachStatementContext {
    fn children(&self) -> Vec<ChildRef<'_>> {
        let mut out = vec![t(&self.for_token), t(&self.lparen)];
        push_opt(&mut out, &self.decl_type);
        out.push(n(&self.var_id));
        out.push(t(&self.colon));
        out.push(n(&self.expression));
        out.push(t(&self.rparen));
        out.push(t(&self.lbrace));
        push_opt(&mut out, &self.block_statements);
        out.push(t(&self.rbrace));
        out
    }
}

impl HasChildren for FunctionStatementContext {
    fn children(&self) -> Vec<ChildRef<'_>> {
        let mut out = vec![t(&self.function_token), n(&self.var_id), t(&self.lparen)];
        push_opt(&mut out, &self.params);
        out.push(t(&self.rparen));
        out.push(t(&self.lbrace));
        push_opt(&mut out, &self.block_statements);
        out.push(t(&self.rbrace));
        out
    }
}

impl HasChildren for MacroStatementContext {
    fn children(&self) -> Vec<ChildRef<'_>> {
        let mut out = vec![t(&self.macro_token), n(&self.var_id), t(&self.lbrace)];
        push_opt(&mut out, &self.block_statements);
        out.push(t(&self.rbrace));
        out
    }
}

impl HasChildren for BreakContinueStatementContext {
    fn children(&self) -> Vec<ChildRef<'_>> {
        vec![t(&self.token)]
    }
}

impl HasChildren for ReturnStatementContext {
    fn children(&self) -> Vec<ChildRef<'_>> {
        let mut out = vec![t(&self.return_token)];
        push_opt(&mut out, &self.expression);
        out
    }
}

impl HasChildren for EmptyStatementContext {
    fn children(&self) -> Vec<ChildRef<'_>> {
        vec![t(&self.token)]
    }
}

impl HasChildren for ExpressionStatementContext {
    fn children(&self) -> Vec<ChildRef<'_>> {
        vec![n(&self.expression)]
    }
}

impl HasChildren for NonExpressionStatementContext {
    fn children(&self) -> Vec<ChildRef<'_>> {
        vec![n(&self.statement)]
    }
}

impl HasChildren for LocalVariableDeclarationContext {
    fn children(&self) -> Vec<ChildRef<'_>> {
        vec![n(&self.decl_type), n(&self.variable_declarator_list)]
    }
}

impl HasChildren for VariableDeclaratorListContext {
    fn children(&self) -> Vec<ChildRef<'_>> {
        let mut out = Vec::new();
        push_interleaved(&mut out, &self.variables, &self.commas);
        out
    }
}

impl HasChildren for VariableDeclaratorContext {
    fn children(&self) -> Vec<ChildRef<'_>> {
        let mut out = vec![n(&self.id)];
        push_opt_term(&mut out, &self.equals);
        push_opt(&mut out, &self.initializer);
        out
    }
}

impl HasChildren for VariableDeclaratorIdContext {
    fn children(&self) -> Vec<ChildRef<'_>> {
        let mut out = vec![n(&self.var_id)];
        push_opt(&mut out, &self.dims);
        out
    }
}

impl HasChildren for VariableInitializerContext {
    fn children(&self) -> Vec<ChildRef<'_>> {
        let mut out = Vec::new();
        push_opt(&mut out, &self.expression);
        push_opt(&mut out, &self.array_initializer);
        out
    }
}

impl HasChildren for ArrayInitializerContext {
    fn children(&self) -> Vec<ChildRef<'_>> {
        let mut out = vec![t(&self.lbrace)];
        push_opt(&mut out, &self.initializers);
        out.push(t(&self.rbrace));
        out
    }
}

impl HasChildren for VariableInitializerListContext {
    fn children(&self) -> Vec<ChildRef<'_>> {
        let mut out = Vec::new();
        push_interleaved(&mut out, &self.initializers, &self.commas);
        out
    }
}

impl HasChildren for DeclTypeContext {
    fn children(&self) -> Vec<ChildRef<'_>> {
        let mut out = Vec::new();
        push_opt(&mut out, &self.primitive_type);
        push_opt(&mut out, &self.cls_type);
        push_opt(&mut out, &self.dims);
        out
    }
}

impl HasChildren for DeclTypeNoArrContext {
    fn children(&self) -> Vec<ChildRef<'_>> {
        let mut out = Vec::new();
        push_opt(&mut out, &self.primitive_type);
        push_opt(&mut out, &self.cls_type);
        out
    }
}

impl HasChildren for PrimitiveTypeContext {
    fn children(&self) -> Vec<ChildRef<'_>> {
        vec![t(&self.token)]
    }
}

impl HasChildren for ClsTypeContext {
    fn children(&self) -> Vec<ChildRef<'_>> {
        self.var_ids.iter().map(n).collect()
    }
}

impl HasChildren for DimsContext {
    fn children(&self) -> Vec<ChildRef<'_>> {
        self.brackets.iter().map(t).collect()
    }
}

impl HasChildren for DimExprsContext {
    fn children(&self) -> Vec<ChildRef<'_>> {
        let mut out = Vec::with_capacity(self.expressions.len() * 3);
        for (index, expression) in self.expressions.iter().enumerate() {
            if let Some(lbrack) = self.brackets.get(index * 2) {
                out.push(t(lbrack));
            }
            out.push(n(expression));
            if let Some(rbrack) = self.brackets.get(index * 2 + 1) {
                out.push(t(rbrack));
            }
        }
        out
    }
}

impl HasChildren for ExpressionContext {
    fn children(&self) -> Vec<ChildRef<'_>> {
        let mut out = Vec::new();
        push_opt(&mut out, &self.left);
        push_opt(&mut out, &self.assign_operator);
        push_opt(&mut out, &self.expression);
        push_opt(&mut out, &self.ternary);
        out
    }
}

impl HasChildren for LeftHandSideContext {
    fn children(&self) -> Vec<ChildRef<'_>> {
        let mut out = vec![n(&self.var_id)];
        push_opt_term(&mut out, &self.lparen);
        push_opt(&mut out, &self.argument_list);
        push_opt_term(&mut out, &self.rparen);
        push_all(&mut out, &self.path_parts);
        out
    }
}

