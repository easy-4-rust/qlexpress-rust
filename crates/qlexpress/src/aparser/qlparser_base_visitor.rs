//! 语法树 Visitor 基 trait,对应 Java `com.alibaba.qlexpress4.aparser.QLParserBaseVisitor`。
//! 职责:定义 95 个 visit 默认方法(默认访问全部孩子并返回最后一个孩子的结果,
//! 终结符返回 T::default() 即 Java 的 null)。95 个默认方法保持不动。
//! 本文件由 `syntax_tree.rs` 拆分而来(SPEC §5.5 一类一文件),仅移动代码与补充中文注释,行为完全一致。

use super::rule_context::{ChildRef, HasChildren};
use super::syntax_tree_factory::{
    ArgumentListContext, ArrayInitializerContext, AssignOperatorContext, BaseExprContext,
    BinaryopContext, BlockExprContext, BlockStatementsContext, BoolenLiteralContext,
    BreakContinueStatementContext, CastExprContext, CatchParamsContext, ClsTypeContext,
    ClsValueContext, ConstExprContext, ContextSelectExprContext, CustomPathContext,
    DeclTypeContext, DeclTypeNoArrContext, DimExprsContext, DimsContext,
    DoubleQuoteStringLiteralContext, EValueContext, ElseBodyContext, EmptyStatementContext,
    ExpressionContext, ExpressionListContext, ExpressionStatementContext, FieldAccessContext,
    FieldIdContext, ForEachStatementContext, ForInitContext, FormalOrInferredParameterContext,
    FormalOrInferredParameterListContext, FunctionStatementContext, GroupExprContext, IdKeyContext,
    ImportClsContext, ImportPackContext, IndexExprContext, LambdaExprContext,
    LambdaParametersContext, LeftAssoContext, LeftHandSideContext, ListExprContext,
    ListItemsContext, LiteralContext, LocalVariableDeclarationContext,
    LocalVariableDeclarationStatementContext, MacroStatementContext, MapEntriesContext,
    MapEntryContext, MapExprContext, MethodAccessContext, MethodInvokeContext,
    NewEmptyArrExprContext, NewInitArrExprContext, NewObjExprContext, Node,
    NonExpressionStatementContext, OpIdContext, PrefixExpressContext, PrimaryContext,
    PrimitiveTypeContext, ProgramContext, QlIfContext, QuoteStringKeyContext,
    ReturnStatementContext, SingleIndexContext, SliceIndexContext, StringExpressionContext,
    StringKeyContext, SuffixExpressContext, SwitchCaseGroupsContext, SwitchExprContext,
    SwitchExprGroupContext, SwitchExpressionLabelContext, SwitchLabelContext, SwitchLabelsContext,
    SwitchStatementGroupContext, TernaryExprContext, ThenBodyContext, ThrowStatementContext,
    TraditionalForStatementContext, TryCatchContext, TryCatchExprContext, TryCatchesContext,
    TryFinallyContext, TypeExprContext, VarIdContext, VarIdExprContext, VariableDeclaratorContext,
    VariableDeclaratorIdContext, VariableDeclaratorListContext, VariableInitializerContext,
    VariableInitializerListContext, WhileStatementContext,
};
use super::terminal_node::TerminalNode;

// ---------------------------------------------------------------------------
// Visitor: mirrors QLParserBaseVisitor (default = visit children, returning
// the last child's result; terminals yield T::default(), Java's null).
// ---------------------------------------------------------------------------

macro_rules! default_visit_methods {
    ($($name:ident ( $ty:ty ) ;)*) => {
        $(
            /// Java `QLParserBaseVisitor` default: visit children.
            fn $name(&mut self, ctx: &$ty) -> Self::T {
                self.visit_children_of(ctx)
            }
        )*
    };
}

/// 语法树 Visitor 基 trait。对应 Java: com.alibaba.qlexpress4.aparser.QLParserBaseVisitor(95 个 visit 默认方法)
/// Java `QLParserBaseVisitor<T>`. `T` must be [`Default`]; the default
/// `T::default()` plays the role of Java's `null` result.
pub trait Visitor {
    /// Visit result type (Java `T`); `()` for `QLParserBaseVisitor<Void>`.
    type T: Default;

    /// Java `visitTerminal` (returns null / default).
    fn visit_terminal(&mut self, _node: &TerminalNode) -> Self::T {
        Self::T::default()
    }

    /// Java `visitChildren`: visit every child in order, returning the last
    /// child's result.
    fn visit_children_of(&mut self, ctx: &dyn HasChildren) -> Self::T {
        let mut result = Self::T::default();
        for child in ctx.children() {
            result = match child {
                ChildRef::Node(node) => node.accept(self),
                ChildRef::Term(term) => self.visit_terminal(term),
            };
        }
        result
    }

    /// Java `visitChildren(RuleContext)`.
    fn visit_children(&mut self, node: &Node) -> Self::T {
        self.visit_children_of(node)
    }

    default_visit_methods! {
        visit_program(ProgramContext);
        visit_block_statements(BlockStatementsContext);
        visit_local_variable_declaration_statement(LocalVariableDeclarationStatementContext);
        visit_throw_statement(ThrowStatementContext);
        visit_while_statement(WhileStatementContext);
        visit_traditional_for_statement(TraditionalForStatementContext);
        visit_for_each_statement(ForEachStatementContext);
        visit_function_statement(FunctionStatementContext);
        visit_macro_statement(MacroStatementContext);
        visit_break_continue_statement(BreakContinueStatementContext);
        visit_return_statement(ReturnStatementContext);
        visit_empty_statement(EmptyStatementContext);
        visit_expression_statement(ExpressionStatementContext);
        visit_non_expression_statement(NonExpressionStatementContext);
        visit_local_variable_declaration(LocalVariableDeclarationContext);
        visit_for_init(ForInitContext);
        visit_variable_declarator_list(VariableDeclaratorListContext);
        visit_variable_declarator(VariableDeclaratorContext);
        visit_variable_declarator_id(VariableDeclaratorIdContext);
        visit_variable_initializer(VariableInitializerContext);
        visit_array_initializer(ArrayInitializerContext);
        visit_variable_initializer_list(VariableInitializerListContext);
        visit_decl_type(DeclTypeContext);
        visit_decl_type_no_arr(DeclTypeNoArrContext);
        visit_primitive_type(PrimitiveTypeContext);
        visit_cls_type(ClsTypeContext);
        visit_dims(DimsContext);
        visit_dim_exprs(DimExprsContext);
        visit_expression(ExpressionContext);
        visit_left_hand_side(LeftHandSideContext);
        visit_assign_operator(AssignOperatorContext);
        visit_ternary_expr(TernaryExprContext);
        visit_base_expr(BaseExprContext);
        visit_left_asso(LeftAssoContext);
        visit_binaryop(BinaryopContext);
        visit_primary(PrimaryContext);
        visit_prefix_express(PrefixExpressContext);
        visit_suffix_express(SuffixExpressContext);
        visit_const_expr(ConstExprContext);
        visit_cast_expr(CastExprContext);
        visit_group_expr(GroupExprContext);
        visit_new_obj_expr(NewObjExprContext);
        visit_new_empty_arr_expr(NewEmptyArrExprContext);
        visit_new_init_arr_expr(NewInitArrExprContext);
        visit_var_id_expr(VarIdExprContext);
        visit_type_expr(TypeExprContext);
        visit_list_expr(ListExprContext);
        visit_list_items(ListItemsContext);
        visit_map_expr(MapExprContext);
        visit_block_expr(BlockExprContext);
        visit_context_select_expr(ContextSelectExprContext);
        visit_ql_if(QlIfContext);
        visit_then_body(ThenBodyContext);
        visit_else_body(ElseBodyContext);
        visit_switch_expr(SwitchExprContext);
        visit_switch_case_groups(SwitchCaseGroupsContext);
        visit_switch_statement_group(SwitchStatementGroupContext);
        visit_switch_expr_group(SwitchExprGroupContext);
        visit_switch_labels(SwitchLabelsContext);
        visit_switch_label(SwitchLabelContext);
        visit_switch_expression_label(SwitchExpressionLabelContext);
        visit_expression_list(ExpressionListContext);
        visit_try_catch_expr(TryCatchExprContext);
        visit_try_catches(TryCatchesContext);
        visit_try_catch(TryCatchContext);
        visit_catch_params(CatchParamsContext);
        visit_try_finally(TryFinallyContext);
        visit_map_entries(MapEntriesContext);
        visit_map_entry(MapEntryContext);
        visit_cls_value(ClsValueContext);
        visit_e_value(EValueContext);
        visit_id_key(IdKeyContext);
        visit_string_key(StringKeyContext);
        visit_quote_string_key(QuoteStringKeyContext);
        visit_method_invoke(MethodInvokeContext);
        visit_field_access(FieldAccessContext);
        visit_method_access(MethodAccessContext);
        visit_index_expr(IndexExprContext);
        visit_custom_path(CustomPathContext);
        visit_field_id(FieldIdContext);
        visit_single_index(SingleIndexContext);
        visit_slice_index(SliceIndexContext);
        visit_argument_list(ArgumentListContext);
        visit_literal(LiteralContext);
        visit_double_quote_string_literal(DoubleQuoteStringLiteralContext);
        visit_string_expression(StringExpressionContext);
        visit_boolen_literal(BoolenLiteralContext);
        visit_lambda_expr(LambdaExprContext);
        visit_lambda_parameters(LambdaParametersContext);
        visit_formal_or_inferred_parameter_list(FormalOrInferredParameterListContext);
        visit_formal_or_inferred_parameter(FormalOrInferredParameterContext);
        visit_import_cls(ImportClsContext);
        visit_import_pack(ImportPackContext);
        visit_op_id(OpIdContext);
        visit_var_id(VarIdContext);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::aparser::token::{self, Token};

    fn tok(text: &str) -> Token {
        Token::new(token::ID as i32, text, 0, 0, 1, 0)
    }

    /// Counts visited nodes; exercises the default traversal.
    struct Counter {
        count: usize,
    }

    impl Visitor for Counter {
        type T = ();

        fn visit_children_of(&mut self, ctx: &dyn HasChildren) -> Self::T {
            self.count += 1;
            // default traversal
            for child in ctx.children() {
                match child {
                    ChildRef::Node(node) => node.accept(self),
                    ChildRef::Term(term) => {
                        self.visit_terminal(term);
                    }
                }
            }
        }
    }

    #[test]
    fn default_visitor_traverses_in_source_order() {
        // a = b  ->  Expression{left, assign op, expression}
        let tree = Node::Expression(ExpressionContext {
            left: Some(Box::new(Node::LeftHandSide(LeftHandSideContext {
                var_id: Box::new(Node::VarId(VarIdContext {
                    token: TerminalNode::new(tok("a")),
                })),
                lparen: None,
                argument_list: None,
                path_parts: vec![],
            }))),
            assign_operator: Some(Box::new(Node::AssignOperator(AssignOperatorContext {
                token: TerminalNode::new(Token::new(token::EQ as i32, "=", 2, 2, 1, 2)),
            }))),
            expression: Some(Box::new(Node::Expression(ExpressionContext {
                left: None,
                assign_operator: None,
                expression: None,
                ternary: Some(Box::new(Node::TernaryExpr(TernaryExprContext {
                    condition: Box::new(Node::BaseExpr(BaseExprContext {
                        primary: Box::new(Node::Primary(PrimaryContext {
                            prefix: None,
                            pathable: Some(Box::new(Node::VarIdExpr(VarIdExprContext {
                                var_id: Box::new(Node::VarId(VarIdContext {
                                    token: TerminalNode::new(tok("b")),
                                })),
                                lparen: None,
                                argument_list: None,
                            }))),
                            path_parts: vec![],
                            suffix: None,
                            non_pathable: None,
                        })),
                        left_assos: vec![],
                    })),
                    question: None,
                    then_expr: None,
                    else_expr: None,
                }))),
            }))),
            ternary: None,
        });
        let mut counter = Counter { count: 0 };
        tree.accept(&mut counter);
        // Expression, LeftHandSide, VarId, AssignOperator, Expression,
        // TernaryExpr, BaseExpr, Primary, VarIdExpr, VarId
        assert_eq!(counter.count, 10);
        assert_eq!(tree.text(), "a=b");
        assert!(tree.start_token().is_some());
    }

    #[test]
    fn terminal_text_and_positions() {
        let term = TerminalNode::new(Token::new(token::ID as i32, "x", 3, 3, 2, 5));
        assert_eq!(term.text(), "x");
        assert_eq!(term.symbol().line(), 2);
        assert_eq!(term.symbol().char_position_in_line(), 5);
    }
}
