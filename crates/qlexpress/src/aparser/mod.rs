//! Lexer/parser package mirroring Java `com.alibaba.qlexpress4.aparser`.
//!
//! Stage 0 delivered the enums/structs referenced by `InitOptions`;
//! Stage 1 added the token model ([`token`]), the scanner ([`qlexer`]) and
//! the [`parser_operator_manager`] contract. Stage 2 adds the syntax tree
//! ([`syntax_tree_factory`]), the recursive-descent parser ([`qlparser`]), the
//! compile-time visitors ([`check_visitor`], [`out_var_names_visitor`], [`out_var_attrs_visitor`],
//! [`scope_stack_visitor`]), the import resolver ([`import_manager`]),
//! macro/scope helpers and the compile cache.

pub mod built_in_types_set;
pub mod check_visitor;
pub mod child_ref;
pub mod compile_cache;
pub mod compile_cache_store;
pub use crate::compiletimefunction as compile_time_function;
pub mod exist_stack;
pub mod exist_var_stack;
pub mod generator_scope;
pub mod import_manager;
pub mod import_scope;
pub mod instruction_context;
/// `interpolation_mode` 子模块。
pub mod interpolation_mode;
pub mod load_part_qualified_result;
pub mod macro_define;
pub mod op_type;
pub mod operator_factory;
pub mod operator_manager;
pub mod out_function_visitor;
pub mod out_var_attrs_visitor;
pub mod out_var_names_visitor;
pub mod parse_fail;
pub mod parse_tree;
pub mod parser_operator_manager;
pub mod q_compile_cache;
pub mod q_lexer;
pub mod ql_import;
pub mod ql_parser;
pub mod ql_parser_base_visitor;
pub use q_lexer as qlexer;
pub use ql_parser as qlparser;
pub use ql_parser_base_visitor as qlparser_base_visitor;
/// Java QLParser 内部 AST 类型 `ArgumentListContext`。
pub mod argument_list_context;
/// Java QLParser 内部 AST 类型 `ArrayInitializerContext`。
pub mod array_initializer_context;
/// Java QLParser 内部 AST 类型 `AssignOperatorContext`。
pub mod assign_operator_context;
/// Java QLParser 内部 AST 类型 `BaseExprContext`。
pub mod base_expr_context;
/// Java QLParser 内部 AST 类型 `BinaryopContext`。
pub mod binaryop_context;
/// Java QLParser 内部 AST 类型 `BlockExprContext`。
pub mod block_expr_context;
/// Java QLParser 内部 AST 类型 `BlockStatementsContext`。
pub mod block_statements_context;
/// Java QLParser 内部 AST 类型 `BoolenLiteralContext`。
pub mod boolen_literal_context;
/// Java QLParser 内部 AST 类型 `BreakContinueStatementContext`。
pub mod break_continue_statement_context;
/// Java QLParser 内部 AST 类型 `CastExprContext`。
pub mod cast_expr_context;
/// Java QLParser 内部 AST 类型 `CatchParamsContext`。
pub mod catch_params_context;
/// Java QLParser 内部 AST 类型 `ChainKind`。
pub mod chain_kind;
/// Java QLParser 内部 AST 类型 `ClsTypeContext`。
pub mod cls_type_context;
/// Java QLParser 内部 AST 类型 `ClsValueContext`。
pub mod cls_value_context;
/// Java QLParser 内部 AST 类型 `ConstExprContext`。
pub mod const_expr_context;
/// Java QLParser 内部 AST 类型 `ContextSelectExprContext`。
pub mod context_select_expr_context;
/// Java QLParser 内部 AST 类型 `CustomPathContext`。
pub mod custom_path_context;
/// Java QLParser 内部 AST 类型 `DeclTypeContext`。
pub mod decl_type_context;
/// Java QLParser 内部 AST 类型 `DeclTypeNoArrContext`。
pub mod decl_type_no_arr_context;
/// Java QLParser 内部 AST 类型 `DimExprsContext`。
pub mod dim_exprs_context;
/// Java QLParser 内部 AST 类型 `DimsContext`。
pub mod dims_context;
/// Java QLParser 内部 AST 类型 `DoubleQuoteStringLiteralContext`。
pub mod double_quote_string_literal_context;
/// Java QLParser 内部 AST 类型 `DyStrPart`。
pub mod dy_str_part;
/// Java QLParser 内部 AST 类型 `EValueContext`。
pub mod e_value_context;
/// Java QLParser 内部 AST 类型 `ElseBodyContext`。
pub mod else_body_context;
/// Java QLParser 内部 AST 类型 `EmptyStatementContext`。
pub mod empty_statement_context;
/// Java QLParser 内部 AST 类型 `ExpressionContext`。
pub mod expression_context;
/// Java QLParser 内部 AST 类型 `ExpressionListContext`。
pub mod expression_list_context;
/// Java QLParser 内部 AST 类型 `ExpressionStatementContext`。
pub mod expression_statement_context;
/// Java QLParser 内部 AST 类型 `FieldAccessContext`。
pub mod field_access_context;
/// Java QLParser 内部 AST 类型 `FieldIdContext`。
pub mod field_id_context;
/// Java QLParser 内部 AST 类型 `ForEachStatementContext`。
pub mod for_each_statement_context;
/// Java QLParser 内部 AST 类型 `ForInitContext`。
pub mod for_init_context;
/// Java QLParser 内部 AST 类型 `FormalOrInferredParameterContext`。
pub mod formal_or_inferred_parameter_context;
/// Java QLParser 内部 AST 类型 `FormalOrInferredParameterListContext`。
pub mod formal_or_inferred_parameter_list_context;
/// Java QLParser 内部 AST 类型 `FunctionStatementContext`。
pub mod function_statement_context;
/// Java QLParser 内部 AST 类型 `GroupExprContext`。
pub mod group_expr_context;
/// Java QLParser 内部 AST 类型 `IdKeyContext`。
pub mod id_key_context;
/// Java QLParser 内部 AST 类型 `ImportClsContext`。
pub mod import_cls_context;
/// Java QLParser 内部 AST 类型 `ImportPackContext`。
pub mod import_pack_context;
/// Java QLParser 内部 AST 类型 `IndexExprContext`。
pub mod index_expr_context;
/// Java QLParser 内部 AST 类型 `LambdaExprContext`。
pub mod lambda_expr_context;
/// Java QLParser 内部 AST 类型 `LambdaParametersContext`。
pub mod lambda_parameters_context;
/// Java QLParser 内部 AST 类型 `LeftAssoContext`。
pub mod left_asso_context;
/// Java QLParser 内部 AST 类型 `LeftHandSideContext`。
pub mod left_hand_side_context;
/// Java QLParser 内部 AST 类型 `ListExprContext`。
pub mod list_expr_context;
/// Java QLParser 内部 AST 类型 `ListItemsContext`。
pub mod list_items_context;
/// Java QLParser 内部 AST 类型 `LiteralContext`。
pub mod literal_context;
/// Java QLParser 内部 AST 类型 `LocalVariableDeclarationContext`。
pub mod local_variable_declaration_context;
/// Java QLParser 内部 AST 类型 `LocalVariableDeclarationStatementContext`。
pub mod local_variable_declaration_statement_context;
/// Java QLParser 内部 AST 类型 `MacroStatementContext`。
pub mod macro_statement_context;
/// Java QLParser 内部 AST 类型 `MapEntriesContext`。
pub mod map_entries_context;
/// Java QLParser 内部 AST 类型 `MapEntryContext`。
pub mod map_entry_context;
/// Java QLParser 内部 AST 类型 `MapExprContext`。
pub mod map_expr_context;
/// Java QLParser 内部 AST 类型 `MethodAccessContext`。
pub mod method_access_context;
/// Java QLParser 内部 AST 类型 `MethodInvokeContext`。
pub mod method_invoke_context;
/// Java QLParser 内部 AST 类型 `NewEmptyArrExprContext`。
pub mod new_empty_arr_expr_context;
/// Java QLParser 内部 AST 类型 `NewInitArrExprContext`。
pub mod new_init_arr_expr_context;
/// Java QLParser 内部 AST 类型 `NewObjExprContext`。
pub mod new_obj_expr_context;
/// Java QLParser 内部 AST 类型 `Node`。
pub mod node;
/// Java QLParser 内部 AST 类型 `NonExpressionStatementContext`。
pub mod non_expression_statement_context;
/// Java QLParser 内部 AST 类型 `OpIdContext`。
pub mod op_id_context;
/// Java QLParser 内部 AST 类型 `PrefixExpressContext`。
pub mod prefix_express_context;
/// Java QLParser 内部 AST 类型 `PrimaryContext`。
pub mod primary_context;
/// Java QLParser 内部 AST 类型 `PrimitiveTypeContext`。
pub mod primitive_type_context;
/// Java QLParser 内部 AST 类型 `ProgramContext`。
pub mod program_context;
/// Java QLParser 内部 AST 类型 `QlIfContext`。
pub mod ql_if_context;
/// Java QLParser 内部 AST 类型 `QuoteStringKeyContext`。
pub mod quote_string_key_context;
pub mod qvm_instruction_visitor;
/// Java QLParser 内部 AST 类型 `ReturnStatementContext`。
pub mod return_statement_context;
pub mod rule_context;
pub mod scope_stack;
pub mod scope_stack_visitor;
/// Java QLParser 内部 AST 类型 `SingleIndexContext`。
pub mod single_index_context;
/// Java QLParser 内部 AST 类型 `SliceIndexContext`。
pub mod slice_index_context;
/// Java QLParser 内部 AST 类型 `StringExpressionContext`。
pub mod string_expression_context;
/// Java QLParser 内部 AST 类型 `StringKeyContext`。
pub mod string_key_context;
/// Java QLParser 内部 AST 类型 `SuffixExpressContext`。
pub mod suffix_express_context;
/// Java QLParser 内部 AST 类型 `SwitchCaseGroupsContext`。
pub mod switch_case_groups_context;
/// Java QLParser 内部 AST 类型 `SwitchExprContext`。
pub mod switch_expr_context;
/// Java QLParser 内部 AST 类型 `SwitchExprGroupContext`。
pub mod switch_expr_group_context;
/// Java QLParser 内部 AST 类型 `SwitchExpressionLabelContext`。
pub mod switch_expression_label_context;
/// Java QLParser 内部 AST 类型 `SwitchLabelContext`。
pub mod switch_label_context;
/// Java QLParser 内部 AST 类型 `SwitchLabelsContext`。
pub mod switch_labels_context;
/// Java QLParser 内部 AST 类型 `SwitchStatementGroupContext`。
pub mod switch_statement_group_context;
pub mod syntax_tree_factory;
pub mod terminal_node;
/// Java QLParser 内部 AST 类型 `TernaryExprContext`。
pub mod ternary_expr_context;
/// Java QLParser 内部 AST 类型 `ThenBodyContext`。
pub mod then_body_context;
/// Java QLParser 内部 AST 类型 `ThrowStatementContext`。
pub mod throw_statement_context;
pub mod token;
pub mod trace_expression_visitor;
/// Java QLParser 内部 AST 类型 `TraditionalForStatementContext`。
pub mod traditional_for_statement_context;
/// Java QLParser 内部 AST 类型 `TryCatchContext`。
pub mod try_catch_context;
/// Java QLParser 内部 AST 类型 `TryCatchExprContext`。
pub mod try_catch_expr_context;
/// Java QLParser 内部 AST 类型 `TryCatchesContext`。
pub mod try_catches_context;
/// Java QLParser 内部 AST 类型 `TryFinallyContext`。
pub mod try_finally_context;
/// Java QLParser 内部 AST 类型 `TypeExprContext`。
pub mod type_expr_context;
/// Java QLParser 内部 AST 类型 `VarIdContext`。
pub mod var_id_context;
/// Java QLParser 内部 AST 类型 `VarIdExprContext`。
pub mod var_id_expr_context;
/// Java QLParser 内部 AST 类型 `VariableDeclaratorContext`。
pub mod variable_declarator_context;
/// Java QLParser 内部 AST 类型 `VariableDeclaratorIdContext`。
pub mod variable_declarator_id_context;
/// Java QLParser 内部 AST 类型 `VariableDeclaratorListContext`。
pub mod variable_declarator_list_context;
/// Java QLParser 内部 AST 类型 `VariableInitializerContext`。
pub mod variable_initializer_context;
/// Java QLParser 内部 AST 类型 `VariableInitializerListContext`。
pub mod variable_initializer_list_context;
/// Java QLParser 内部 AST 类型 `WhileStatementContext`。
pub mod while_statement_context;

pub use built_in_types_set::{
    get_cls, BuiltInType, BOOLEAN, BYTE, CHAR, DOUBLE, FLOAT, INT, LONG, SHORT,
};
pub use check_visitor::CheckVisitor;
pub use child_ref::ChildRef;
pub use compile_cache::{CompileCache, QCompileCache, ScriptCompileCache};
pub use compile_cache_store::CompileCacheStore;
pub use compile_time_function::{CodeGenerator, CompileTimeFunction};
pub use exist_stack::{ExistStack, ExistVarStack};
pub use generator_scope::GeneratorScope;
pub use import_manager::{ImportManager, ImportScope, LoadPartQualifiedResult, QLImport};
pub use interpolation_mode::InterpolationMode;
pub use macro_define::MacroDefine;
pub use op_type::OpType;
pub use operator_factory::{OperatorFactory, OperatorManager};
pub use out_function_visitor::OutFunctionVisitor;
pub use out_var_attrs_visitor::OutVarAttrsVisitor;
pub use out_var_names_visitor::OutVarNamesVisitor;
pub use parser_operator_manager::ParserOperatorManager;
pub use qlexer::{tokenize, tokenize_with_limit};
pub use qlparser::{build_tree, build_tree_from_tokens, QLParser};
pub use qlparser_base_visitor::Visitor;
pub use qvm_instruction_visitor::{
    compile_script, CompileTimeFunctions, Context, InstructionMacroDefine, InstructionScope,
    QvmInstructionVisitor, SharedInstruction, UserDefineFunctions,
};
pub use rule_context::{HasChildren, RuleContext};
pub use scope_stack::ScopeStack;
pub use scope_stack_visitor::ScopedVisitor;
pub use syntax_tree_factory::Node;
pub use terminal_node::TerminalNode;
pub use token::Token;
pub use trace_expression_visitor::TraceExpressionVisitor;
