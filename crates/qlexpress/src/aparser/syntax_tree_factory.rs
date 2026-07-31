//! 语法树节点体系(语法树工厂产物),对应 Java `com.alibaba.qlexpress4.aparser.SyntaxTreeFactory`。
//! Java 侧节点为 QLParser 内部 *Context 类(每条文法规则一个内部类),此处聚合于
//! syntax_tree_factory:以单个 [`Node`] 枚举 + 每变体一个 *Context 结构体表示,
//! [`Node::accept`] 派发至同名 Visitor 方法,等价于 Java 的运行时 accept(visitor) 双分派。
//! 本文件由 `syntax_tree.rs` 拆分而来(SPEC §5.5 一类一文件),仅移动代码与补充中文注释,行为完全一致。

use super::rule_context::{
    n, push_all, push_interleaved, push_opt, push_opt_term, t, ChildRef, HasChildren,
};

pub use super::argument_list_context::ArgumentListContext;
pub use super::array_initializer_context::ArrayInitializerContext;
pub use super::assign_operator_context::AssignOperatorContext;
pub use super::base_expr_context::BaseExprContext;
pub use super::binaryop_context::BinaryopContext;
pub use super::block_expr_context::BlockExprContext;
pub use super::block_statements_context::BlockStatementsContext;
pub use super::boolen_literal_context::BoolenLiteralContext;
pub use super::break_continue_statement_context::BreakContinueStatementContext;
pub use super::cast_expr_context::CastExprContext;
pub use super::catch_params_context::CatchParamsContext;
pub use super::chain_kind::ChainKind;
pub use super::cls_type_context::ClsTypeContext;
pub use super::cls_value_context::ClsValueContext;
pub use super::const_expr_context::ConstExprContext;
pub use super::context_select_expr_context::ContextSelectExprContext;
pub use super::custom_path_context::CustomPathContext;
pub use super::decl_type_context::DeclTypeContext;
pub use super::decl_type_no_arr_context::DeclTypeNoArrContext;
pub use super::dim_exprs_context::DimExprsContext;
pub use super::dims_context::DimsContext;
pub use super::double_quote_string_literal_context::DoubleQuoteStringLiteralContext;
pub use super::dy_str_part::DyStrPart;
pub use super::e_value_context::EValueContext;
pub use super::else_body_context::ElseBodyContext;
pub use super::empty_statement_context::EmptyStatementContext;
pub use super::expression_context::ExpressionContext;
pub use super::expression_list_context::ExpressionListContext;
pub use super::expression_statement_context::ExpressionStatementContext;
pub use super::field_access_context::FieldAccessContext;
pub use super::field_id_context::FieldIdContext;
pub use super::for_each_statement_context::ForEachStatementContext;
pub use super::for_init_context::ForInitContext;
pub use super::formal_or_inferred_parameter_context::FormalOrInferredParameterContext;
pub use super::formal_or_inferred_parameter_list_context::FormalOrInferredParameterListContext;
pub use super::function_statement_context::FunctionStatementContext;
pub use super::group_expr_context::GroupExprContext;
pub use super::id_key_context::IdKeyContext;
pub use super::import_cls_context::ImportClsContext;
pub use super::import_pack_context::ImportPackContext;
pub use super::index_expr_context::IndexExprContext;
pub use super::lambda_expr_context::LambdaExprContext;
pub use super::lambda_parameters_context::LambdaParametersContext;
pub use super::left_asso_context::LeftAssoContext;
pub use super::left_hand_side_context::LeftHandSideContext;
pub use super::list_expr_context::ListExprContext;
pub use super::list_items_context::ListItemsContext;
pub use super::literal_context::LiteralContext;
pub use super::local_variable_declaration_context::LocalVariableDeclarationContext;
pub use super::local_variable_declaration_statement_context::LocalVariableDeclarationStatementContext;
pub use super::macro_statement_context::MacroStatementContext;
pub use super::map_entries_context::MapEntriesContext;
pub use super::map_entry_context::MapEntryContext;
pub use super::map_expr_context::MapExprContext;
pub use super::method_access_context::MethodAccessContext;
pub use super::method_invoke_context::MethodInvokeContext;
pub use super::new_empty_arr_expr_context::NewEmptyArrExprContext;
pub use super::new_init_arr_expr_context::NewInitArrExprContext;
pub use super::new_obj_expr_context::NewObjExprContext;
pub use super::node::Node;
pub use super::non_expression_statement_context::NonExpressionStatementContext;
pub use super::op_id_context::OpIdContext;
pub use super::prefix_express_context::PrefixExpressContext;
pub use super::primary_context::PrimaryContext;
pub use super::primitive_type_context::PrimitiveTypeContext;
pub use super::program_context::ProgramContext;
pub use super::ql_if_context::QlIfContext;
pub use super::quote_string_key_context::QuoteStringKeyContext;
pub use super::return_statement_context::ReturnStatementContext;
pub use super::single_index_context::SingleIndexContext;
pub use super::slice_index_context::SliceIndexContext;
pub use super::string_expression_context::StringExpressionContext;
pub use super::string_key_context::StringKeyContext;
pub use super::suffix_express_context::SuffixExpressContext;
pub use super::switch_case_groups_context::SwitchCaseGroupsContext;
pub use super::switch_expr_context::SwitchExprContext;
pub use super::switch_expr_group_context::SwitchExprGroupContext;
pub use super::switch_expression_label_context::SwitchExpressionLabelContext;
pub use super::switch_label_context::SwitchLabelContext;
pub use super::switch_labels_context::SwitchLabelsContext;
pub use super::switch_statement_group_context::SwitchStatementGroupContext;
pub use super::ternary_expr_context::TernaryExprContext;
pub use super::then_body_context::ThenBodyContext;
pub use super::throw_statement_context::ThrowStatementContext;
pub use super::traditional_for_statement_context::TraditionalForStatementContext;
pub use super::try_catch_context::TryCatchContext;
pub use super::try_catch_expr_context::TryCatchExprContext;
pub use super::try_catches_context::TryCatchesContext;
pub use super::try_finally_context::TryFinallyContext;
pub use super::type_expr_context::TypeExprContext;
pub use super::var_id_context::VarIdContext;
pub use super::var_id_expr_context::VarIdExprContext;
pub use super::variable_declarator_context::VariableDeclaratorContext;
pub use super::variable_declarator_id_context::VariableDeclaratorIdContext;
pub use super::variable_declarator_list_context::VariableDeclaratorListContext;
pub use super::variable_initializer_context::VariableInitializerContext;
pub use super::variable_initializer_list_context::VariableInitializerListContext;
pub use super::while_statement_context::WhileStatementContext;

/// 创建和承载 QLParser 语法树对象的工厂身份。
///
/// 对应 Java: `com.alibaba.qlexpress4.aparser.SyntaxTreeFactory`。
pub struct SyntaxTreeFactory;

// ---------------------------------------------------------------------------
// Context structs (one per Java QLParser inner class).
// ---------------------------------------------------------------------------

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

impl HasChildren for Node {
    fn children(&self) -> Vec<ChildRef<'_>> {
        macro_rules! dispatch {
            ($($variant:ident),* $(,)?) => {
                match self {
                    $(Node::$variant(c) => <_ as HasChildren>::children(c)),*
                }
            };
        }
        dispatch!(
            Program,
            BlockStatements,
            LocalVariableDeclarationStatement,
            ThrowStatement,
            WhileStatement,
            TraditionalForStatement,
            ForEachStatement,
            FunctionStatement,
            MacroStatement,
            BreakContinueStatement,
            ReturnStatement,
            EmptyStatement,
            ExpressionStatement,
            NonExpressionStatement,
            LocalVariableDeclaration,
            ForInit,
            VariableDeclaratorList,
            VariableDeclarator,
            VariableDeclaratorId,
            VariableInitializer,
            ArrayInitializer,
            VariableInitializerList,
            DeclType,
            DeclTypeNoArr,
            PrimitiveType,
            ClsType,
            Dims,
            DimExprs,
            Expression,
            LeftHandSide,
            AssignOperator,
            TernaryExpr,
            BaseExpr,
            LeftAsso,
            Binaryop,
            Primary,
            PrefixExpress,
            SuffixExpress,
            ConstExpr,
            CastExpr,
            GroupExpr,
            NewObjExpr,
            NewEmptyArrExpr,
            NewInitArrExpr,
            VarIdExpr,
            TypeExpr,
            ListExpr,
            ListItems,
            MapExpr,
            BlockExpr,
            ContextSelectExpr,
            QlIf,
            ThenBody,
            ElseBody,
            SwitchExpr,
            SwitchCaseGroups,
            SwitchStatementGroup,
            SwitchExprGroup,
            SwitchLabels,
            SwitchLabel,
            SwitchExpressionLabel,
            ExpressionList,
            TryCatchExpr,
            TryCatches,
            TryCatch,
            CatchParams,
            TryFinally,
            MapEntries,
            MapEntry,
            ClsValue,
            EValue,
            IdKey,
            StringKey,
            QuoteStringKey,
            MethodInvoke,
            FieldAccess,
            MethodAccess,
            IndexExpr,
            CustomPath,
            FieldId,
            SingleIndex,
            SliceIndex,
            ArgumentList,
            Literal,
            DoubleQuoteStringLiteral,
            StringExpression,
            BoolenLiteral,
            LambdaExpr,
            LambdaParameters,
            FormalOrInferredParameterList,
            FormalOrInferredParameter,
            ImportCls,
            ImportPack,
            OpId,
            VarId
        )
    }
}
