//! Syntax tree node体系, mirroring the `*Context` classes of Java
//! `QLParser` plus `ParseTree`/`RuleContext`/`TerminalNode` and the
//! `QLParserBaseVisitor` traversal semantics.
//!
//! Design notes (Rust adaptation):
//! - Java models the parse tree with one class per grammar rule and runtime
//!   `accept(visitor)` dispatch. Rust uses a single [`Node`] enum with one
//!   variant per Java `*Context` class; [`Node::accept`] dispatches to the
//!   same-named [`Visitor`] method.
//! - Punctuation tokens that Java keeps only in the untyped `children` list
//!   (parentheses, commas, semicolons of expression lists) are not stored;
//!   every *semantically meaningful* token Java keeps in a typed field
//!   (`whileToken`, `newToken`, dots of path parts, ...) is stored as a
//!   [`TerminalNode`]. Consequently [`Node::text`] reproduces Java
//!   `getText()` for all token-level nodes but may omit punctuation in
//!   composite nodes (affects only the `printTree` debug output).
//! - Source positions come from [`Node::start_token`]/[`Node::stop_token`],
//!   computed from the first/last child exactly like Java `RuleContext`
//!   bounds computation.

use super::token::Token;

/// Java `TerminalNode`: a leaf of the syntax tree wrapping a [`Token`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TerminalNode {
    symbol: Token,
}

impl TerminalNode {
    pub fn new(symbol: Token) -> Self {
        TerminalNode { symbol }
    }

    /// Java `getSymbol`.
    pub fn symbol(&self) -> &Token {
        &self.symbol
    }

    /// Java `getText` (the token text).
    pub fn text(&self) -> &str {
        self.symbol.text()
    }
}

/// A borrowed child of a syntax node: either a rule node or a terminal,
/// mirroring Java `ParseTree` children.
#[derive(Clone, Copy, Debug)]
pub enum ChildRef<'a> {
    Node(&'a Node),
    Term(&'a TerminalNode),
}

impl<'a> ChildRef<'a> {
    /// Java `ParseTree.getText`.
    pub fn text(&self) -> String {
        match self {
            ChildRef::Node(n) => n.text(),
            ChildRef::Term(t) => t.text().to_string(),
        }
    }

    /// First token covered by this child (Java bounds computation).
    pub fn start_token(&self) -> Option<&'a Token> {
        match self {
            ChildRef::Node(n) => n.start_token(),
            ChildRef::Term(t) => Some(t.symbol()),
        }
    }

    /// Last token covered by this child.
    pub fn stop_token(&self) -> Option<&'a Token> {
        match self {
            ChildRef::Node(n) => n.stop_token(),
            ChildRef::Term(t) => Some(t.symbol()),
        }
    }
}

/// Anything that can enumerate its children in source order.
pub trait HasChildren {
    /// Children in the exact order the Java parser `addChild`ed them.
    fn children(&self) -> Vec<ChildRef<'_>>;
}

// ---------------------------------------------------------------------------
// Helper constructors used by the `children()` implementations.
// ---------------------------------------------------------------------------

fn n(node: &Node) -> ChildRef<'_> {
    ChildRef::Node(node)
}

fn t(term: &TerminalNode) -> ChildRef<'_> {
    ChildRef::Term(term)
}

fn push_opt<'a>(out: &mut Vec<ChildRef<'a>>, opt: &'a Option<Box<Node>>) {
    if let Some(node) = opt {
        out.push(n(node));
    }
}

fn push_opt_term<'a>(out: &mut Vec<ChildRef<'a>>, opt: &'a Option<TerminalNode>) {
    if let Some(term) = opt {
        out.push(t(term));
    }
}

fn push_all<'a>(out: &mut Vec<ChildRef<'a>>, list: &'a [Node]) {
    for node in list {
        out.push(n(node));
    }
}

// ---------------------------------------------------------------------------
// Context structs (one per Java QLParser inner class).
// ---------------------------------------------------------------------------

/// Java `ProgramContext`.
#[derive(Clone, Debug)]
pub struct ProgramContext {
    /// Import declarations (`ImportCls`/`ImportPack` nodes).
    pub imports: Vec<Node>,
    /// Top-level statements; `None` for an import-only or empty script.
    pub block_statements: Option<Box<Node>>,
}

/// Java `BlockStatementsContext`.
#[derive(Clone, Debug)]
pub struct BlockStatementsContext {
    /// `BlockStatement` nodes in source order.
    pub statements: Vec<Node>,
}

/// Java `LocalVariableDeclarationStatementContext`.
#[derive(Clone, Debug)]
pub struct LocalVariableDeclarationStatementContext {
    pub local_variable_declaration: Box<Node>,
    pub semi: TerminalNode,
}

/// Java `ThrowStatementContext`.
#[derive(Clone, Debug)]
pub struct ThrowStatementContext {
    pub throw_token: TerminalNode,
    pub expression: Box<Node>,
}

/// Java `WhileStatementContext`.
#[derive(Clone, Debug)]
pub struct WhileStatementContext {
    pub while_token: TerminalNode,
    pub expression: Box<Node>,
    /// `None` for an empty `{}` body (Java returns null from
    /// `parseBracedBlock`).
    pub block_statements: Option<Box<Node>>,
}

/// Java `TraditionalForStatementContext`.
#[derive(Clone, Debug)]
pub struct TraditionalForStatementContext {
    pub for_token: TerminalNode,
    pub for_init: Box<Node>,
    pub for_condition: Option<Box<Node>>,
    pub for_update: Option<Box<Node>>,
    pub block_statements: Option<Box<Node>>,
}

/// Java `ForInitContext`. Exactly one of the optionals is `Some`; when both
/// are `None` the init was just `;`.
#[derive(Clone, Debug)]
pub struct ForInitContext {
    pub local_variable_declaration: Option<Box<Node>>,
    pub expression: Option<Box<Node>>,
    pub semi: TerminalNode,
}

/// Java `ForEachStatementContext`.
#[derive(Clone, Debug)]
pub struct ForEachStatementContext {
    pub for_token: TerminalNode,
    /// Declared element type; `None` for `for (x : xs)` (inferred).
    pub decl_type: Option<Box<Node>>,
    pub var_id: Box<Node>,
    pub expression: Box<Node>,
    pub block_statements: Option<Box<Node>>,
}

/// Java `FunctionStatementContext`.
#[derive(Clone, Debug)]
pub struct FunctionStatementContext {
    pub function_token: TerminalNode,
    pub var_id: Box<Node>,
    pub params: Option<Box<Node>>,
    pub block_statements: Option<Box<Node>>,
}

/// Java `MacroStatementContext`.
#[derive(Clone, Debug)]
pub struct MacroStatementContext {
    pub macro_token: TerminalNode,
    pub var_id: Box<Node>,
    pub block_statements: Option<Box<Node>>,
}

/// Java `BreakContinueStatementContext` (`breakToken`/`continueToken` are
/// distinguished by the token type, like Java's null checks).
#[derive(Clone, Debug)]
pub struct BreakContinueStatementContext {
    pub token: TerminalNode,
}

impl BreakContinueStatementContext {
    /// Java `BREAK() != null`.
    pub fn is_break(&self) -> bool {
        self.token.symbol().token_type() == super::token::BREAK as i32
    }
}

/// Java `ReturnStatementContext`.
#[derive(Clone, Debug)]
pub struct ReturnStatementContext {
    pub return_token: TerminalNode,
    pub expression: Option<Box<Node>>,
}

/// Java `EmptyStatementContext` (a lone `;` or newline).
#[derive(Clone, Debug)]
pub struct EmptyStatementContext {
    pub token: TerminalNode,
}

/// Java `ExpressionStatementContext`.
#[derive(Clone, Debug)]
pub struct ExpressionStatementContext {
    pub expression: Box<Node>,
}

/// Java `NonExpressionStatementContext`: wraps a statement usable as an
/// if/else body.
#[derive(Clone, Debug)]
pub struct NonExpressionStatementContext {
    pub statement: Box<Node>,
}

/// Java `LocalVariableDeclarationContext`.
#[derive(Clone, Debug)]
pub struct LocalVariableDeclarationContext {
    pub decl_type: Box<Node>,
    pub variable_declarator_list: Box<Node>,
}

/// Java `VariableDeclaratorListContext`.
#[derive(Clone, Debug)]
pub struct VariableDeclaratorListContext {
    pub variables: Vec<Node>,
}

/// Java `VariableDeclaratorContext`.
#[derive(Clone, Debug)]
pub struct VariableDeclaratorContext {
    pub id: Box<Node>,
    pub initializer: Option<Box<Node>>,
}

/// Java `VariableDeclaratorIdContext`.
#[derive(Clone, Debug)]
pub struct VariableDeclaratorIdContext {
    pub var_id: Box<Node>,
    pub dims: Option<Box<Node>>,
}

/// Java `VariableInitializerContext`: exactly one variant is `Some`.
#[derive(Clone, Debug)]
pub struct VariableInitializerContext {
    pub expression: Option<Box<Node>>,
    pub array_initializer: Option<Box<Node>>,
}

/// Java `ArrayInitializerContext`.
#[derive(Clone, Debug)]
pub struct ArrayInitializerContext {
    pub lbrace: TerminalNode,
    pub initializers: Option<Box<Node>>,
}

/// Java `VariableInitializerListContext`.
#[derive(Clone, Debug)]
pub struct VariableInitializerListContext {
    pub initializers: Vec<Node>,
}

/// Java `DeclTypeContext`.
#[derive(Clone, Debug)]
pub struct DeclTypeContext {
    pub primitive_type: Option<Box<Node>>,
    pub cls_type: Option<Box<Node>>,
    pub dims: Option<Box<Node>>,
}

/// Java `DeclTypeNoArrContext`.
#[derive(Clone, Debug)]
pub struct DeclTypeNoArrContext {
    pub primitive_type: Option<Box<Node>>,
    pub cls_type: Option<Box<Node>>,
}

/// Java `PrimitiveTypeContext`.
#[derive(Clone, Debug)]
pub struct PrimitiveTypeContext {
    pub token: TerminalNode,
}

/// Java `ClsTypeContext` (type arguments are consumed but not kept, like
/// Java's `parseTypeArguments`).
#[derive(Clone, Debug)]
pub struct ClsTypeContext {
    pub var_ids: Vec<Node>,
}

/// Java `DimsContext` (`int[][]`): one `[`/`]` token pair per dimension.
#[derive(Clone, Debug)]
pub struct DimsContext {
    pub brackets: Vec<TerminalNode>,
}

impl DimsContext {
    /// Number of `[]` dimensions (Java `LBRACK().size()`).
    pub fn dim_count(&self) -> usize {
        self.brackets.len() / 2
    }
}

/// Java `DimExprsContext` (`new int[3][4]`).
#[derive(Clone, Debug)]
pub struct DimExprsContext {
    pub expressions: Vec<Node>,
}

/// Java `ExpressionContext`: assignment or ternary.
#[derive(Clone, Debug)]
pub struct ExpressionContext {
    pub left: Option<Box<Node>>,
    pub assign_operator: Option<Box<Node>>,
    pub expression: Option<Box<Node>>,
    pub ternary: Option<Box<Node>>,
}

impl ExpressionContext {
    /// Java `leftHandSide()`.
    pub fn is_assign(&self) -> bool {
        self.left.is_some()
    }
}

/// Java `LeftHandSideContext`.
#[derive(Clone, Debug)]
pub struct LeftHandSideContext {
    pub var_id: Box<Node>,
    /// `Some` when the head is a function call `f(...)`.
    pub lparen: Option<TerminalNode>,
    pub argument_list: Option<Box<Node>>,
    pub path_parts: Vec<Node>,
}

/// Java `AssignOperatorContext`.
#[derive(Clone, Debug)]
pub struct AssignOperatorContext {
    pub token: TerminalNode,
}

/// Java `TernaryExprContext`.
#[derive(Clone, Debug)]
pub struct TernaryExprContext {
    pub condition: Box<Node>,
    pub question: Option<TerminalNode>,
    pub then_expr: Option<Box<Node>>,
    pub else_expr: Option<Box<Node>>,
}

/// Java `BaseExprContext`: a primary plus left-associative binary chain.
#[derive(Clone, Debug)]
pub struct BaseExprContext {
    pub primary: Box<Node>,
    pub left_assos: Vec<Node>,
}

/// Java `LeftAssoContext`: one `op right` step.
#[derive(Clone, Debug)]
pub struct LeftAssoContext {
    pub binaryop: Box<Node>,
    pub right: Box<Node>,
}

/// Java `BinaryopContext`.
#[derive(Clone, Debug)]
pub struct BinaryopContext {
    pub token: TerminalNode,
}

/// Java `PrimaryContext`.
#[derive(Clone, Debug)]
pub struct PrimaryContext {
    pub prefix: Option<Box<Node>>,
    pub pathable: Option<Box<Node>>,
    pub path_parts: Vec<Node>,
    pub suffix: Option<Box<Node>>,
    pub non_pathable: Option<Box<Node>>,
}

/// Java `PrefixExpressContext`.
#[derive(Clone, Debug)]
pub struct PrefixExpressContext {
    pub op_id: Box<Node>,
}

/// Java `SuffixExpressContext`.
#[derive(Clone, Debug)]
pub struct SuffixExpressContext {
    pub op_id: Box<Node>,
}

/// Java `ConstExprContext`.
#[derive(Clone, Debug)]
pub struct ConstExprContext {
    pub literal: Box<Node>,
}

/// Java `CastExprContext`.
#[derive(Clone, Debug)]
pub struct CastExprContext {
    pub lparen: TerminalNode,
    pub decl_type: Box<Node>,
    pub primary: Box<Node>,
}

/// Java `GroupExprContext` (parenthesised expression).
#[derive(Clone, Debug)]
pub struct GroupExprContext {
    pub lparen: TerminalNode,
    pub expression: Box<Node>,
}

/// Java `NewObjExprContext`.
#[derive(Clone, Debug)]
pub struct NewObjExprContext {
    pub new_token: TerminalNode,
    pub var_ids: Vec<Node>,
    pub argument_list: Option<Box<Node>>,
}

/// Java `NewEmptyArrExprContext` (`new int[3]`).
#[derive(Clone, Debug)]
pub struct NewEmptyArrExprContext {
    pub new_token: TerminalNode,
    pub decl_type_no_arr: Box<Node>,
    pub dim_exprs: Box<Node>,
}

/// Java `NewInitArrExprContext` (`new int[]{1,2}`).
#[derive(Clone, Debug)]
pub struct NewInitArrExprContext {
    pub new_token: TerminalNode,
    pub decl_type_no_arr: Box<Node>,
    pub dims: Box<Node>,
    pub array_initializer: Box<Node>,
}

/// Java `VarIdExprContext` (variable reference or function call head).
#[derive(Clone, Debug)]
pub struct VarIdExprContext {
    pub var_id: Box<Node>,
    pub lparen: Option<TerminalNode>,
    pub argument_list: Option<Box<Node>>,
}

/// Java `TypeExprContext` (a primitive type used as a value, e.g. `int.class`).
#[derive(Clone, Debug)]
pub struct TypeExprContext {
    pub primitive_type: Box<Node>,
}

/// Java `ListItemsContext`.
#[derive(Clone, Debug)]
pub struct ListItemsContext {
    pub expressions: Vec<Node>,
}

/// Java `ListExprContext`.
#[derive(Clone, Debug)]
pub struct ListExprContext {
    pub lbrack: TerminalNode,
    pub list_items: Option<Box<Node>>,
}

/// Java `MapExprContext`.
#[derive(Clone, Debug)]
pub struct MapExprContext {
    pub lbrace: TerminalNode,
    pub map_entries: Box<Node>,
}

/// Java `BlockExprContext` (a block used as an expression).
#[derive(Clone, Debug)]
pub struct BlockExprContext {
    pub lbrace: TerminalNode,
    pub block_statements: Option<Box<Node>>,
}

/// Java `ContextSelectExprContext` (selector expression).
#[derive(Clone, Debug)]
pub struct ContextSelectExprContext {
    pub selector_start: TerminalNode,
    pub selector_variable: TerminalNode,
}

/// Java `QlIfContext`.
#[derive(Clone, Debug)]
pub struct QlIfContext {
    pub if_token: TerminalNode,
    pub then_keyword: Option<TerminalNode>,
    pub condition: Box<Node>,
    pub then_body: Box<Node>,
    pub else_body: Option<Box<Node>>,
}

/// Java `ThenBodyContext`: exactly one of the optionals is `Some`.
#[derive(Clone, Debug)]
pub struct ThenBodyContext {
    pub lbrace: Option<TerminalNode>,
    pub block_statements: Option<Box<Node>>,
    pub non_expression_statement: Option<Box<Node>>,
    pub expression: Option<Box<Node>>,
}

/// Java `ElseBodyContext`: exactly one of the optionals is `Some`.
#[derive(Clone, Debug)]
pub struct ElseBodyContext {
    pub lbrace: Option<TerminalNode>,
    pub block_statements: Option<Box<Node>>,
    pub ql_if: Option<Box<Node>>,
    pub non_expression_statement: Option<Box<Node>>,
    pub expression: Option<Box<Node>>,
}

/// Java `SwitchExprContext`.
#[derive(Clone, Debug)]
pub struct SwitchExprContext {
    pub switch_token: TerminalNode,
    pub expression: Box<Node>,
    pub groups: Option<Box<Node>>,
}

/// Java `SwitchCaseGroupsContext`.
#[derive(Clone, Debug)]
pub struct SwitchCaseGroupsContext {
    pub groups: Vec<Node>,
}

/// Java `SwitchStatementGroupContext`.
#[derive(Clone, Debug)]
pub struct SwitchStatementGroupContext {
    pub labels: Box<Node>,
    pub block_statements: Option<Box<Node>>,
}

/// Java `SwitchExprGroupContext` (`case a -> expr`).
#[derive(Clone, Debug)]
pub struct SwitchExprGroupContext {
    pub label: Box<Node>,
    pub expression: Box<Node>,
}

/// Java `SwitchLabelsContext`.
#[derive(Clone, Debug)]
pub struct SwitchLabelsContext {
    pub labels: Vec<Node>,
}

/// Java `SwitchLabelContext`.
#[derive(Clone, Debug)]
pub struct SwitchLabelContext {
    pub case_token: Option<TerminalNode>,
    pub default_token: Option<TerminalNode>,
    pub expression: Option<Box<Node>>,
}

/// Java `SwitchExpressionLabelContext`.
#[derive(Clone, Debug)]
pub struct SwitchExpressionLabelContext {
    pub case_token: Option<TerminalNode>,
    pub default_token: Option<TerminalNode>,
    pub expression_list: Option<Box<Node>>,
    pub arrow: TerminalNode,
}

/// Java `ExpressionListContext`.
#[derive(Clone, Debug)]
pub struct ExpressionListContext {
    pub expressions: Vec<Node>,
}

/// Java `TryCatchExprContext`.
#[derive(Clone, Debug)]
pub struct TryCatchExprContext {
    pub try_token: TerminalNode,
    pub block_statements: Option<Box<Node>>,
    pub try_catches: Option<Box<Node>>,
    pub try_finally: Option<Box<Node>>,
}

/// Java `TryCatchesContext`.
#[derive(Clone, Debug)]
pub struct TryCatchesContext {
    pub catches: Vec<Node>,
}

/// Java `TryCatchContext` (one catch clause).
#[derive(Clone, Debug)]
pub struct TryCatchContext {
    pub catch_token: TerminalNode,
    pub catch_params: Box<Node>,
    pub block_statements: Option<Box<Node>>,
}

/// Java `CatchParamsContext`.
#[derive(Clone, Debug)]
pub struct CatchParamsContext {
    pub decl_types: Vec<Node>,
    pub var_id: Box<Node>,
}

/// Java `TryFinallyContext`.
#[derive(Clone, Debug)]
pub struct TryFinallyContext {
    pub finally_token: TerminalNode,
    pub block_statements: Option<Box<Node>>,
}

/// Java `MapEntriesContext`. `empty_colon` is `Some` for the empty-map
/// literal `{:}`.
#[derive(Clone, Debug)]
pub struct MapEntriesContext {
    pub empty_colon: Option<TerminalNode>,
    pub entries: Vec<Node>,
}

/// Java `MapEntryContext`.
#[derive(Clone, Debug)]
pub struct MapEntryContext {
    pub map_key: Box<Node>,
    pub map_value: Box<Node>,
}

/// Java `ClsValueContext` (map value of the special `'@class'` key).
#[derive(Clone, Debug)]
pub struct ClsValueContext {
    pub quote: TerminalNode,
}

/// Java `EValueContext` (ordinary map value).
#[derive(Clone, Debug)]
pub struct EValueContext {
    pub expression: Box<Node>,
}

/// Java `IdKeyContext`.
#[derive(Clone, Debug)]
pub struct IdKeyContext {
    pub token: TerminalNode,
}

/// Java `StringKeyContext` (double-quoted map key).
#[derive(Clone, Debug)]
pub struct StringKeyContext {
    pub double_quote_string: Box<Node>,
}

/// Java `QuoteStringKeyContext` (single-quoted map key).
#[derive(Clone, Debug)]
pub struct QuoteStringKeyContext {
    pub token: TerminalNode,
}

/// How a path part is chained, mirroring the Java `Optional*`/`Spread*`
/// subclasses of `MethodInvokeContext`/`FieldAccessContext`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ChainKind {
    /// `.`
    Plain,
    /// `?.` (Java `OPTIONAL_CHAINING`)
    Optional,
    /// `*.` (Java `SPREAD_CHAINING`)
    Spread,
}

/// Java `MethodInvokeContext` (plus the optional/spread subclasses).
#[derive(Clone, Debug)]
pub struct MethodInvokeContext {
    /// The `.` / `?.` / `*.` token (Java stores it as the first child).
    pub dot: TerminalNode,
    pub var_id: Box<Node>,
    pub argument_list: Option<Box<Node>>,
    pub chain: ChainKind,
}

/// Java `FieldAccessContext` (plus the optional/spread subclasses).
#[derive(Clone, Debug)]
pub struct FieldAccessContext {
    /// The `.` / `?.` / `*.` token.
    pub dot: TerminalNode,
    pub field_id: Box<Node>,
    pub chain: ChainKind,
}

/// Java `MethodAccessContext` (`Cls::method`).
#[derive(Clone, Debug)]
pub struct MethodAccessContext {
    pub dcolon: TerminalNode,
    pub var_id: Box<Node>,
}

/// Java `IndexExprContext` (`a[i]` / `a[i:j]`); `None` index for `a[]`.
#[derive(Clone, Debug)]
pub struct IndexExprContext {
    pub lbrack: TerminalNode,
    pub index_value_expr: Option<Box<Node>>,
}

/// Java `CustomPathContext` (custom operator path, e.g. `a %% 'path'`).
#[derive(Clone, Debug)]
pub struct CustomPathContext {
    pub op_id: Box<Node>,
    pub var_id: Option<Box<Node>>,
    pub quote: Option<TerminalNode>,
    pub path_text: String,
}

/// Java `FieldIdContext`.
#[derive(Clone, Debug)]
pub struct FieldIdContext {
    pub token: Option<TerminalNode>,
    pub quote: Option<TerminalNode>,
}

/// Java `SingleIndexContext`.
#[derive(Clone, Debug)]
pub struct SingleIndexContext {
    pub expression: Box<Node>,
}

/// Java `SliceIndexContext` (`a[start:end]`).
#[derive(Clone, Debug)]
pub struct SliceIndexContext {
    pub start: Option<Box<Node>>,
    pub colon: TerminalNode,
    pub end: Option<Box<Node>>,
}

/// Java `ArgumentListContext`.
#[derive(Clone, Debug)]
pub struct ArgumentListContext {
    pub expressions: Vec<Node>,
}

/// Java `LiteralContext`: exactly one of the fields is `Some`.
#[derive(Clone, Debug)]
pub struct LiteralContext {
    /// Number / single-quoted string / `null` token.
    pub token: Option<TerminalNode>,
    pub boolen: Option<Box<Node>>,
    pub double_quote_string: Option<Box<Node>>,
}

/// One piece of a double-quoted string: literal text or an interpolation.
#[derive(Clone, Debug)]
pub enum DyStrPart {
    /// Java `DyStrText` token.
    Text(TerminalNode),
    /// Java `StringExpressionContext`.
    Expr(Box<Node>),
}

/// Java `DoubleQuoteStringLiteralContext`.
#[derive(Clone, Debug)]
pub struct DoubleQuoteStringLiteralContext {
    pub open_quote: TerminalNode,
    pub static_characters: Option<TerminalNode>,
    pub parts: Vec<DyStrPart>,
    pub close_quote: TerminalNode,
}

/// Java `StringExpressionContext` (`${expr}` or `${#var}` inside a string).
#[derive(Clone, Debug)]
pub struct StringExpressionContext {
    pub start: TerminalNode,
    pub selector_variable: Option<TerminalNode>,
    pub expression: Option<Box<Node>>,
}

/// Java `BoolenLiteralContext` (sic, Java spelling).
#[derive(Clone, Debug)]
pub struct BoolenLiteralContext {
    pub token: TerminalNode,
}

/// Java `LambdaExprContext`.
#[derive(Clone, Debug)]
pub struct LambdaExprContext {
    pub lambda_parameters: Box<Node>,
    pub arrow: TerminalNode,
    pub lbrace: Option<TerminalNode>,
    pub block_statements: Option<Box<Node>>,
    pub expression: Option<Box<Node>>,
}

/// Java `LambdaParametersContext`: single id (`x -> ..`) or parameter list.
#[derive(Clone, Debug)]
pub struct LambdaParametersContext {
    pub var_id: Option<Box<Node>>,
    pub params: Option<Box<Node>>,
}

/// Java `FormalOrInferredParameterListContext`.
#[derive(Clone, Debug)]
pub struct FormalOrInferredParameterListContext {
    pub params: Vec<Node>,
}

/// Java `FormalOrInferredParameterContext`.
#[derive(Clone, Debug)]
pub struct FormalOrInferredParameterContext {
    pub decl_type: Option<Box<Node>>,
    pub var_id: Box<Node>,
}

/// Java `ImportClsContext` (`import a.b.C;`).
#[derive(Clone, Debug)]
pub struct ImportClsContext {
    pub import_token: TerminalNode,
    pub var_ids: Vec<Node>,
}

/// Java `ImportPackContext` (`import a.b.*;` / `import a.b.*;`).
#[derive(Clone, Debug)]
pub struct ImportPackContext {
    pub import_token: TerminalNode,
    pub var_ids: Vec<Node>,
}

/// Java `OpIdContext` (prefix/suffix/custom operator token).
#[derive(Clone, Debug)]
pub struct OpIdContext {
    pub token: TerminalNode,
}

/// Java `VarIdContext` (an identifier token).
#[derive(Clone, Debug)]
pub struct VarIdContext {
    pub token: TerminalNode,
}

// ---------------------------------------------------------------------------
// Node enum: one variant per Java QLParser *Context class.
// ---------------------------------------------------------------------------

/// A syntax tree node, mirroring the Java `QLParser.*Context` hierarchy
/// (flattened: Java's abstract intermediates `BlockStatementContext`,
/// `PathPartContext`, `MapKeyContext`, ... are represented directly by the
/// concrete variants).
#[derive(Clone, Debug)]
pub enum Node {
    Program(ProgramContext),
    BlockStatements(BlockStatementsContext),
    LocalVariableDeclarationStatement(LocalVariableDeclarationStatementContext),
    ThrowStatement(ThrowStatementContext),
    WhileStatement(WhileStatementContext),
    TraditionalForStatement(TraditionalForStatementContext),
    ForEachStatement(ForEachStatementContext),
    FunctionStatement(FunctionStatementContext),
    MacroStatement(MacroStatementContext),
    BreakContinueStatement(BreakContinueStatementContext),
    ReturnStatement(ReturnStatementContext),
    EmptyStatement(EmptyStatementContext),
    ExpressionStatement(ExpressionStatementContext),
    NonExpressionStatement(NonExpressionStatementContext),
    LocalVariableDeclaration(LocalVariableDeclarationContext),
    ForInit(ForInitContext),
    VariableDeclaratorList(VariableDeclaratorListContext),
    VariableDeclarator(VariableDeclaratorContext),
    VariableDeclaratorId(VariableDeclaratorIdContext),
    VariableInitializer(VariableInitializerContext),
    ArrayInitializer(ArrayInitializerContext),
    VariableInitializerList(VariableInitializerListContext),
    DeclType(DeclTypeContext),
    DeclTypeNoArr(DeclTypeNoArrContext),
    PrimitiveType(PrimitiveTypeContext),
    ClsType(ClsTypeContext),
    Dims(DimsContext),
    DimExprs(DimExprsContext),
    Expression(ExpressionContext),
    LeftHandSide(LeftHandSideContext),
    AssignOperator(AssignOperatorContext),
    TernaryExpr(TernaryExprContext),
    BaseExpr(BaseExprContext),
    LeftAsso(LeftAssoContext),
    Binaryop(BinaryopContext),
    Primary(PrimaryContext),
    PrefixExpress(PrefixExpressContext),
    SuffixExpress(SuffixExpressContext),
    ConstExpr(ConstExprContext),
    CastExpr(CastExprContext),
    GroupExpr(GroupExprContext),
    NewObjExpr(NewObjExprContext),
    NewEmptyArrExpr(NewEmptyArrExprContext),
    NewInitArrExpr(NewInitArrExprContext),
    VarIdExpr(VarIdExprContext),
    TypeExpr(TypeExprContext),
    ListExpr(ListExprContext),
    ListItems(ListItemsContext),
    MapExpr(MapExprContext),
    BlockExpr(BlockExprContext),
    ContextSelectExpr(ContextSelectExprContext),
    QlIf(QlIfContext),
    ThenBody(ThenBodyContext),
    ElseBody(ElseBodyContext),
    SwitchExpr(SwitchExprContext),
    SwitchCaseGroups(SwitchCaseGroupsContext),
    SwitchStatementGroup(SwitchStatementGroupContext),
    SwitchExprGroup(SwitchExprGroupContext),
    SwitchLabels(SwitchLabelsContext),
    SwitchLabel(SwitchLabelContext),
    SwitchExpressionLabel(SwitchExpressionLabelContext),
    ExpressionList(ExpressionListContext),
    TryCatchExpr(TryCatchExprContext),
    TryCatches(TryCatchesContext),
    TryCatch(TryCatchContext),
    CatchParams(CatchParamsContext),
    TryFinally(TryFinallyContext),
    MapEntries(MapEntriesContext),
    MapEntry(MapEntryContext),
    ClsValue(ClsValueContext),
    EValue(EValueContext),
    IdKey(IdKeyContext),
    StringKey(StringKeyContext),
    QuoteStringKey(QuoteStringKeyContext),
    MethodInvoke(MethodInvokeContext),
    FieldAccess(FieldAccessContext),
    MethodAccess(MethodAccessContext),
    IndexExpr(IndexExprContext),
    CustomPath(CustomPathContext),
    FieldId(FieldIdContext),
    SingleIndex(SingleIndexContext),
    SliceIndex(SliceIndexContext),
    ArgumentList(ArgumentListContext),
    Literal(LiteralContext),
    DoubleQuoteStringLiteral(DoubleQuoteStringLiteralContext),
    StringExpression(StringExpressionContext),
    BoolenLiteral(BoolenLiteralContext),
    LambdaExpr(LambdaExprContext),
    LambdaParameters(LambdaParametersContext),
    FormalOrInferredParameterList(FormalOrInferredParameterListContext),
    FormalOrInferredParameter(FormalOrInferredParameterContext),
    ImportCls(ImportClsContext),
    ImportPack(ImportPackContext),
    OpId(OpIdContext),
    VarId(VarIdContext),
}

impl Node {
    /// Children in source order (Java `RuleContext.children`).
    pub fn children(&self) -> Vec<ChildRef<'_>> {
        <Self as HasChildren>::children(self)
    }

    /// Java `ParseTree.getText`: concatenation of the stored child texts.
    ///
    /// Punctuation Java keeps only in the untyped child list (parentheses,
    /// commas) is omitted; token-level texts match Java exactly.
    pub fn text(&self) -> String {
        let mut out = String::new();
        for child in self.children() {
            out.push_str(&child.text());
        }
        out
    }

    /// First token covered by this node (Java `RuleContext.getStart`).
    pub fn start_token(&self) -> Option<&Token> {
        self.children().into_iter().find_map(|c| c.start_token())
    }

    /// Last token covered by this node (Java `RuleContext.getStop`).
    pub fn stop_token(&self) -> Option<&Token> {
        self.children().into_iter().rev().find_map(|c| c.stop_token())
    }

    /// 1-based line of the first token, if any.
    pub fn line(&self) -> Option<i32> {
        self.start_token().map(Token::line)
    }

    /// Dispatch to the typed visitor method, mirroring Java
    /// `*Context.accept`.
    pub fn accept<V: Visitor + ?Sized>(&self, visitor: &mut V) -> V::T {
        match self {
            Node::Program(c) => visitor.visit_program(c),
            Node::BlockStatements(c) => visitor.visit_block_statements(c),
            Node::LocalVariableDeclarationStatement(c) => {
                visitor.visit_local_variable_declaration_statement(c)
            }
            Node::ThrowStatement(c) => visitor.visit_throw_statement(c),
            Node::WhileStatement(c) => visitor.visit_while_statement(c),
            Node::TraditionalForStatement(c) => visitor.visit_traditional_for_statement(c),
            Node::ForEachStatement(c) => visitor.visit_for_each_statement(c),
            Node::FunctionStatement(c) => visitor.visit_function_statement(c),
            Node::MacroStatement(c) => visitor.visit_macro_statement(c),
            Node::BreakContinueStatement(c) => visitor.visit_break_continue_statement(c),
            Node::ReturnStatement(c) => visitor.visit_return_statement(c),
            Node::EmptyStatement(c) => visitor.visit_empty_statement(c),
            Node::ExpressionStatement(c) => visitor.visit_expression_statement(c),
            Node::NonExpressionStatement(c) => visitor.visit_non_expression_statement(c),
            Node::LocalVariableDeclaration(c) => visitor.visit_local_variable_declaration(c),
            Node::ForInit(c) => visitor.visit_for_init(c),
            Node::VariableDeclaratorList(c) => visitor.visit_variable_declarator_list(c),
            Node::VariableDeclarator(c) => visitor.visit_variable_declarator(c),
            Node::VariableDeclaratorId(c) => visitor.visit_variable_declarator_id(c),
            Node::VariableInitializer(c) => visitor.visit_variable_initializer(c),
            Node::ArrayInitializer(c) => visitor.visit_array_initializer(c),
            Node::VariableInitializerList(c) => visitor.visit_variable_initializer_list(c),
            Node::DeclType(c) => visitor.visit_decl_type(c),
            Node::DeclTypeNoArr(c) => visitor.visit_decl_type_no_arr(c),
            Node::PrimitiveType(c) => visitor.visit_primitive_type(c),
            Node::ClsType(c) => visitor.visit_cls_type(c),
            Node::Dims(c) => visitor.visit_dims(c),
            Node::DimExprs(c) => visitor.visit_dim_exprs(c),
            Node::Expression(c) => visitor.visit_expression(c),
            Node::LeftHandSide(c) => visitor.visit_left_hand_side(c),
            Node::AssignOperator(c) => visitor.visit_assign_operator(c),
            Node::TernaryExpr(c) => visitor.visit_ternary_expr(c),
            Node::BaseExpr(c) => visitor.visit_base_expr(c),
            Node::LeftAsso(c) => visitor.visit_left_asso(c),
            Node::Binaryop(c) => visitor.visit_binaryop(c),
            Node::Primary(c) => visitor.visit_primary(c),
            Node::PrefixExpress(c) => visitor.visit_prefix_express(c),
            Node::SuffixExpress(c) => visitor.visit_suffix_express(c),
            Node::ConstExpr(c) => visitor.visit_const_expr(c),
            Node::CastExpr(c) => visitor.visit_cast_expr(c),
            Node::GroupExpr(c) => visitor.visit_group_expr(c),
            Node::NewObjExpr(c) => visitor.visit_new_obj_expr(c),
            Node::NewEmptyArrExpr(c) => visitor.visit_new_empty_arr_expr(c),
            Node::NewInitArrExpr(c) => visitor.visit_new_init_arr_expr(c),
            Node::VarIdExpr(c) => visitor.visit_var_id_expr(c),
            Node::TypeExpr(c) => visitor.visit_type_expr(c),
            Node::ListExpr(c) => visitor.visit_list_expr(c),
            Node::ListItems(c) => visitor.visit_list_items(c),
            Node::MapExpr(c) => visitor.visit_map_expr(c),
            Node::BlockExpr(c) => visitor.visit_block_expr(c),
            Node::ContextSelectExpr(c) => visitor.visit_context_select_expr(c),
            Node::QlIf(c) => visitor.visit_ql_if(c),
            Node::ThenBody(c) => visitor.visit_then_body(c),
            Node::ElseBody(c) => visitor.visit_else_body(c),
            Node::SwitchExpr(c) => visitor.visit_switch_expr(c),
            Node::SwitchCaseGroups(c) => visitor.visit_switch_case_groups(c),
            Node::SwitchStatementGroup(c) => visitor.visit_switch_statement_group(c),
            Node::SwitchExprGroup(c) => visitor.visit_switch_expr_group(c),
            Node::SwitchLabels(c) => visitor.visit_switch_labels(c),
            Node::SwitchLabel(c) => visitor.visit_switch_label(c),
            Node::SwitchExpressionLabel(c) => visitor.visit_switch_expression_label(c),
            Node::ExpressionList(c) => visitor.visit_expression_list(c),
            Node::TryCatchExpr(c) => visitor.visit_try_catch_expr(c),
            Node::TryCatches(c) => visitor.visit_try_catches(c),
            Node::TryCatch(c) => visitor.visit_try_catch(c),
            Node::CatchParams(c) => visitor.visit_catch_params(c),
            Node::TryFinally(c) => visitor.visit_try_finally(c),
            Node::MapEntries(c) => visitor.visit_map_entries(c),
            Node::MapEntry(c) => visitor.visit_map_entry(c),
            Node::ClsValue(c) => visitor.visit_cls_value(c),
            Node::EValue(c) => visitor.visit_e_value(c),
            Node::IdKey(c) => visitor.visit_id_key(c),
            Node::StringKey(c) => visitor.visit_string_key(c),
            Node::QuoteStringKey(c) => visitor.visit_quote_string_key(c),
            Node::MethodInvoke(c) => visitor.visit_method_invoke(c),
            Node::FieldAccess(c) => visitor.visit_field_access(c),
            Node::MethodAccess(c) => visitor.visit_method_access(c),
            Node::IndexExpr(c) => visitor.visit_index_expr(c),
            Node::CustomPath(c) => visitor.visit_custom_path(c),
            Node::FieldId(c) => visitor.visit_field_id(c),
            Node::SingleIndex(c) => visitor.visit_single_index(c),
            Node::SliceIndex(c) => visitor.visit_slice_index(c),
            Node::ArgumentList(c) => visitor.visit_argument_list(c),
            Node::Literal(c) => visitor.visit_literal(c),
            Node::DoubleQuoteStringLiteral(c) => visitor.visit_double_quote_string_literal(c),
            Node::StringExpression(c) => visitor.visit_string_expression(c),
            Node::BoolenLiteral(c) => visitor.visit_boolen_literal(c),
            Node::LambdaExpr(c) => visitor.visit_lambda_expr(c),
            Node::LambdaParameters(c) => visitor.visit_lambda_parameters(c),
            Node::FormalOrInferredParameterList(c) => {
                visitor.visit_formal_or_inferred_parameter_list(c)
            }
            Node::FormalOrInferredParameter(c) => visitor.visit_formal_or_inferred_parameter(c),
            Node::ImportCls(c) => visitor.visit_import_cls(c),
            Node::ImportPack(c) => visitor.visit_import_pack(c),
            Node::OpId(c) => visitor.visit_op_id(c),
            Node::VarId(c) => visitor.visit_var_id(c),
        }
    }
}

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
        let mut out = vec![t(&self.while_token), n(&self.expression)];
        push_opt(&mut out, &self.block_statements);
        out
    }
}

impl HasChildren for TraditionalForStatementContext {
    fn children(&self) -> Vec<ChildRef<'_>> {
        let mut out = vec![t(&self.for_token), n(&self.for_init)];
        push_opt(&mut out, &self.for_condition);
        push_opt(&mut out, &self.for_update);
        push_opt(&mut out, &self.block_statements);
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
        let mut out = vec![t(&self.for_token)];
        push_opt(&mut out, &self.decl_type);
        out.push(n(&self.var_id));
        out.push(n(&self.expression));
        push_opt(&mut out, &self.block_statements);
        out
    }
}

impl HasChildren for FunctionStatementContext {
    fn children(&self) -> Vec<ChildRef<'_>> {
        let mut out = vec![t(&self.function_token), n(&self.var_id)];
        push_opt(&mut out, &self.params);
        push_opt(&mut out, &self.block_statements);
        out
    }
}

impl HasChildren for MacroStatementContext {
    fn children(&self) -> Vec<ChildRef<'_>> {
        let mut out = vec![t(&self.macro_token), n(&self.var_id)];
        push_opt(&mut out, &self.block_statements);
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
        self.variables.iter().map(n).collect()
    }
}

impl HasChildren for VariableDeclaratorContext {
    fn children(&self) -> Vec<ChildRef<'_>> {
        let mut out = vec![n(&self.id)];
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
        out
    }
}

impl HasChildren for VariableInitializerListContext {
    fn children(&self) -> Vec<ChildRef<'_>> {
        self.initializers.iter().map(n).collect()
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
        self.expressions.iter().map(n).collect()
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
        vec![t(&self.lparen), n(&self.decl_type), n(&self.primary)]
    }
}

impl HasChildren for GroupExprContext {
    fn children(&self) -> Vec<ChildRef<'_>> {
        vec![t(&self.lparen), n(&self.expression)]
    }
}

impl HasChildren for NewObjExprContext {
    fn children(&self) -> Vec<ChildRef<'_>> {
        let mut out = vec![t(&self.new_token)];
        push_all(&mut out, &self.var_ids);
        push_opt(&mut out, &self.argument_list);
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
        out
    }
}

impl HasChildren for TypeExprContext {
    fn children(&self) -> Vec<ChildRef<'_>> {
        vec![n(&self.primitive_type)]
    }
}

impl HasChildren for ListExprContext {
    fn children(&self) -> Vec<ChildRef<'_>> {
        let mut out = vec![t(&self.lbrack)];
        push_opt(&mut out, &self.list_items);
        out
    }
}

impl HasChildren for ListItemsContext {
    fn children(&self) -> Vec<ChildRef<'_>> {
        self.expressions.iter().map(n).collect()
    }
}

impl HasChildren for MapExprContext {
    fn children(&self) -> Vec<ChildRef<'_>> {
        vec![t(&self.lbrace), n(&self.map_entries)]
    }
}

impl HasChildren for BlockExprContext {
    fn children(&self) -> Vec<ChildRef<'_>> {
        let mut out = vec![t(&self.lbrace)];
        push_opt(&mut out, &self.block_statements);
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
        let mut out = vec![t(&self.if_token), n(&self.condition)];
        push_opt_term(&mut out, &self.then_keyword);
        out.push(n(&self.then_body));
        push_opt(&mut out, &self.else_body);
        out
    }
}

impl HasChildren for ThenBodyContext {
    fn children(&self) -> Vec<ChildRef<'_>> {
        let mut out = Vec::new();
        push_opt_term(&mut out, &self.lbrace);
        push_opt(&mut out, &self.block_statements);
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
        push_opt(&mut out, &self.ql_if);
        push_opt(&mut out, &self.non_expression_statement);
        push_opt(&mut out, &self.expression);
        out
    }
}

impl HasChildren for SwitchExprContext {
    fn children(&self) -> Vec<ChildRef<'_>> {
        let mut out = vec![t(&self.switch_token), n(&self.expression)];
        push_opt(&mut out, &self.groups);
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
        self.expressions.iter().map(n).collect()
    }
}

impl HasChildren for TryCatchExprContext {
    fn children(&self) -> Vec<ChildRef<'_>> {
        let mut out = vec![t(&self.try_token)];
        push_opt(&mut out, &self.block_statements);
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
        let mut out = vec![t(&self.catch_token), n(&self.catch_params)];
        push_opt(&mut out, &self.block_statements);
        out
    }
}

impl HasChildren for CatchParamsContext {
    fn children(&self) -> Vec<ChildRef<'_>> {
        let mut out = Vec::new();
        push_all(&mut out, &self.decl_types);
        out.push(n(&self.var_id));
        out
    }
}

impl HasChildren for TryFinallyContext {
    fn children(&self) -> Vec<ChildRef<'_>> {
        let mut out = vec![t(&self.finally_token)];
        push_opt(&mut out, &self.block_statements);
        out
    }
}

impl HasChildren for MapEntriesContext {
    fn children(&self) -> Vec<ChildRef<'_>> {
        let mut out = Vec::new();
        push_opt_term(&mut out, &self.empty_colon);
        push_all(&mut out, &self.entries);
        out
    }
}

impl HasChildren for MapEntryContext {
    fn children(&self) -> Vec<ChildRef<'_>> {
        vec![n(&self.map_key), n(&self.map_value)]
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
        let mut out = vec![t(&self.dot), n(&self.var_id)];
        push_opt(&mut out, &self.argument_list);
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
        self.expressions.iter().map(n).collect()
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
        push_opt(&mut out, &self.expression);
        out
    }
}

impl HasChildren for LambdaParametersContext {
    fn children(&self) -> Vec<ChildRef<'_>> {
        let mut out = Vec::new();
        push_opt(&mut out, &self.var_id);
        push_opt(&mut out, &self.params);
        out
    }
}

impl HasChildren for FormalOrInferredParameterListContext {
    fn children(&self) -> Vec<ChildRef<'_>> {
        self.params.iter().map(n).collect()
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
        out
    }
}

impl HasChildren for ImportPackContext {
    fn children(&self) -> Vec<ChildRef<'_>> {
        let mut out = vec![t(&self.import_token)];
        push_all(&mut out, &self.var_ids);
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
            Program, BlockStatements, LocalVariableDeclarationStatement, ThrowStatement,
            WhileStatement, TraditionalForStatement, ForEachStatement, FunctionStatement,
            MacroStatement, BreakContinueStatement, ReturnStatement, EmptyStatement,
            ExpressionStatement, NonExpressionStatement, LocalVariableDeclaration, ForInit,
            VariableDeclaratorList, VariableDeclarator, VariableDeclaratorId, VariableInitializer,
            ArrayInitializer, VariableInitializerList, DeclType, DeclTypeNoArr, PrimitiveType,
            ClsType, Dims, DimExprs, Expression, LeftHandSide, AssignOperator, TernaryExpr,
            BaseExpr, LeftAsso, Binaryop, Primary, PrefixExpress, SuffixExpress, ConstExpr,
            CastExpr, GroupExpr, NewObjExpr, NewEmptyArrExpr, NewInitArrExpr, VarIdExpr, TypeExpr,
            ListExpr, ListItems, MapExpr, BlockExpr, ContextSelectExpr, QlIf, ThenBody, ElseBody,
            SwitchExpr,
            SwitchCaseGroups, SwitchStatementGroup, SwitchExprGroup, SwitchLabels, SwitchLabel,
            SwitchExpressionLabel, ExpressionList, TryCatchExpr, TryCatches, TryCatch, CatchParams,
            TryFinally, MapEntries, MapEntry, ClsValue, EValue, IdKey, StringKey, QuoteStringKey,
            MethodInvoke, FieldAccess, MethodAccess, IndexExpr, CustomPath, FieldId, SingleIndex,
            SliceIndex, ArgumentList, Literal, DoubleQuoteStringLiteral, StringExpression,
            BoolenLiteral, LambdaExpr, LambdaParameters, FormalOrInferredParameterList,
            FormalOrInferredParameter, ImportCls, ImportPack, OpId, VarId
        )
    }
}

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
    use crate::aparser::token;

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
                        let _ = self.visit_terminal(term);
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
