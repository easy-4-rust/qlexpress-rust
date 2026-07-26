//! 语法树节点体系(语法树工厂产物),对应 Java `com.alibaba.qlexpress4.aparser.SyntaxTreeFactory`。
//! Java 侧节点为 QLParser 内部 *Context 类(每条文法规则一个内部类),此处聚合于
//! syntax_tree_factory:以单个 [`Node`] 枚举 + 每变体一个 *Context 结构体表示,
//! [`Node::accept`] 派发至同名 Visitor 方法,等价于 Java 的运行时 accept(visitor) 双分派。
//! 本文件由 `syntax_tree.rs` 拆分而来(SPEC §5.5 一类一文件),仅移动代码与补充中文注释,行为完全一致。

use super::rule_context::{n, push_all, push_opt, push_opt_term, t, ChildRef, HasChildren};
use super::terminal_node::TerminalNode;

// ---------------------------------------------------------------------------
// Context structs (one per Java QLParser inner class).
// ---------------------------------------------------------------------------

/// 语法树节点 ProgramContext。对应 Java: com.alibaba.qlexpress4.aparser.QLParser 内部类 ProgramContext
/// Java `ProgramContext`.
#[derive(Clone, Debug)]
pub struct ProgramContext {
    /// Import declarations (`ImportCls`/`ImportPack` nodes).
    pub imports: Vec<Node>,
    /// Top-level statements; `None` for an import-only or empty script.
    pub block_statements: Option<Box<Node>>,
}

/// 语法树节点 BlockStatementsContext。对应 Java: com.alibaba.qlexpress4.aparser.QLParser 内部类 BlockStatementsContext
/// Java `BlockStatementsContext`.
#[derive(Clone, Debug)]
pub struct BlockStatementsContext {
    /// `BlockStatement` nodes in source order.
    pub statements: Vec<Node>,
}

/// 语法树节点 LocalVariableDeclarationStatementContext。对应 Java: com.alibaba.qlexpress4.aparser.QLParser 内部类 LocalVariableDeclarationStatementContext
/// Java `LocalVariableDeclarationStatementContext`.
#[derive(Clone, Debug)]
pub struct LocalVariableDeclarationStatementContext {
    pub local_variable_declaration: Box<Node>,
    pub semi: TerminalNode,
}

/// 语法树节点 ThrowStatementContext。对应 Java: com.alibaba.qlexpress4.aparser.QLParser 内部类 ThrowStatementContext
/// Java `ThrowStatementContext`.
#[derive(Clone, Debug)]
pub struct ThrowStatementContext {
    pub throw_token: TerminalNode,
    pub expression: Box<Node>,
}

/// 语法树节点 WhileStatementContext。对应 Java: com.alibaba.qlexpress4.aparser.QLParser 内部类 WhileStatementContext
/// Java `WhileStatementContext`.
#[derive(Clone, Debug)]
pub struct WhileStatementContext {
    pub while_token: TerminalNode,
    pub expression: Box<Node>,
    /// `None` for an empty `{}` body (Java returns null from
    /// `parseBracedBlock`).
    pub block_statements: Option<Box<Node>>,
}

/// 语法树节点 TraditionalForStatementContext。对应 Java: com.alibaba.qlexpress4.aparser.QLParser 内部类 TraditionalForStatementContext
/// Java `TraditionalForStatementContext`.
#[derive(Clone, Debug)]
pub struct TraditionalForStatementContext {
    pub for_token: TerminalNode,
    pub for_init: Box<Node>,
    pub for_condition: Option<Box<Node>>,
    pub for_update: Option<Box<Node>>,
    pub block_statements: Option<Box<Node>>,
}

/// 语法树节点 ForInitContext。对应 Java: com.alibaba.qlexpress4.aparser.QLParser 内部类 ForInitContext
/// Java `ForInitContext`. Exactly one of the optionals is `Some`; when both
/// are `None` the init was just `;`.
#[derive(Clone, Debug)]
pub struct ForInitContext {
    pub local_variable_declaration: Option<Box<Node>>,
    pub expression: Option<Box<Node>>,
    pub semi: TerminalNode,
}

/// 语法树节点 ForEachStatementContext。对应 Java: com.alibaba.qlexpress4.aparser.QLParser 内部类 ForEachStatementContext
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

/// 语法树节点 FunctionStatementContext。对应 Java: com.alibaba.qlexpress4.aparser.QLParser 内部类 FunctionStatementContext
/// Java `FunctionStatementContext`.
#[derive(Clone, Debug)]
pub struct FunctionStatementContext {
    pub function_token: TerminalNode,
    pub var_id: Box<Node>,
    pub params: Option<Box<Node>>,
    pub block_statements: Option<Box<Node>>,
}

/// 语法树节点 MacroStatementContext。对应 Java: com.alibaba.qlexpress4.aparser.QLParser 内部类 MacroStatementContext
/// Java `MacroStatementContext`.
#[derive(Clone, Debug)]
pub struct MacroStatementContext {
    pub macro_token: TerminalNode,
    pub var_id: Box<Node>,
    pub block_statements: Option<Box<Node>>,
}

/// 语法树节点 BreakContinueStatementContext。对应 Java: com.alibaba.qlexpress4.aparser.QLParser 内部类 BreakContinueStatementContext
/// Java `BreakContinueStatementContext` (`breakToken`/`continueToken` are
/// distinguished by the token type, like Java's null checks).
#[derive(Clone, Debug)]
pub struct BreakContinueStatementContext {
    pub token: TerminalNode,
}

impl BreakContinueStatementContext {
    /// 判断 break 条件。
    /// 无显式参数；返回：`bool`。
    /// 对应或承接 Java 源文件：`com/alibaba/qlexpress4/aparser/SyntaxTreeFactory.java`，方法 `isBreak`；Rust 侧按所有权与 `Result` 语义适配。
    /// Java `BREAK() != null`.
    pub fn is_break(&self) -> bool {
        self.token.symbol().token_type() == super::token::BREAK as i32
    }
}

/// 语法树节点 ReturnStatementContext。对应 Java: com.alibaba.qlexpress4.aparser.QLParser 内部类 ReturnStatementContext
/// Java `ReturnStatementContext`.
#[derive(Clone, Debug)]
pub struct ReturnStatementContext {
    pub return_token: TerminalNode,
    pub expression: Option<Box<Node>>,
}

/// 语法树节点 EmptyStatementContext。对应 Java: com.alibaba.qlexpress4.aparser.QLParser 内部类 EmptyStatementContext
/// Java `EmptyStatementContext` (a lone `;` or newline).
#[derive(Clone, Debug)]
pub struct EmptyStatementContext {
    pub token: TerminalNode,
}

/// 语法树节点 ExpressionStatementContext。对应 Java: com.alibaba.qlexpress4.aparser.QLParser 内部类 ExpressionStatementContext
/// Java `ExpressionStatementContext`.
#[derive(Clone, Debug)]
pub struct ExpressionStatementContext {
    pub expression: Box<Node>,
}

/// 语法树节点 NonExpressionStatementContext。对应 Java: com.alibaba.qlexpress4.aparser.QLParser 内部类 NonExpressionStatementContext
/// Java `NonExpressionStatementContext`: wraps a statement usable as an
/// if/else body.
#[derive(Clone, Debug)]
pub struct NonExpressionStatementContext {
    pub statement: Box<Node>,
}

/// 语法树节点 LocalVariableDeclarationContext。对应 Java: com.alibaba.qlexpress4.aparser.QLParser 内部类 LocalVariableDeclarationContext
/// Java `LocalVariableDeclarationContext`.
#[derive(Clone, Debug)]
pub struct LocalVariableDeclarationContext {
    pub decl_type: Box<Node>,
    pub variable_declarator_list: Box<Node>,
}

/// 语法树节点 VariableDeclaratorListContext。对应 Java: com.alibaba.qlexpress4.aparser.QLParser 内部类 VariableDeclaratorListContext
/// Java `VariableDeclaratorListContext`.
#[derive(Clone, Debug)]
pub struct VariableDeclaratorListContext {
    pub variables: Vec<Node>,
}

/// 语法树节点 VariableDeclaratorContext。对应 Java: com.alibaba.qlexpress4.aparser.QLParser 内部类 VariableDeclaratorContext
/// Java `VariableDeclaratorContext`.
#[derive(Clone, Debug)]
pub struct VariableDeclaratorContext {
    pub id: Box<Node>,
    pub initializer: Option<Box<Node>>,
}

/// 语法树节点 VariableDeclaratorIdContext。对应 Java: com.alibaba.qlexpress4.aparser.QLParser 内部类 VariableDeclaratorIdContext
/// Java `VariableDeclaratorIdContext`.
#[derive(Clone, Debug)]
pub struct VariableDeclaratorIdContext {
    pub var_id: Box<Node>,
    pub dims: Option<Box<Node>>,
}

/// 语法树节点 VariableInitializerContext。对应 Java: com.alibaba.qlexpress4.aparser.QLParser 内部类 VariableInitializerContext
/// Java `VariableInitializerContext`: exactly one variant is `Some`.
#[derive(Clone, Debug)]
pub struct VariableInitializerContext {
    pub expression: Option<Box<Node>>,
    pub array_initializer: Option<Box<Node>>,
}

/// 语法树节点 ArrayInitializerContext。对应 Java: com.alibaba.qlexpress4.aparser.QLParser 内部类 ArrayInitializerContext
/// Java `ArrayInitializerContext`.
#[derive(Clone, Debug)]
pub struct ArrayInitializerContext {
    pub lbrace: TerminalNode,
    pub initializers: Option<Box<Node>>,
}

/// 语法树节点 VariableInitializerListContext。对应 Java: com.alibaba.qlexpress4.aparser.QLParser 内部类 VariableInitializerListContext
/// Java `VariableInitializerListContext`.
#[derive(Clone, Debug)]
pub struct VariableInitializerListContext {
    pub initializers: Vec<Node>,
}

/// 语法树节点 DeclTypeContext。对应 Java: com.alibaba.qlexpress4.aparser.QLParser 内部类 DeclTypeContext
/// Java `DeclTypeContext`.
#[derive(Clone, Debug)]
pub struct DeclTypeContext {
    pub primitive_type: Option<Box<Node>>,
    pub cls_type: Option<Box<Node>>,
    pub dims: Option<Box<Node>>,
}

/// 语法树节点 DeclTypeNoArrContext。对应 Java: com.alibaba.qlexpress4.aparser.QLParser 内部类 DeclTypeNoArrContext
/// Java `DeclTypeNoArrContext`.
#[derive(Clone, Debug)]
pub struct DeclTypeNoArrContext {
    pub primitive_type: Option<Box<Node>>,
    pub cls_type: Option<Box<Node>>,
}

/// 语法树节点 PrimitiveTypeContext。对应 Java: com.alibaba.qlexpress4.aparser.QLParser 内部类 PrimitiveTypeContext
/// Java `PrimitiveTypeContext`.
#[derive(Clone, Debug)]
pub struct PrimitiveTypeContext {
    pub token: TerminalNode,
}

/// 语法树节点 ClsTypeContext。对应 Java: com.alibaba.qlexpress4.aparser.QLParser 内部类 ClsTypeContext
/// Java `ClsTypeContext` (type arguments are consumed but not kept, like
/// Java's `parseTypeArguments`).
#[derive(Clone, Debug)]
pub struct ClsTypeContext {
    pub var_ids: Vec<Node>,
}

/// 语法树节点 DimsContext。对应 Java: com.alibaba.qlexpress4.aparser.QLParser 内部类 DimsContext
/// Java `DimsContext` (`int[][]`): one `[`/`]` token pair per dimension.
#[derive(Clone, Debug)]
pub struct DimsContext {
    pub brackets: Vec<TerminalNode>,
}

impl DimsContext {
    /// 处理 dim count 对应的领域职责。
    /// 无显式参数；返回：`usize`。
    /// 对应或承接 Java 源文件：`com/alibaba/qlexpress4/aparser/SyntaxTreeFactory.java`，方法 `dimCount`；Rust 侧按所有权与 `Result` 语义适配。
    /// Number of `[]` dimensions (Java `LBRACK().size()`).
    pub fn dim_count(&self) -> usize {
        self.brackets.len() / 2
    }
}

/// 语法树节点 DimExprsContext。对应 Java: com.alibaba.qlexpress4.aparser.QLParser 内部类 DimExprsContext
/// Java `DimExprsContext` (`new int[3][4]`).
#[derive(Clone, Debug)]
pub struct DimExprsContext {
    pub expressions: Vec<Node>,
}

/// 语法树节点 ExpressionContext。对应 Java: com.alibaba.qlexpress4.aparser.QLParser 内部类 ExpressionContext
/// Java `ExpressionContext`: assignment or ternary.
#[derive(Clone, Debug)]
pub struct ExpressionContext {
    pub left: Option<Box<Node>>,
    pub assign_operator: Option<Box<Node>>,
    pub expression: Option<Box<Node>>,
    pub ternary: Option<Box<Node>>,
}

impl ExpressionContext {
    /// 判断 assign 条件。
    /// 无显式参数；返回：`bool`。
    /// 对应或承接 Java 源文件：`com/alibaba/qlexpress4/aparser/SyntaxTreeFactory.java`，方法 `isAssign`；Rust 侧按所有权与 `Result` 语义适配。
    /// Java `leftHandSide()`.
    pub fn is_assign(&self) -> bool {
        self.left.is_some()
    }
}

/// 语法树节点 LeftHandSideContext。对应 Java: com.alibaba.qlexpress4.aparser.QLParser 内部类 LeftHandSideContext
/// Java `LeftHandSideContext`.
#[derive(Clone, Debug)]
pub struct LeftHandSideContext {
    pub var_id: Box<Node>,
    /// `Some` when the head is a function call `f(...)`.
    pub lparen: Option<TerminalNode>,
    pub argument_list: Option<Box<Node>>,
    pub path_parts: Vec<Node>,
}

/// 语法树节点 AssignOperatorContext。对应 Java: com.alibaba.qlexpress4.aparser.QLParser 内部类 AssignOperatorContext
/// Java `AssignOperatorContext`.
#[derive(Clone, Debug)]
pub struct AssignOperatorContext {
    pub token: TerminalNode,
}

/// 语法树节点 TernaryExprContext。对应 Java: com.alibaba.qlexpress4.aparser.QLParser 内部类 TernaryExprContext
/// Java `TernaryExprContext`.
#[derive(Clone, Debug)]
pub struct TernaryExprContext {
    pub condition: Box<Node>,
    pub question: Option<TerminalNode>,
    pub then_expr: Option<Box<Node>>,
    pub else_expr: Option<Box<Node>>,
}

/// 语法树节点 BaseExprContext。对应 Java: com.alibaba.qlexpress4.aparser.QLParser 内部类 BaseExprContext
/// Java `BaseExprContext`: a primary plus left-associative binary chain.
#[derive(Clone, Debug)]
pub struct BaseExprContext {
    pub primary: Box<Node>,
    pub left_assos: Vec<Node>,
}

/// 语法树节点 LeftAssoContext。对应 Java: com.alibaba.qlexpress4.aparser.QLParser 内部类 LeftAssoContext
/// Java `LeftAssoContext`: one `op right` step.
#[derive(Clone, Debug)]
pub struct LeftAssoContext {
    pub binaryop: Box<Node>,
    pub right: Box<Node>,
}

/// 语法树节点 BinaryopContext。对应 Java: com.alibaba.qlexpress4.aparser.QLParser 内部类 BinaryopContext
/// Java `BinaryopContext`.
#[derive(Clone, Debug)]
pub struct BinaryopContext {
    pub token: TerminalNode,
}

/// 语法树节点 PrimaryContext。对应 Java: com.alibaba.qlexpress4.aparser.QLParser 内部类 PrimaryContext
/// Java `PrimaryContext`.
#[derive(Clone, Debug)]
pub struct PrimaryContext {
    pub prefix: Option<Box<Node>>,
    pub pathable: Option<Box<Node>>,
    pub path_parts: Vec<Node>,
    pub suffix: Option<Box<Node>>,
    pub non_pathable: Option<Box<Node>>,
}

/// 语法树节点 PrefixExpressContext。对应 Java: com.alibaba.qlexpress4.aparser.QLParser 内部类 PrefixExpressContext
/// Java `PrefixExpressContext`.
#[derive(Clone, Debug)]
pub struct PrefixExpressContext {
    pub op_id: Box<Node>,
}

/// 语法树节点 SuffixExpressContext。对应 Java: com.alibaba.qlexpress4.aparser.QLParser 内部类 SuffixExpressContext
/// Java `SuffixExpressContext`.
#[derive(Clone, Debug)]
pub struct SuffixExpressContext {
    pub op_id: Box<Node>,
}

/// 语法树节点 ConstExprContext。对应 Java: com.alibaba.qlexpress4.aparser.QLParser 内部类 ConstExprContext
/// Java `ConstExprContext`.
#[derive(Clone, Debug)]
pub struct ConstExprContext {
    pub literal: Box<Node>,
}

/// 语法树节点 CastExprContext。对应 Java: com.alibaba.qlexpress4.aparser.QLParser 内部类 CastExprContext
/// Java `CastExprContext`.
#[derive(Clone, Debug)]
pub struct CastExprContext {
    pub lparen: TerminalNode,
    pub decl_type: Box<Node>,
    pub primary: Box<Node>,
}

/// 语法树节点 GroupExprContext。对应 Java: com.alibaba.qlexpress4.aparser.QLParser 内部类 GroupExprContext
/// Java `GroupExprContext` (parenthesised expression).
#[derive(Clone, Debug)]
pub struct GroupExprContext {
    pub lparen: TerminalNode,
    pub expression: Box<Node>,
}

/// 语法树节点 NewObjExprContext。对应 Java: com.alibaba.qlexpress4.aparser.QLParser 内部类 NewObjExprContext
/// Java `NewObjExprContext`.
#[derive(Clone, Debug)]
pub struct NewObjExprContext {
    pub new_token: TerminalNode,
    pub var_ids: Vec<Node>,
    pub argument_list: Option<Box<Node>>,
}

/// 语法树节点 NewEmptyArrExprContext。对应 Java: com.alibaba.qlexpress4.aparser.QLParser 内部类 NewEmptyArrExprContext
/// Java `NewEmptyArrExprContext` (`new int[3]`).
#[derive(Clone, Debug)]
pub struct NewEmptyArrExprContext {
    pub new_token: TerminalNode,
    pub decl_type_no_arr: Box<Node>,
    pub dim_exprs: Box<Node>,
}

/// 语法树节点 NewInitArrExprContext。对应 Java: com.alibaba.qlexpress4.aparser.QLParser 内部类 NewInitArrExprContext
/// Java `NewInitArrExprContext` (`new int[]{1,2}`).
#[derive(Clone, Debug)]
pub struct NewInitArrExprContext {
    pub new_token: TerminalNode,
    pub decl_type_no_arr: Box<Node>,
    pub dims: Box<Node>,
    pub array_initializer: Box<Node>,
}

/// 语法树节点 VarIdExprContext。对应 Java: com.alibaba.qlexpress4.aparser.QLParser 内部类 VarIdExprContext
/// Java `VarIdExprContext` (variable reference or function call head).
#[derive(Clone, Debug)]
pub struct VarIdExprContext {
    pub var_id: Box<Node>,
    pub lparen: Option<TerminalNode>,
    pub argument_list: Option<Box<Node>>,
}

/// 语法树节点 TypeExprContext。对应 Java: com.alibaba.qlexpress4.aparser.QLParser 内部类 TypeExprContext
/// Java `TypeExprContext` (a primitive type used as a value, e.g. `int.class`).
#[derive(Clone, Debug)]
pub struct TypeExprContext {
    pub primitive_type: Box<Node>,
}

/// 语法树节点 ListItemsContext。对应 Java: com.alibaba.qlexpress4.aparser.QLParser 内部类 ListItemsContext
/// Java `ListItemsContext`.
#[derive(Clone, Debug)]
pub struct ListItemsContext {
    pub expressions: Vec<Node>,
}

/// 语法树节点 ListExprContext。对应 Java: com.alibaba.qlexpress4.aparser.QLParser 内部类 ListExprContext
/// Java `ListExprContext`.
#[derive(Clone, Debug)]
pub struct ListExprContext {
    pub lbrack: TerminalNode,
    pub list_items: Option<Box<Node>>,
}

/// 语法树节点 MapExprContext。对应 Java: com.alibaba.qlexpress4.aparser.QLParser 内部类 MapExprContext
/// Java `MapExprContext`.
#[derive(Clone, Debug)]
pub struct MapExprContext {
    pub lbrace: TerminalNode,
    pub map_entries: Box<Node>,
}

/// 语法树节点 BlockExprContext。对应 Java: com.alibaba.qlexpress4.aparser.QLParser 内部类 BlockExprContext
/// Java `BlockExprContext` (a block used as an expression).
#[derive(Clone, Debug)]
pub struct BlockExprContext {
    pub lbrace: TerminalNode,
    pub block_statements: Option<Box<Node>>,
}

/// 语法树节点 ContextSelectExprContext。对应 Java: com.alibaba.qlexpress4.aparser.QLParser 内部类 ContextSelectExprContext
/// Java `ContextSelectExprContext` (selector expression).
#[derive(Clone, Debug)]
pub struct ContextSelectExprContext {
    pub selector_start: TerminalNode,
    pub selector_variable: TerminalNode,
}

/// 语法树节点 QlIfContext。对应 Java: com.alibaba.qlexpress4.aparser.QLParser 内部类 QlIfContext
/// Java `QlIfContext`.
#[derive(Clone, Debug)]
pub struct QlIfContext {
    pub if_token: TerminalNode,
    pub then_keyword: Option<TerminalNode>,
    pub condition: Box<Node>,
    pub then_body: Box<Node>,
    pub else_body: Option<Box<Node>>,
}

/// 语法树节点 ThenBodyContext。对应 Java: com.alibaba.qlexpress4.aparser.QLParser 内部类 ThenBodyContext
/// Java `ThenBodyContext`: exactly one of the optionals is `Some`.
#[derive(Clone, Debug)]
pub struct ThenBodyContext {
    pub lbrace: Option<TerminalNode>,
    pub block_statements: Option<Box<Node>>,
    pub non_expression_statement: Option<Box<Node>>,
    pub expression: Option<Box<Node>>,
}

/// 语法树节点 ElseBodyContext。对应 Java: com.alibaba.qlexpress4.aparser.QLParser 内部类 ElseBodyContext
/// Java `ElseBodyContext`: exactly one of the optionals is `Some`.
#[derive(Clone, Debug)]
pub struct ElseBodyContext {
    pub lbrace: Option<TerminalNode>,
    pub block_statements: Option<Box<Node>>,
    pub ql_if: Option<Box<Node>>,
    pub non_expression_statement: Option<Box<Node>>,
    pub expression: Option<Box<Node>>,
}

/// 语法树节点 SwitchExprContext。对应 Java: com.alibaba.qlexpress4.aparser.QLParser 内部类 SwitchExprContext
/// Java `SwitchExprContext`.
#[derive(Clone, Debug)]
pub struct SwitchExprContext {
    pub switch_token: TerminalNode,
    pub expression: Box<Node>,
    pub groups: Option<Box<Node>>,
}

/// 语法树节点 SwitchCaseGroupsContext。对应 Java: com.alibaba.qlexpress4.aparser.QLParser 内部类 SwitchCaseGroupsContext
/// Java `SwitchCaseGroupsContext`.
#[derive(Clone, Debug)]
pub struct SwitchCaseGroupsContext {
    pub groups: Vec<Node>,
}

/// 语法树节点 SwitchStatementGroupContext。对应 Java: com.alibaba.qlexpress4.aparser.QLParser 内部类 SwitchStatementGroupContext
/// Java `SwitchStatementGroupContext`.
#[derive(Clone, Debug)]
pub struct SwitchStatementGroupContext {
    pub labels: Box<Node>,
    pub block_statements: Option<Box<Node>>,
}

/// 语法树节点 SwitchExprGroupContext。对应 Java: com.alibaba.qlexpress4.aparser.QLParser 内部类 SwitchExprGroupContext
/// Java `SwitchExprGroupContext` (`case a -> expr`).
#[derive(Clone, Debug)]
pub struct SwitchExprGroupContext {
    pub label: Box<Node>,
    pub expression: Box<Node>,
}

/// 语法树节点 SwitchLabelsContext。对应 Java: com.alibaba.qlexpress4.aparser.QLParser 内部类 SwitchLabelsContext
/// Java `SwitchLabelsContext`.
#[derive(Clone, Debug)]
pub struct SwitchLabelsContext {
    pub labels: Vec<Node>,
}

/// 语法树节点 SwitchLabelContext。对应 Java: com.alibaba.qlexpress4.aparser.QLParser 内部类 SwitchLabelContext
/// Java `SwitchLabelContext`.
#[derive(Clone, Debug)]
pub struct SwitchLabelContext {
    pub case_token: Option<TerminalNode>,
    pub default_token: Option<TerminalNode>,
    pub expression: Option<Box<Node>>,
}

/// 语法树节点 SwitchExpressionLabelContext。对应 Java: com.alibaba.qlexpress4.aparser.QLParser 内部类 SwitchExpressionLabelContext
/// Java `SwitchExpressionLabelContext`.
#[derive(Clone, Debug)]
pub struct SwitchExpressionLabelContext {
    pub case_token: Option<TerminalNode>,
    pub default_token: Option<TerminalNode>,
    pub expression_list: Option<Box<Node>>,
    pub arrow: TerminalNode,
}

/// 语法树节点 ExpressionListContext。对应 Java: com.alibaba.qlexpress4.aparser.QLParser 内部类 ExpressionListContext
/// Java `ExpressionListContext`.
#[derive(Clone, Debug)]
pub struct ExpressionListContext {
    pub expressions: Vec<Node>,
}

/// 语法树节点 TryCatchExprContext。对应 Java: com.alibaba.qlexpress4.aparser.QLParser 内部类 TryCatchExprContext
/// Java `TryCatchExprContext`.
#[derive(Clone, Debug)]
pub struct TryCatchExprContext {
    pub try_token: TerminalNode,
    pub block_statements: Option<Box<Node>>,
    pub try_catches: Option<Box<Node>>,
    pub try_finally: Option<Box<Node>>,
}

/// 语法树节点 TryCatchesContext。对应 Java: com.alibaba.qlexpress4.aparser.QLParser 内部类 TryCatchesContext
/// Java `TryCatchesContext`.
#[derive(Clone, Debug)]
pub struct TryCatchesContext {
    pub catches: Vec<Node>,
}

/// 语法树节点 TryCatchContext。对应 Java: com.alibaba.qlexpress4.aparser.QLParser 内部类 TryCatchContext
/// Java `TryCatchContext` (one catch clause).
#[derive(Clone, Debug)]
pub struct TryCatchContext {
    pub catch_token: TerminalNode,
    pub catch_params: Box<Node>,
    pub block_statements: Option<Box<Node>>,
}

/// 语法树节点 CatchParamsContext。对应 Java: com.alibaba.qlexpress4.aparser.QLParser 内部类 CatchParamsContext
/// Java `CatchParamsContext`.
#[derive(Clone, Debug)]
pub struct CatchParamsContext {
    pub decl_types: Vec<Node>,
    pub var_id: Box<Node>,
}

/// 语法树节点 TryFinallyContext。对应 Java: com.alibaba.qlexpress4.aparser.QLParser 内部类 TryFinallyContext
/// Java `TryFinallyContext`.
#[derive(Clone, Debug)]
pub struct TryFinallyContext {
    pub finally_token: TerminalNode,
    pub block_statements: Option<Box<Node>>,
}

/// 语法树节点 MapEntriesContext。对应 Java: com.alibaba.qlexpress4.aparser.QLParser 内部类 MapEntriesContext
/// Java `MapEntriesContext`. `empty_colon` is `Some` for the empty-map
/// literal `{:}`.
#[derive(Clone, Debug)]
pub struct MapEntriesContext {
    pub empty_colon: Option<TerminalNode>,
    pub entries: Vec<Node>,
}

/// 语法树节点 MapEntryContext。对应 Java: com.alibaba.qlexpress4.aparser.QLParser 内部类 MapEntryContext
/// Java `MapEntryContext`.
#[derive(Clone, Debug)]
pub struct MapEntryContext {
    pub map_key: Box<Node>,
    pub map_value: Box<Node>,
}

/// 语法树节点 ClsValueContext。对应 Java: com.alibaba.qlexpress4.aparser.QLParser 内部类 ClsValueContext
/// Java `ClsValueContext` (map value of the special `'@class'` key).
#[derive(Clone, Debug)]
pub struct ClsValueContext {
    pub quote: TerminalNode,
}

/// 语法树节点 EValueContext。对应 Java: com.alibaba.qlexpress4.aparser.QLParser 内部类 EValueContext
/// Java `EValueContext` (ordinary map value).
#[derive(Clone, Debug)]
pub struct EValueContext {
    pub expression: Box<Node>,
}

/// 语法树节点 IdKeyContext。对应 Java: com.alibaba.qlexpress4.aparser.QLParser 内部类 IdKeyContext
/// Java `IdKeyContext`.
#[derive(Clone, Debug)]
pub struct IdKeyContext {
    pub token: TerminalNode,
}

/// 语法树节点 StringKeyContext。对应 Java: com.alibaba.qlexpress4.aparser.QLParser 内部类 StringKeyContext
/// Java `StringKeyContext` (double-quoted map key).
#[derive(Clone, Debug)]
pub struct StringKeyContext {
    pub double_quote_string: Box<Node>,
}

/// 语法树节点 QuoteStringKeyContext。对应 Java: com.alibaba.qlexpress4.aparser.QLParser 内部类 QuoteStringKeyContext
/// Java `QuoteStringKeyContext` (single-quoted map key).
#[derive(Clone, Debug)]
pub struct QuoteStringKeyContext {
    pub token: TerminalNode,
}

/// `ChainKind` 枚举的 Rust 实现，保留对应对象的领域职责与公开契约。
/// 对应或承接 Java 源文件：`com/alibaba/qlexpress4/aparser/SyntaxTreeFactory.java`；具体对象路径见 `docs/对象级对照表.md`。
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

/// 语法树节点 MethodInvokeContext。对应 Java: com.alibaba.qlexpress4.aparser.QLParser 内部类 MethodInvokeContext
/// Java `MethodInvokeContext` (plus the optional/spread subclasses).
#[derive(Clone, Debug)]
pub struct MethodInvokeContext {
    /// The `.` / `?.` / `*.` token (Java stores it as the first child).
    pub dot: TerminalNode,
    pub var_id: Box<Node>,
    pub argument_list: Option<Box<Node>>,
    pub chain: ChainKind,
}

/// 语法树节点 FieldAccessContext。对应 Java: com.alibaba.qlexpress4.aparser.QLParser 内部类 FieldAccessContext
/// Java `FieldAccessContext` (plus the optional/spread subclasses).
#[derive(Clone, Debug)]
pub struct FieldAccessContext {
    /// The `.` / `?.` / `*.` token.
    pub dot: TerminalNode,
    pub field_id: Box<Node>,
    pub chain: ChainKind,
}

/// 语法树节点 MethodAccessContext。对应 Java: com.alibaba.qlexpress4.aparser.QLParser 内部类 MethodAccessContext
/// Java `MethodAccessContext` (`Cls::method`).
#[derive(Clone, Debug)]
pub struct MethodAccessContext {
    pub dcolon: TerminalNode,
    pub var_id: Box<Node>,
}

/// 语法树节点 IndexExprContext。对应 Java: com.alibaba.qlexpress4.aparser.QLParser 内部类 IndexExprContext
/// Java `IndexExprContext` (`a[i]` / `a[i:j]`); `None` index for `a[]`.
#[derive(Clone, Debug)]
pub struct IndexExprContext {
    pub lbrack: TerminalNode,
    pub index_value_expr: Option<Box<Node>>,
}

/// 语法树节点 CustomPathContext。对应 Java: com.alibaba.qlexpress4.aparser.QLParser 内部类 CustomPathContext
/// Java `CustomPathContext` (custom operator path, e.g. `a %% 'path'`).
#[derive(Clone, Debug)]
pub struct CustomPathContext {
    pub op_id: Box<Node>,
    pub var_id: Option<Box<Node>>,
    pub quote: Option<TerminalNode>,
    pub path_text: String,
}

/// 语法树节点 FieldIdContext。对应 Java: com.alibaba.qlexpress4.aparser.QLParser 内部类 FieldIdContext
/// Java `FieldIdContext`.
#[derive(Clone, Debug)]
pub struct FieldIdContext {
    pub token: Option<TerminalNode>,
    pub quote: Option<TerminalNode>,
}

/// 语法树节点 SingleIndexContext。对应 Java: com.alibaba.qlexpress4.aparser.QLParser 内部类 SingleIndexContext
/// Java `SingleIndexContext`.
#[derive(Clone, Debug)]
pub struct SingleIndexContext {
    pub expression: Box<Node>,
}

/// 语法树节点 SliceIndexContext。对应 Java: com.alibaba.qlexpress4.aparser.QLParser 内部类 SliceIndexContext
/// Java `SliceIndexContext` (`a[start:end]`).
#[derive(Clone, Debug)]
pub struct SliceIndexContext {
    pub start: Option<Box<Node>>,
    pub colon: TerminalNode,
    pub end: Option<Box<Node>>,
}

/// 语法树节点 ArgumentListContext。对应 Java: com.alibaba.qlexpress4.aparser.QLParser 内部类 ArgumentListContext
/// Java `ArgumentListContext`.
#[derive(Clone, Debug)]
pub struct ArgumentListContext {
    pub expressions: Vec<Node>,
}

/// 语法树节点 LiteralContext。对应 Java: com.alibaba.qlexpress4.aparser.QLParser 内部类 LiteralContext
/// Java `LiteralContext`: exactly one of the fields is `Some`.
#[derive(Clone, Debug)]
pub struct LiteralContext {
    /// Number / single-quoted string / `null` token.
    pub token: Option<TerminalNode>,
    pub boolen: Option<Box<Node>>,
    pub double_quote_string: Option<Box<Node>>,
}

/// `DyStrPart` 枚举的 Rust 实现，保留对应对象的领域职责与公开契约。
/// 对应或承接 Java 源文件：`com/alibaba/qlexpress4/aparser/SyntaxTreeFactory.java`；具体对象路径见 `docs/对象级对照表.md`。
/// One piece of a double-quoted string: literal text or an interpolation.
#[derive(Clone, Debug)]
pub enum DyStrPart {
    /// Java `DyStrText` token.
    Text(TerminalNode),
    /// Java `StringExpressionContext`.
    Expr(Box<Node>),
}

/// 语法树节点 DoubleQuoteStringLiteralContext。对应 Java: com.alibaba.qlexpress4.aparser.QLParser 内部类 DoubleQuoteStringLiteralContext
/// Java `DoubleQuoteStringLiteralContext`.
#[derive(Clone, Debug)]
pub struct DoubleQuoteStringLiteralContext {
    pub open_quote: TerminalNode,
    pub static_characters: Option<TerminalNode>,
    pub parts: Vec<DyStrPart>,
    pub close_quote: TerminalNode,
}

/// 语法树节点 StringExpressionContext。对应 Java: com.alibaba.qlexpress4.aparser.QLParser 内部类 StringExpressionContext
/// Java `StringExpressionContext` (`${expr}` or `${#var}` inside a string).
#[derive(Clone, Debug)]
pub struct StringExpressionContext {
    pub start: TerminalNode,
    pub selector_variable: Option<TerminalNode>,
    pub expression: Option<Box<Node>>,
}

/// 语法树节点 BoolenLiteralContext。对应 Java: com.alibaba.qlexpress4.aparser.QLParser 内部类 BoolenLiteralContext
/// Java `BoolenLiteralContext` (sic, Java spelling).
#[derive(Clone, Debug)]
pub struct BoolenLiteralContext {
    pub token: TerminalNode,
}

/// 语法树节点 LambdaExprContext。对应 Java: com.alibaba.qlexpress4.aparser.QLParser 内部类 LambdaExprContext
/// Java `LambdaExprContext`.
#[derive(Clone, Debug)]
pub struct LambdaExprContext {
    pub lambda_parameters: Box<Node>,
    pub arrow: TerminalNode,
    pub lbrace: Option<TerminalNode>,
    pub block_statements: Option<Box<Node>>,
    pub expression: Option<Box<Node>>,
}

/// 语法树节点 LambdaParametersContext。对应 Java: com.alibaba.qlexpress4.aparser.QLParser 内部类 LambdaParametersContext
/// Java `LambdaParametersContext`: single id (`x -> ..`) or parameter list.
#[derive(Clone, Debug)]
pub struct LambdaParametersContext {
    pub var_id: Option<Box<Node>>,
    pub params: Option<Box<Node>>,
}

/// 语法树节点 FormalOrInferredParameterListContext。对应 Java: com.alibaba.qlexpress4.aparser.QLParser 内部类 FormalOrInferredParameterListContext
/// Java `FormalOrInferredParameterListContext`.
#[derive(Clone, Debug)]
pub struct FormalOrInferredParameterListContext {
    pub params: Vec<Node>,
}

/// 语法树节点 FormalOrInferredParameterContext。对应 Java: com.alibaba.qlexpress4.aparser.QLParser 内部类 FormalOrInferredParameterContext
/// Java `FormalOrInferredParameterContext`.
#[derive(Clone, Debug)]
pub struct FormalOrInferredParameterContext {
    pub decl_type: Option<Box<Node>>,
    pub var_id: Box<Node>,
}

/// 语法树节点 ImportClsContext。对应 Java: com.alibaba.qlexpress4.aparser.QLParser 内部类 ImportClsContext
/// Java `ImportClsContext` (`import a.b.C;`).
#[derive(Clone, Debug)]
pub struct ImportClsContext {
    pub import_token: TerminalNode,
    pub var_ids: Vec<Node>,
}

/// 语法树节点 ImportPackContext。对应 Java: com.alibaba.qlexpress4.aparser.QLParser 内部类 ImportPackContext
/// Java `ImportPackContext` (`import a.b.*;` / `import a.b.*;`).
#[derive(Clone, Debug)]
pub struct ImportPackContext {
    pub import_token: TerminalNode,
    pub var_ids: Vec<Node>,
}

/// 语法树节点 OpIdContext。对应 Java: com.alibaba.qlexpress4.aparser.QLParser 内部类 OpIdContext
/// Java `OpIdContext` (prefix/suffix/custom operator token).
#[derive(Clone, Debug)]
pub struct OpIdContext {
    pub token: TerminalNode,
}

/// 语法树节点 VarIdContext。对应 Java: com.alibaba.qlexpress4.aparser.QLParser 内部类 VarIdContext
/// Java `VarIdContext` (an identifier token).
#[derive(Clone, Debug)]
pub struct VarIdContext {
    pub token: TerminalNode,
}

// ---------------------------------------------------------------------------
// Node enum: one variant per Java QLParser *Context class.
// ---------------------------------------------------------------------------

/// 语法树节点:一个变体对应 Java `QLParser` 的一个内部 *Context 类。
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
