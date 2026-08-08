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

include!("syntax_tree_factory/statements_and_expressions.rs");
include!("syntax_tree_factory/control_flow.rs");
include!("syntax_tree_factory/path_and_literals.rs");
include!("syntax_tree_factory/node_dispatch.rs");
