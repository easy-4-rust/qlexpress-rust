impl HasChildren for TryCatchContext {
    fn children(&self) -> Vec<ChildRef<'_>> {
        let mut out = vec![
            t(&self.catch_token),
            t(&self.lparen),
            n(&self.catch_params),
            t(&self.rparen),
            t(&self.lbrace),
        ];
        push_opt(&mut out, &self.block_statements);
        out.push(t(&self.rbrace));
        out
    }
}

impl HasChildren for CatchParamsContext {
    fn children(&self) -> Vec<ChildRef<'_>> {
        let mut out = Vec::new();
        push_interleaved(&mut out, &self.decl_types, &self.bit_ors);
        out.push(n(&self.var_id));
        out
    }
}

impl HasChildren for TryFinallyContext {
    fn children(&self) -> Vec<ChildRef<'_>> {
        let mut out = vec![t(&self.finally_token), t(&self.lbrace)];
        push_opt(&mut out, &self.block_statements);
        out.push(t(&self.rbrace));
        out
    }
}

impl HasChildren for MapEntriesContext {
    fn children(&self) -> Vec<ChildRef<'_>> {
        let mut out = Vec::new();
        push_opt_term(&mut out, &self.empty_colon);
        push_interleaved(&mut out, &self.entries, &self.commas);
        out
    }
}

impl HasChildren for MapEntryContext {
    fn children(&self) -> Vec<ChildRef<'_>> {
        vec![n(&self.map_key), t(&self.colon), n(&self.map_value)]
    }
}

impl HasChildren for ClsValueContext {
    fn children(&self) -> Vec<ChildRef<'_>> {
        vec![t(&self.quote)]
    }
}

impl HasChildren for EValueContext {
    fn children(&self) -> Vec<ChildRef<'_>> {
        vec![n(&self.expression)]
    }
}

impl HasChildren for IdKeyContext {
    fn children(&self) -> Vec<ChildRef<'_>> {
        vec![t(&self.token)]
    }
}

impl HasChildren for StringKeyContext {
    fn children(&self) -> Vec<ChildRef<'_>> {
        vec![n(&self.double_quote_string)]
    }
}

impl HasChildren for QuoteStringKeyContext {
    fn children(&self) -> Vec<ChildRef<'_>> {
        vec![t(&self.token)]
    }
}

impl HasChildren for MethodInvokeContext {
    fn children(&self) -> Vec<ChildRef<'_>> {
        let mut out = vec![t(&self.dot), n(&self.var_id), t(&self.lparen)];
        push_opt(&mut out, &self.argument_list);
        out.push(t(&self.rparen));
        out
    }
}

impl HasChildren for FieldAccessContext {
    fn children(&self) -> Vec<ChildRef<'_>> {
        vec![t(&self.dot), n(&self.field_id)]
    }
}

impl HasChildren for MethodAccessContext {
    fn children(&self) -> Vec<ChildRef<'_>> {
        vec![t(&self.dcolon), n(&self.var_id)]
    }
}

impl HasChildren for IndexExprContext {
    fn children(&self) -> Vec<ChildRef<'_>> {
        let mut out = vec![t(&self.lbrack)];
        push_opt(&mut out, &self.index_value_expr);
        out.push(t(&self.rbrack));
        out
    }
}

impl HasChildren for CustomPathContext {
    fn children(&self) -> Vec<ChildRef<'_>> {
        let mut out = vec![n(&self.op_id)];
        push_opt(&mut out, &self.var_id);
        push_opt_term(&mut out, &self.quote);
        out
    }
}

impl HasChildren for FieldIdContext {
    fn children(&self) -> Vec<ChildRef<'_>> {
        let mut out = Vec::new();
        push_opt_term(&mut out, &self.token);
        push_opt_term(&mut out, &self.quote);
        out
    }
}

impl HasChildren for SingleIndexContext {
    fn children(&self) -> Vec<ChildRef<'_>> {
        vec![n(&self.expression)]
    }
}

impl HasChildren for SliceIndexContext {
    fn children(&self) -> Vec<ChildRef<'_>> {
        let mut out = Vec::new();
        push_opt(&mut out, &self.start);
        out.push(t(&self.colon));
        push_opt(&mut out, &self.end);
        out
    }
}

impl HasChildren for ArgumentListContext {
    fn children(&self) -> Vec<ChildRef<'_>> {
        let mut out = Vec::new();
        push_interleaved(&mut out, &self.expressions, &self.commas);
        out
    }
}

impl HasChildren for LiteralContext {
    fn children(&self) -> Vec<ChildRef<'_>> {
        let mut out = Vec::new();
        push_opt_term(&mut out, &self.token);
        push_opt(&mut out, &self.boolen);
        push_opt(&mut out, &self.double_quote_string);
        out
    }
}

impl HasChildren for DoubleQuoteStringLiteralContext {
    fn children(&self) -> Vec<ChildRef<'_>> {
        let mut out = vec![t(&self.open_quote)];
        push_opt_term(&mut out, &self.static_characters);
        for part in &self.parts {
            match part {
                DyStrPart::Text(term) => out.push(t(term)),
                DyStrPart::Expr(node) => out.push(n(node)),
            }
        }
        out.push(t(&self.close_quote));
        out
    }
}

impl HasChildren for StringExpressionContext {
    fn children(&self) -> Vec<ChildRef<'_>> {
        let mut out = vec![t(&self.start)];
        push_opt_term(&mut out, &self.selector_variable);
        push_opt(&mut out, &self.expression);
        push_opt_term(&mut out, &self.rbrace);
        out
    }
}

impl HasChildren for BoolenLiteralContext {
    fn children(&self) -> Vec<ChildRef<'_>> {
        vec![t(&self.token)]
    }
}

impl HasChildren for LambdaExprContext {
    fn children(&self) -> Vec<ChildRef<'_>> {
        let mut out = vec![n(&self.lambda_parameters), t(&self.arrow)];
        push_opt_term(&mut out, &self.lbrace);
        push_opt(&mut out, &self.block_statements);
        push_opt_term(&mut out, &self.rbrace);
        push_opt(&mut out, &self.expression);
        out
    }
}

impl HasChildren for LambdaParametersContext {
    fn children(&self) -> Vec<ChildRef<'_>> {
        let mut out = Vec::new();
        push_opt(&mut out, &self.var_id);
        push_opt_term(&mut out, &self.lparen);
        push_opt(&mut out, &self.params);
        push_opt_term(&mut out, &self.rparen);
        out
    }
}

impl HasChildren for FormalOrInferredParameterListContext {
    fn children(&self) -> Vec<ChildRef<'_>> {
        let mut out = Vec::new();
        push_interleaved(&mut out, &self.params, &self.commas);
        out
    }
}

impl HasChildren for FormalOrInferredParameterContext {
    fn children(&self) -> Vec<ChildRef<'_>> {
        let mut out = Vec::new();
        push_opt(&mut out, &self.decl_type);
        out.push(n(&self.var_id));
        out
    }
}

impl HasChildren for ImportClsContext {
    fn children(&self) -> Vec<ChildRef<'_>> {
        let mut out = vec![t(&self.import_token)];
        push_all(&mut out, &self.var_ids);
        out.push(t(&self.semi));
        out
    }
}

impl HasChildren for ImportPackContext {
    fn children(&self) -> Vec<ChildRef<'_>> {
        let mut out = vec![t(&self.import_token)];
        push_all(&mut out, &self.var_ids);
        out.push(t(&self.semi));
        out
    }
}

impl HasChildren for OpIdContext {
    fn children(&self) -> Vec<ChildRef<'_>> {
        vec![t(&self.token)]
    }
}

impl HasChildren for VarIdContext {
    fn children(&self) -> Vec<ChildRef<'_>> {
        vec![t(&self.token)]
    }
}

