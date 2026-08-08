impl HasChildren for AssignOperatorContext {
    fn children(&self) -> Vec<ChildRef<'_>> {
        vec![t(&self.token)]
    }
}

impl HasChildren for TernaryExprContext {
    fn children(&self) -> Vec<ChildRef<'_>> {
        let mut out = vec![n(&self.condition)];
        push_opt_term(&mut out, &self.question);
        push_opt(&mut out, &self.then_expr);
        push_opt_term(&mut out, &self.colon);
        push_opt(&mut out, &self.else_expr);
        out
    }
}

impl HasChildren for BaseExprContext {
    fn children(&self) -> Vec<ChildRef<'_>> {
        let mut out = vec![n(&self.primary)];
        push_all(&mut out, &self.left_assos);
        out
    }
}

impl HasChildren for LeftAssoContext {
    fn children(&self) -> Vec<ChildRef<'_>> {
        vec![n(&self.binaryop), n(&self.right)]
    }
}

impl HasChildren for BinaryopContext {
    fn children(&self) -> Vec<ChildRef<'_>> {
        vec![t(&self.token)]
    }
}

impl HasChildren for PrimaryContext {
    fn children(&self) -> Vec<ChildRef<'_>> {
        let mut out = Vec::new();
        if let Some(non_pathable) = &self.non_pathable {
            out.push(n(non_pathable));
            return out;
        }
        push_opt(&mut out, &self.prefix);
        push_opt(&mut out, &self.pathable);
        push_all(&mut out, &self.path_parts);
        push_opt(&mut out, &self.suffix);
        out
    }
}

impl HasChildren for PrefixExpressContext {
    fn children(&self) -> Vec<ChildRef<'_>> {
        vec![n(&self.op_id)]
    }
}

impl HasChildren for SuffixExpressContext {
    fn children(&self) -> Vec<ChildRef<'_>> {
        vec![n(&self.op_id)]
    }
}

impl HasChildren for ConstExprContext {
    fn children(&self) -> Vec<ChildRef<'_>> {
        vec![n(&self.literal)]
    }
}

impl HasChildren for CastExprContext {
    fn children(&self) -> Vec<ChildRef<'_>> {
        vec![
            t(&self.lparen),
            n(&self.decl_type),
            t(&self.rparen),
            n(&self.primary),
        ]
    }
}

impl HasChildren for GroupExprContext {
    fn children(&self) -> Vec<ChildRef<'_>> {
        vec![t(&self.lparen), n(&self.expression), t(&self.rparen)]
    }
}

impl HasChildren for NewObjExprContext {
    fn children(&self) -> Vec<ChildRef<'_>> {
        let mut out = vec![t(&self.new_token)];
        push_all(&mut out, &self.var_ids);
        out.push(t(&self.lparen));
        push_opt(&mut out, &self.argument_list);
        out.push(t(&self.rparen));
        out
    }
}

impl HasChildren for NewEmptyArrExprContext {
    fn children(&self) -> Vec<ChildRef<'_>> {
        vec![
            t(&self.new_token),
            n(&self.decl_type_no_arr),
            n(&self.dim_exprs),
        ]
    }
}

impl HasChildren for NewInitArrExprContext {
    fn children(&self) -> Vec<ChildRef<'_>> {
        vec![
            t(&self.new_token),
            n(&self.decl_type_no_arr),
            n(&self.dims),
            n(&self.array_initializer),
        ]
    }
}

impl HasChildren for VarIdExprContext {
    fn children(&self) -> Vec<ChildRef<'_>> {
        let mut out = vec![n(&self.var_id)];
        push_opt_term(&mut out, &self.lparen);
        push_opt(&mut out, &self.argument_list);
        push_opt_term(&mut out, &self.rparen);
        out
    }
}

impl HasChildren for TypeExprContext {
    fn children(&self) -> Vec<ChildRef<'_>> {
        vec![n(&self.decl_type)]
    }
}

impl HasChildren for ListExprContext {
    fn children(&self) -> Vec<ChildRef<'_>> {
        let mut out = vec![t(&self.lbrack)];
        push_opt(&mut out, &self.list_items);
        out.push(t(&self.rbrack));
        out
    }
}

impl HasChildren for ListItemsContext {
    fn children(&self) -> Vec<ChildRef<'_>> {
        let mut out = Vec::new();
        push_interleaved(&mut out, &self.expressions, &self.commas);
        out
    }
}

impl HasChildren for MapExprContext {
    fn children(&self) -> Vec<ChildRef<'_>> {
        vec![t(&self.lbrace), n(&self.map_entries), t(&self.rbrace)]
    }
}

impl HasChildren for BlockExprContext {
    fn children(&self) -> Vec<ChildRef<'_>> {
        let mut out = vec![t(&self.lbrace)];
        push_opt(&mut out, &self.block_statements);
        out.push(t(&self.rbrace));
        out
    }
}

impl HasChildren for ContextSelectExprContext {
    fn children(&self) -> Vec<ChildRef<'_>> {
        vec![t(&self.selector_start), t(&self.selector_variable)]
    }
}

impl HasChildren for QlIfContext {
    fn children(&self) -> Vec<ChildRef<'_>> {
        let mut out = vec![
            t(&self.if_token),
            t(&self.lparen),
            n(&self.condition),
            t(&self.rparen),
        ];
        push_opt_term(&mut out, &self.then_keyword);
        out.push(n(&self.then_body));
        push_opt_term(&mut out, &self.else_keyword);
        push_opt(&mut out, &self.else_body);
        out
    }
}

impl HasChildren for ThenBodyContext {
    fn children(&self) -> Vec<ChildRef<'_>> {
        let mut out = Vec::new();
        push_opt_term(&mut out, &self.lbrace);
        push_opt(&mut out, &self.block_statements);
        push_opt_term(&mut out, &self.rbrace);
        push_opt(&mut out, &self.non_expression_statement);
        push_opt(&mut out, &self.expression);
        out
    }
}

impl HasChildren for ElseBodyContext {
    fn children(&self) -> Vec<ChildRef<'_>> {
        let mut out = Vec::new();
        push_opt_term(&mut out, &self.lbrace);
        push_opt(&mut out, &self.block_statements);
        push_opt_term(&mut out, &self.rbrace);
        push_opt(&mut out, &self.ql_if);
        push_opt(&mut out, &self.non_expression_statement);
        push_opt(&mut out, &self.expression);
        out
    }
}

impl HasChildren for SwitchExprContext {
    fn children(&self) -> Vec<ChildRef<'_>> {
        let mut out = vec![
            t(&self.switch_token),
            t(&self.lparen),
            n(&self.expression),
            t(&self.rparen),
            t(&self.lbrace),
        ];
        push_opt(&mut out, &self.groups);
        out.push(t(&self.rbrace));
        out
    }
}

impl HasChildren for SwitchCaseGroupsContext {
    fn children(&self) -> Vec<ChildRef<'_>> {
        self.groups.iter().map(n).collect()
    }
}

impl HasChildren for SwitchStatementGroupContext {
    fn children(&self) -> Vec<ChildRef<'_>> {
        let mut out = vec![n(&self.labels)];
        push_opt(&mut out, &self.block_statements);
        out
    }
}

impl HasChildren for SwitchExprGroupContext {
    fn children(&self) -> Vec<ChildRef<'_>> {
        vec![n(&self.label), n(&self.expression)]
    }
}

impl HasChildren for SwitchLabelsContext {
    fn children(&self) -> Vec<ChildRef<'_>> {
        self.labels.iter().map(n).collect()
    }
}

impl HasChildren for SwitchLabelContext {
    fn children(&self) -> Vec<ChildRef<'_>> {
        let mut out = Vec::new();
        push_opt_term(&mut out, &self.case_token);
        push_opt_term(&mut out, &self.default_token);
        push_opt(&mut out, &self.expression);
        out.push(t(&self.colon));
        out
    }
}

impl HasChildren for SwitchExpressionLabelContext {
    fn children(&self) -> Vec<ChildRef<'_>> {
        let mut out = Vec::new();
        push_opt_term(&mut out, &self.case_token);
        push_opt_term(&mut out, &self.default_token);
        push_opt(&mut out, &self.expression_list);
        out.push(t(&self.arrow));
        out
    }
}

impl HasChildren for ExpressionListContext {
    fn children(&self) -> Vec<ChildRef<'_>> {
        let mut out = Vec::new();
        push_interleaved(&mut out, &self.expressions, &self.commas);
        out
    }
}

impl HasChildren for TryCatchExprContext {
    fn children(&self) -> Vec<ChildRef<'_>> {
        let mut out = vec![t(&self.try_token), t(&self.lbrace)];
        push_opt(&mut out, &self.block_statements);
        out.push(t(&self.rbrace));
        push_opt(&mut out, &self.try_catches);
        push_opt(&mut out, &self.try_finally);
        out
    }
}

impl HasChildren for TryCatchesContext {
    fn children(&self) -> Vec<ChildRef<'_>> {
        self.catches.iter().map(n).collect()
    }
}

