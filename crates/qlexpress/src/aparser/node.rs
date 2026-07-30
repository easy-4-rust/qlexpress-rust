//! QLParser 语法树类型；由 Java 生成式内部类型按对象边界拆分。

use super::argument_list_context::ArgumentListContext;
use super::array_initializer_context::ArrayInitializerContext;
use super::assign_operator_context::AssignOperatorContext;
use super::base_expr_context::BaseExprContext;
use super::binaryop_context::BinaryopContext;
use super::block_expr_context::BlockExprContext;
use super::block_statements_context::BlockStatementsContext;
use super::boolen_literal_context::BoolenLiteralContext;
use super::break_continue_statement_context::BreakContinueStatementContext;
use super::cast_expr_context::CastExprContext;
use super::catch_params_context::CatchParamsContext;
use super::cls_type_context::ClsTypeContext;
use super::cls_value_context::ClsValueContext;
use super::const_expr_context::ConstExprContext;
use super::context_select_expr_context::ContextSelectExprContext;
use super::custom_path_context::CustomPathContext;
use super::decl_type_context::DeclTypeContext;
use super::decl_type_no_arr_context::DeclTypeNoArrContext;
use super::dim_exprs_context::DimExprsContext;
use super::dims_context::DimsContext;
use super::double_quote_string_literal_context::DoubleQuoteStringLiteralContext;
use super::e_value_context::EValueContext;
use super::else_body_context::ElseBodyContext;
use super::empty_statement_context::EmptyStatementContext;
use super::expression_context::ExpressionContext;
use super::expression_list_context::ExpressionListContext;
use super::expression_statement_context::ExpressionStatementContext;
use super::field_access_context::FieldAccessContext;
use super::field_id_context::FieldIdContext;
use super::for_each_statement_context::ForEachStatementContext;
use super::for_init_context::ForInitContext;
use super::formal_or_inferred_parameter_context::FormalOrInferredParameterContext;
use super::formal_or_inferred_parameter_list_context::FormalOrInferredParameterListContext;
use super::function_statement_context::FunctionStatementContext;
use super::group_expr_context::GroupExprContext;
use super::id_key_context::IdKeyContext;
use super::import_cls_context::ImportClsContext;
use super::import_pack_context::ImportPackContext;
use super::index_expr_context::IndexExprContext;
use super::lambda_expr_context::LambdaExprContext;
use super::lambda_parameters_context::LambdaParametersContext;
use super::left_asso_context::LeftAssoContext;
use super::left_hand_side_context::LeftHandSideContext;
use super::list_expr_context::ListExprContext;
use super::list_items_context::ListItemsContext;
use super::literal_context::LiteralContext;
use super::local_variable_declaration_context::LocalVariableDeclarationContext;
use super::local_variable_declaration_statement_context::LocalVariableDeclarationStatementContext;
use super::macro_statement_context::MacroStatementContext;
use super::map_entries_context::MapEntriesContext;
use super::map_entry_context::MapEntryContext;
use super::map_expr_context::MapExprContext;
use super::method_access_context::MethodAccessContext;
use super::method_invoke_context::MethodInvokeContext;
use super::new_empty_arr_expr_context::NewEmptyArrExprContext;
use super::new_init_arr_expr_context::NewInitArrExprContext;
use super::new_obj_expr_context::NewObjExprContext;
use super::non_expression_statement_context::NonExpressionStatementContext;
use super::op_id_context::OpIdContext;
use super::prefix_express_context::PrefixExpressContext;
use super::primary_context::PrimaryContext;
use super::primitive_type_context::PrimitiveTypeContext;
use super::program_context::ProgramContext;
use super::ql_if_context::QlIfContext;
use super::quote_string_key_context::QuoteStringKeyContext;
use super::return_statement_context::ReturnStatementContext;
use super::single_index_context::SingleIndexContext;
use super::slice_index_context::SliceIndexContext;
use super::string_expression_context::StringExpressionContext;
use super::string_key_context::StringKeyContext;
use super::suffix_express_context::SuffixExpressContext;
use super::switch_case_groups_context::SwitchCaseGroupsContext;
use super::switch_expr_context::SwitchExprContext;
use super::switch_expr_group_context::SwitchExprGroupContext;
use super::switch_expression_label_context::SwitchExpressionLabelContext;
use super::switch_label_context::SwitchLabelContext;
use super::switch_labels_context::SwitchLabelsContext;
use super::switch_statement_group_context::SwitchStatementGroupContext;
use super::ternary_expr_context::TernaryExprContext;
use super::then_body_context::ThenBodyContext;
use super::throw_statement_context::ThrowStatementContext;
use super::traditional_for_statement_context::TraditionalForStatementContext;
use super::try_catch_context::TryCatchContext;
use super::try_catch_expr_context::TryCatchExprContext;
use super::try_catches_context::TryCatchesContext;
use super::try_finally_context::TryFinallyContext;
use super::type_expr_context::TypeExprContext;
use super::var_id_context::VarIdContext;
use super::var_id_expr_context::VarIdExprContext;
use super::variable_declarator_context::VariableDeclaratorContext;
use super::variable_declarator_id_context::VariableDeclaratorIdContext;
use super::variable_declarator_list_context::VariableDeclaratorListContext;
use super::variable_initializer_context::VariableInitializerContext;
use super::variable_initializer_list_context::VariableInitializerListContext;
use super::while_statement_context::WhileStatementContext;

/// 语法树节点:一个变体对应 Java `QLParser` 的一个内部 *Context 类。
/// A syntax tree node, mirroring the Java `QLParser.*Context` hierarchy
/// (flattened: Java's abstract intermediates `BlockStatementContext`,
/// `PathPartContext`, `MapKeyContext`, ... are represented directly by the
/// concrete variants).
///
/// 下列 Java 抽象分组类型没有实例字段或独立执行逻辑，Rust 以 `Node`
/// 枚举的受限变体集合保留其闭合类型语义：
///
/// - 对应 Java: `com.alibaba.qlexpress4.aparser.QLParser.BlockStatementContext`
/// - 对应 Java: `com.alibaba.qlexpress4.aparser.QLParser.PrimaryNoFixPathableContext`
/// - 对应 Java: `com.alibaba.qlexpress4.aparser.QLParser.PrimaryNoFixNonPathableContext`
/// - 对应 Java: `com.alibaba.qlexpress4.aparser.QLParser.MapValueContext`
/// - 对应 Java: `com.alibaba.qlexpress4.aparser.QLParser.MapKeyContext`
/// - 对应 Java: `com.alibaba.qlexpress4.aparser.QLParser.PathPartContext`
/// - 对应 Java: `com.alibaba.qlexpress4.aparser.QLParser.IndexValueExprContext`
/// - 对应 Java: `com.alibaba.qlexpress4.aparser.QLParser.ImportDeclarationContext`
/// - 对应 Java: `com.alibaba.qlexpress4.aparser.QLParser.SwitchCaseGroupContext`
#[derive(Clone, Debug)]
pub enum Node {
    /// `Program` 语法规则节点。
    Program(ProgramContext),
    /// `BlockStatements` 语法规则节点。
    BlockStatements(BlockStatementsContext),
    /// `LocalVariableDeclarationStatement` 语法规则节点。
    LocalVariableDeclarationStatement(LocalVariableDeclarationStatementContext),
    /// `ThrowStatement` 语法规则节点。
    ThrowStatement(ThrowStatementContext),
    /// `WhileStatement` 语法规则节点。
    WhileStatement(WhileStatementContext),
    /// `TraditionalForStatement` 语法规则节点。
    TraditionalForStatement(TraditionalForStatementContext),
    /// `ForEachStatement` 语法规则节点。
    ForEachStatement(ForEachStatementContext),
    /// `FunctionStatement` 语法规则节点。
    FunctionStatement(FunctionStatementContext),
    /// `MacroStatement` 语法规则节点。
    MacroStatement(MacroStatementContext),
    /// `BreakContinueStatement` 语法规则节点。
    BreakContinueStatement(BreakContinueStatementContext),
    /// `ReturnStatement` 语法规则节点。
    ReturnStatement(ReturnStatementContext),
    /// `EmptyStatement` 语法规则节点。
    EmptyStatement(EmptyStatementContext),
    /// `ExpressionStatement` 语法规则节点。
    ExpressionStatement(ExpressionStatementContext),
    /// `NonExpressionStatement` 语法规则节点。
    NonExpressionStatement(NonExpressionStatementContext),
    /// `LocalVariableDeclaration` 语法规则节点。
    LocalVariableDeclaration(LocalVariableDeclarationContext),
    /// `ForInit` 语法规则节点。
    ForInit(ForInitContext),
    /// `VariableDeclaratorList` 语法规则节点。
    VariableDeclaratorList(VariableDeclaratorListContext),
    /// `VariableDeclarator` 语法规则节点。
    VariableDeclarator(VariableDeclaratorContext),
    /// `VariableDeclaratorId` 语法规则节点。
    VariableDeclaratorId(VariableDeclaratorIdContext),
    /// `VariableInitializer` 语法规则节点。
    VariableInitializer(VariableInitializerContext),
    /// `ArrayInitializer` 语法规则节点。
    ArrayInitializer(ArrayInitializerContext),
    /// `VariableInitializerList` 语法规则节点。
    VariableInitializerList(VariableInitializerListContext),
    /// `DeclType` 语法规则节点。
    DeclType(DeclTypeContext),
    /// `DeclTypeNoArr` 语法规则节点。
    DeclTypeNoArr(DeclTypeNoArrContext),
    /// `PrimitiveType` 语法规则节点。
    PrimitiveType(PrimitiveTypeContext),
    /// `ClsType` 语法规则节点。
    ClsType(ClsTypeContext),
    /// `Dims` 语法规则节点。
    Dims(DimsContext),
    /// `DimExprs` 语法规则节点。
    DimExprs(DimExprsContext),
    /// `Expression` 语法规则节点。
    Expression(ExpressionContext),
    /// `LeftHandSide` 语法规则节点。
    LeftHandSide(LeftHandSideContext),
    /// `AssignOperator` 语法规则节点。
    AssignOperator(AssignOperatorContext),
    /// `TernaryExpr` 语法规则节点。
    TernaryExpr(TernaryExprContext),
    /// `BaseExpr` 语法规则节点。
    BaseExpr(BaseExprContext),
    /// `LeftAsso` 语法规则节点。
    LeftAsso(LeftAssoContext),
    /// `Binaryop` 语法规则节点。
    Binaryop(BinaryopContext),
    /// `Primary` 语法规则节点。
    Primary(PrimaryContext),
    /// `PrefixExpress` 语法规则节点。
    PrefixExpress(PrefixExpressContext),
    /// `SuffixExpress` 语法规则节点。
    SuffixExpress(SuffixExpressContext),
    /// `ConstExpr` 语法规则节点。
    ConstExpr(ConstExprContext),
    /// `CastExpr` 语法规则节点。
    CastExpr(CastExprContext),
    /// `GroupExpr` 语法规则节点。
    GroupExpr(GroupExprContext),
    /// `NewObjExpr` 语法规则节点。
    NewObjExpr(NewObjExprContext),
    /// `NewEmptyArrExpr` 语法规则节点。
    NewEmptyArrExpr(NewEmptyArrExprContext),
    /// `NewInitArrExpr` 语法规则节点。
    NewInitArrExpr(NewInitArrExprContext),
    /// `VarIdExpr` 语法规则节点。
    VarIdExpr(VarIdExprContext),
    /// `TypeExpr` 语法规则节点。
    TypeExpr(TypeExprContext),
    /// `ListExpr` 语法规则节点。
    ListExpr(ListExprContext),
    /// `ListItems` 语法规则节点。
    ListItems(ListItemsContext),
    /// `MapExpr` 语法规则节点。
    MapExpr(MapExprContext),
    /// `BlockExpr` 语法规则节点。
    BlockExpr(BlockExprContext),
    /// `ContextSelectExpr` 语法规则节点。
    ContextSelectExpr(ContextSelectExprContext),
    /// `QlIf` 语法规则节点。
    QlIf(QlIfContext),
    /// `ThenBody` 语法规则节点。
    ThenBody(ThenBodyContext),
    /// `ElseBody` 语法规则节点。
    ElseBody(ElseBodyContext),
    /// `SwitchExpr` 语法规则节点。
    SwitchExpr(SwitchExprContext),
    /// `SwitchCaseGroups` 语法规则节点。
    SwitchCaseGroups(SwitchCaseGroupsContext),
    /// `SwitchStatementGroup` 语法规则节点。
    SwitchStatementGroup(SwitchStatementGroupContext),
    /// `SwitchExprGroup` 语法规则节点。
    SwitchExprGroup(SwitchExprGroupContext),
    /// `SwitchLabels` 语法规则节点。
    SwitchLabels(SwitchLabelsContext),
    /// `SwitchLabel` 语法规则节点。
    SwitchLabel(SwitchLabelContext),
    /// `SwitchExpressionLabel` 语法规则节点。
    SwitchExpressionLabel(SwitchExpressionLabelContext),
    /// `ExpressionList` 语法规则节点。
    ExpressionList(ExpressionListContext),
    /// `TryCatchExpr` 语法规则节点。
    TryCatchExpr(TryCatchExprContext),
    /// `TryCatches` 语法规则节点。
    TryCatches(TryCatchesContext),
    /// `TryCatch` 语法规则节点。
    TryCatch(TryCatchContext),
    /// `CatchParams` 语法规则节点。
    CatchParams(CatchParamsContext),
    /// `TryFinally` 语法规则节点。
    TryFinally(TryFinallyContext),
    /// `MapEntries` 语法规则节点。
    MapEntries(MapEntriesContext),
    /// `MapEntry` 语法规则节点。
    MapEntry(MapEntryContext),
    /// `ClsValue` 语法规则节点。
    ClsValue(ClsValueContext),
    /// `EValue` 语法规则节点。
    EValue(EValueContext),
    /// `IdKey` 语法规则节点。
    IdKey(IdKeyContext),
    /// `StringKey` 语法规则节点。
    StringKey(StringKeyContext),
    /// `QuoteStringKey` 语法规则节点。
    QuoteStringKey(QuoteStringKeyContext),
    /// `MethodInvoke` 语法规则节点。
    MethodInvoke(MethodInvokeContext),
    /// `FieldAccess` 语法规则节点。
    FieldAccess(FieldAccessContext),
    /// `MethodAccess` 语法规则节点。
    MethodAccess(MethodAccessContext),
    /// `IndexExpr` 语法规则节点。
    IndexExpr(IndexExprContext),
    /// `CustomPath` 语法规则节点。
    CustomPath(CustomPathContext),
    /// `FieldId` 语法规则节点。
    FieldId(FieldIdContext),
    /// `SingleIndex` 语法规则节点。
    SingleIndex(SingleIndexContext),
    /// `SliceIndex` 语法规则节点。
    SliceIndex(SliceIndexContext),
    /// `ArgumentList` 语法规则节点。
    ArgumentList(ArgumentListContext),
    /// `Literal` 语法规则节点。
    Literal(LiteralContext),
    /// `DoubleQuoteStringLiteral` 语法规则节点。
    DoubleQuoteStringLiteral(DoubleQuoteStringLiteralContext),
    /// `StringExpression` 语法规则节点。
    StringExpression(StringExpressionContext),
    /// `BoolenLiteral` 语法规则节点。
    BoolenLiteral(BoolenLiteralContext),
    /// `LambdaExpr` 语法规则节点。
    LambdaExpr(LambdaExprContext),
    /// `LambdaParameters` 语法规则节点。
    LambdaParameters(LambdaParametersContext),
    /// `FormalOrInferredParameterList` 语法规则节点。
    FormalOrInferredParameterList(FormalOrInferredParameterListContext),
    /// `FormalOrInferredParameter` 语法规则节点。
    FormalOrInferredParameter(FormalOrInferredParameterContext),
    /// `ImportCls` 语法规则节点。
    ImportCls(ImportClsContext),
    /// `ImportPack` 语法规则节点。
    ImportPack(ImportPackContext),
    /// `OpId` 语法规则节点。
    OpId(OpIdContext),
    /// `VarId` 语法规则节点。
    VarId(VarIdContext),
}
