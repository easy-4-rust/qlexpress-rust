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
    /// 该语法规则中的 `local_variable_declaration` 子节点、终结符或节点集合。
    pub local_variable_declaration: Box<Node>,
    /// 该语法规则中的 `semi` 子节点、终结符或节点集合。
    pub semi: TerminalNode,
}

/// 语法树节点 ThrowStatementContext。对应 Java: com.alibaba.qlexpress4.aparser.QLParser 内部类 ThrowStatementContext
/// Java `ThrowStatementContext`.
#[derive(Clone, Debug)]
pub struct ThrowStatementContext {
    /// 该语法规则中的 `throw_token` 子节点、终结符或节点集合。
    pub throw_token: TerminalNode,
    /// 该语法规则中的 `expression` 子节点、终结符或节点集合。
    pub expression: Box<Node>,
}

/// 语法树节点 WhileStatementContext。对应 Java: com.alibaba.qlexpress4.aparser.QLParser 内部类 WhileStatementContext
/// Java `WhileStatementContext`.
#[derive(Clone, Debug)]
pub struct WhileStatementContext {
    /// 该语法规则中的 `while_token` 子节点、终结符或节点集合。
    pub while_token: TerminalNode,
    /// 该语法规则中的 `expression` 子节点、终结符或节点集合。
    pub expression: Box<Node>,
    /// `None` for an empty `{}` body (Java returns null from
    /// `parseBracedBlock`).
    pub block_statements: Option<Box<Node>>,
}

/// 语法树节点 TraditionalForStatementContext。对应 Java: com.alibaba.qlexpress4.aparser.QLParser 内部类 TraditionalForStatementContext
/// Java `TraditionalForStatementContext`.
#[derive(Clone, Debug)]
pub struct TraditionalForStatementContext {
    /// 该语法规则中的 `for_token` 子节点、终结符或节点集合。
    pub for_token: TerminalNode,
    /// 该语法规则中的 `for_init` 子节点、终结符或节点集合。
    pub for_init: Box<Node>,
    /// 该语法规则中的 `for_condition` 子节点、终结符或节点集合。
    pub for_condition: Option<Box<Node>>,
    /// 该语法规则中的 `for_update` 子节点、终结符或节点集合。
    pub for_update: Option<Box<Node>>,
    /// 该语法规则中的 `block_statements` 子节点、终结符或节点集合。
    pub block_statements: Option<Box<Node>>,
}

/// 语法树节点 ForInitContext。对应 Java: com.alibaba.qlexpress4.aparser.QLParser 内部类 ForInitContext
/// Java `ForInitContext`. Exactly one of the optionals is `Some`; when both
/// are `None` the init was just `;`.
#[derive(Clone, Debug)]
pub struct ForInitContext {
    /// 该语法规则中的 `local_variable_declaration` 子节点、终结符或节点集合。
    pub local_variable_declaration: Option<Box<Node>>,
    /// 该语法规则中的 `expression` 子节点、终结符或节点集合。
    pub expression: Option<Box<Node>>,
    /// 该语法规则中的 `semi` 子节点、终结符或节点集合。
    pub semi: TerminalNode,
}

/// 语法树节点 ForEachStatementContext。对应 Java: com.alibaba.qlexpress4.aparser.QLParser 内部类 ForEachStatementContext
/// Java `ForEachStatementContext`.
#[derive(Clone, Debug)]
pub struct ForEachStatementContext {
    /// 该语法规则中的 `for_token` 子节点、终结符或节点集合。
    pub for_token: TerminalNode,
    /// Declared element type; `None` for `for (x : xs)` (inferred).
    pub decl_type: Option<Box<Node>>,
    /// 该语法规则中的 `var_id` 子节点、终结符或节点集合。
    pub var_id: Box<Node>,
    /// 该语法规则中的 `expression` 子节点、终结符或节点集合。
    pub expression: Box<Node>,
    /// 该语法规则中的 `block_statements` 子节点、终结符或节点集合。
    pub block_statements: Option<Box<Node>>,
}

/// 语法树节点 FunctionStatementContext。对应 Java: com.alibaba.qlexpress4.aparser.QLParser 内部类 FunctionStatementContext
/// Java `FunctionStatementContext`.
#[derive(Clone, Debug)]
pub struct FunctionStatementContext {
    /// 该语法规则中的 `function_token` 子节点、终结符或节点集合。
    pub function_token: TerminalNode,
    /// 该语法规则中的 `var_id` 子节点、终结符或节点集合。
    pub var_id: Box<Node>,
    /// 该语法规则中的 `params` 子节点、终结符或节点集合。
    pub params: Option<Box<Node>>,
    /// 该语法规则中的 `block_statements` 子节点、终结符或节点集合。
    pub block_statements: Option<Box<Node>>,
}

/// 语法树节点 MacroStatementContext。对应 Java: com.alibaba.qlexpress4.aparser.QLParser 内部类 MacroStatementContext
/// Java `MacroStatementContext`.
#[derive(Clone, Debug)]
pub struct MacroStatementContext {
    /// 该语法规则中的 `macro_token` 子节点、终结符或节点集合。
    pub macro_token: TerminalNode,
    /// 该语法规则中的 `var_id` 子节点、终结符或节点集合。
    pub var_id: Box<Node>,
    /// 该语法规则中的 `block_statements` 子节点、终结符或节点集合。
    pub block_statements: Option<Box<Node>>,
}

/// 语法树节点 BreakContinueStatementContext。对应 Java: com.alibaba.qlexpress4.aparser.QLParser 内部类 BreakContinueStatementContext
/// Java `BreakContinueStatementContext` (`breakToken`/`continueToken` are
/// distinguished by the token type, like Java's null checks).
#[derive(Clone, Debug)]
pub struct BreakContinueStatementContext {
    /// 该语法规则中的 `token` 子节点、终结符或节点集合。
    pub token: TerminalNode,
}

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

/// 语法树节点 ReturnStatementContext。对应 Java: com.alibaba.qlexpress4.aparser.QLParser 内部类 ReturnStatementContext
/// Java `ReturnStatementContext`.
#[derive(Clone, Debug)]
pub struct ReturnStatementContext {
    /// 该语法规则中的 `return_token` 子节点、终结符或节点集合。
    pub return_token: TerminalNode,
    /// 该语法规则中的 `expression` 子节点、终结符或节点集合。
    pub expression: Option<Box<Node>>,
}

/// 语法树节点 EmptyStatementContext。对应 Java: com.alibaba.qlexpress4.aparser.QLParser 内部类 EmptyStatementContext
/// Java `EmptyStatementContext` (a lone `;` or newline).
#[derive(Clone, Debug)]
pub struct EmptyStatementContext {
    /// 该语法规则中的 `token` 子节点、终结符或节点集合。
    pub token: TerminalNode,
}

/// 语法树节点 ExpressionStatementContext。对应 Java: com.alibaba.qlexpress4.aparser.QLParser 内部类 ExpressionStatementContext
/// Java `ExpressionStatementContext`.
#[derive(Clone, Debug)]
pub struct ExpressionStatementContext {
    /// 该语法规则中的 `expression` 子节点、终结符或节点集合。
    pub expression: Box<Node>,
}

/// 语法树节点 NonExpressionStatementContext。对应 Java: com.alibaba.qlexpress4.aparser.QLParser 内部类 NonExpressionStatementContext
/// Java `NonExpressionStatementContext`: wraps a statement usable as an
/// if/else body.
#[derive(Clone, Debug)]
pub struct NonExpressionStatementContext {
    /// 该语法规则中的 `statement` 子节点、终结符或节点集合。
    pub statement: Box<Node>,
}

/// 语法树节点 LocalVariableDeclarationContext。对应 Java: com.alibaba.qlexpress4.aparser.QLParser 内部类 LocalVariableDeclarationContext
/// Java `LocalVariableDeclarationContext`.
#[derive(Clone, Debug)]
pub struct LocalVariableDeclarationContext {
    /// 该语法规则中的 `decl_type` 子节点、终结符或节点集合。
    pub decl_type: Box<Node>,
    /// 该语法规则中的 `variable_declarator_list` 子节点、终结符或节点集合。
    pub variable_declarator_list: Box<Node>,
}

/// 语法树节点 VariableDeclaratorListContext。对应 Java: com.alibaba.qlexpress4.aparser.QLParser 内部类 VariableDeclaratorListContext
/// Java `VariableDeclaratorListContext`.
#[derive(Clone, Debug)]
pub struct VariableDeclaratorListContext {
    /// 该语法规则中的 `variables` 子节点、终结符或节点集合。
    pub variables: Vec<Node>,
}

/// 语法树节点 VariableDeclaratorContext。对应 Java: com.alibaba.qlexpress4.aparser.QLParser 内部类 VariableDeclaratorContext
/// Java `VariableDeclaratorContext`.
#[derive(Clone, Debug)]
pub struct VariableDeclaratorContext {
    /// 该语法规则中的 `id` 子节点、终结符或节点集合。
    pub id: Box<Node>,
    /// 该语法规则中的 `initializer` 子节点、终结符或节点集合。
    pub initializer: Option<Box<Node>>,
}

/// 语法树节点 VariableDeclaratorIdContext。对应 Java: com.alibaba.qlexpress4.aparser.QLParser 内部类 VariableDeclaratorIdContext
/// Java `VariableDeclaratorIdContext`.
#[derive(Clone, Debug)]
pub struct VariableDeclaratorIdContext {
    /// 该语法规则中的 `var_id` 子节点、终结符或节点集合。
    pub var_id: Box<Node>,
    /// 该语法规则中的 `dims` 子节点、终结符或节点集合。
    pub dims: Option<Box<Node>>,
}

/// 语法树节点 VariableInitializerContext。对应 Java: com.alibaba.qlexpress4.aparser.QLParser 内部类 VariableInitializerContext
/// Java `VariableInitializerContext`: exactly one variant is `Some`.
#[derive(Clone, Debug)]
pub struct VariableInitializerContext {
    /// 该语法规则中的 `expression` 子节点、终结符或节点集合。
    pub expression: Option<Box<Node>>,
    /// 该语法规则中的 `array_initializer` 子节点、终结符或节点集合。
    pub array_initializer: Option<Box<Node>>,
}

/// 语法树节点 ArrayInitializerContext。对应 Java: com.alibaba.qlexpress4.aparser.QLParser 内部类 ArrayInitializerContext
/// Java `ArrayInitializerContext`.
#[derive(Clone, Debug)]
pub struct ArrayInitializerContext {
    /// 该语法规则中的 `lbrace` 子节点、终结符或节点集合。
    pub lbrace: TerminalNode,
    /// 该语法规则中的 `initializers` 子节点、终结符或节点集合。
    pub initializers: Option<Box<Node>>,
}

/// 语法树节点 VariableInitializerListContext。对应 Java: com.alibaba.qlexpress4.aparser.QLParser 内部类 VariableInitializerListContext
/// Java `VariableInitializerListContext`.
#[derive(Clone, Debug)]
pub struct VariableInitializerListContext {
    /// 该语法规则中的 `initializers` 子节点、终结符或节点集合。
    pub initializers: Vec<Node>,
}

/// 语法树节点 DeclTypeContext。对应 Java: com.alibaba.qlexpress4.aparser.QLParser 内部类 DeclTypeContext
/// Java `DeclTypeContext`.
#[derive(Clone, Debug)]
pub struct DeclTypeContext {
    /// 该语法规则中的 `primitive_type` 子节点、终结符或节点集合。
    pub primitive_type: Option<Box<Node>>,
    /// 该语法规则中的 `cls_type` 子节点、终结符或节点集合。
    pub cls_type: Option<Box<Node>>,
    /// 该语法规则中的 `dims` 子节点、终结符或节点集合。
    pub dims: Option<Box<Node>>,
}

/// 语法树节点 DeclTypeNoArrContext。对应 Java: com.alibaba.qlexpress4.aparser.QLParser 内部类 DeclTypeNoArrContext
/// Java `DeclTypeNoArrContext`.
#[derive(Clone, Debug)]
pub struct DeclTypeNoArrContext {
    /// 该语法规则中的 `primitive_type` 子节点、终结符或节点集合。
    pub primitive_type: Option<Box<Node>>,
    /// 该语法规则中的 `cls_type` 子节点、终结符或节点集合。
    pub cls_type: Option<Box<Node>>,
}

/// 语法树节点 PrimitiveTypeContext。对应 Java: com.alibaba.qlexpress4.aparser.QLParser 内部类 PrimitiveTypeContext
/// Java `PrimitiveTypeContext`.
#[derive(Clone, Debug)]
pub struct PrimitiveTypeContext {
    /// 该语法规则中的 `token` 子节点、终结符或节点集合。
    pub token: TerminalNode,
}

/// 语法树节点 ClsTypeContext。对应 Java: com.alibaba.qlexpress4.aparser.QLParser 内部类 ClsTypeContext
/// Java `ClsTypeContext` (type arguments are consumed but not kept, like
/// Java's `parseTypeArguments`).
#[derive(Clone, Debug)]
pub struct ClsTypeContext {
    /// 该语法规则中的 `var_ids` 子节点、终结符或节点集合。
    pub var_ids: Vec<Node>,
}

/// 语法树节点 DimsContext。对应 Java: com.alibaba.qlexpress4.aparser.QLParser 内部类 DimsContext
/// Java `DimsContext` (`int[][]`): one `[`/`]` token pair per dimension.
#[derive(Clone, Debug)]
pub struct DimsContext {
    /// 该语法规则中的 `brackets` 子节点、终结符或节点集合。
    pub brackets: Vec<TerminalNode>,
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

/// 语法树节点 DimExprsContext。对应 Java: com.alibaba.qlexpress4.aparser.QLParser 内部类 DimExprsContext
/// Java `DimExprsContext` (`new int[3][4]`).
#[derive(Clone, Debug)]
pub struct DimExprsContext {
    /// 该语法规则中的 `expressions` 子节点、终结符或节点集合。
    pub expressions: Vec<Node>,
}

/// 语法树节点 ExpressionContext。对应 Java: com.alibaba.qlexpress4.aparser.QLParser 内部类 ExpressionContext
/// Java `ExpressionContext`: assignment or ternary.
#[derive(Clone, Debug)]
pub struct ExpressionContext {
    /// 该语法规则中的 `left` 子节点、终结符或节点集合。
    pub left: Option<Box<Node>>,
    /// 该语法规则中的 `assign_operator` 子节点、终结符或节点集合。
    pub assign_operator: Option<Box<Node>>,
    /// 该语法规则中的 `expression` 子节点、终结符或节点集合。
    pub expression: Option<Box<Node>>,
    /// 该语法规则中的 `ternary` 子节点、终结符或节点集合。
    pub ternary: Option<Box<Node>>,
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

/// 语法树节点 LeftHandSideContext。对应 Java: com.alibaba.qlexpress4.aparser.QLParser 内部类 LeftHandSideContext
/// Java `LeftHandSideContext`.
#[derive(Clone, Debug)]
pub struct LeftHandSideContext {
    /// 该语法规则中的 `var_id` 子节点、终结符或节点集合。
    pub var_id: Box<Node>,
    /// `Some` when the head is a function call `f(...)`.
    pub lparen: Option<TerminalNode>,
    /// 该语法规则中的 `argument_list` 子节点、终结符或节点集合。
    pub argument_list: Option<Box<Node>>,
    /// 该语法规则中的 `path_parts` 子节点、终结符或节点集合。
    pub path_parts: Vec<Node>,
}

/// 语法树节点 AssignOperatorContext。对应 Java: com.alibaba.qlexpress4.aparser.QLParser 内部类 AssignOperatorContext
/// Java `AssignOperatorContext`.
#[derive(Clone, Debug)]
pub struct AssignOperatorContext {
    /// 该语法规则中的 `token` 子节点、终结符或节点集合。
    pub token: TerminalNode,
}

/// 语法树节点 TernaryExprContext。对应 Java: com.alibaba.qlexpress4.aparser.QLParser 内部类 TernaryExprContext
/// Java `TernaryExprContext`.
#[derive(Clone, Debug)]
pub struct TernaryExprContext {
    /// 该语法规则中的 `condition` 子节点、终结符或节点集合。
    pub condition: Box<Node>,
    /// 该语法规则中的 `question` 子节点、终结符或节点集合。
    pub question: Option<TerminalNode>,
    /// 该语法规则中的 `then_expr` 子节点、终结符或节点集合。
    pub then_expr: Option<Box<Node>>,
    /// 该语法规则中的 `else_expr` 子节点、终结符或节点集合。
    pub else_expr: Option<Box<Node>>,
}

/// 语法树节点 BaseExprContext。对应 Java: com.alibaba.qlexpress4.aparser.QLParser 内部类 BaseExprContext
/// Java `BaseExprContext`: a primary plus left-associative binary chain.
#[derive(Clone, Debug)]
pub struct BaseExprContext {
    /// 该语法规则中的 `primary` 子节点、终结符或节点集合。
    pub primary: Box<Node>,
    /// 该语法规则中的 `left_assos` 子节点、终结符或节点集合。
    pub left_assos: Vec<Node>,
}

/// 语法树节点 LeftAssoContext。对应 Java: com.alibaba.qlexpress4.aparser.QLParser 内部类 LeftAssoContext
/// Java `LeftAssoContext`: one `op right` step.
#[derive(Clone, Debug)]
pub struct LeftAssoContext {
    /// 该语法规则中的 `binaryop` 子节点、终结符或节点集合。
    pub binaryop: Box<Node>,
    /// 该语法规则中的 `right` 子节点、终结符或节点集合。
    pub right: Box<Node>,
}

/// 语法树节点 BinaryopContext。对应 Java: com.alibaba.qlexpress4.aparser.QLParser 内部类 BinaryopContext
/// Java `BinaryopContext`.
#[derive(Clone, Debug)]
pub struct BinaryopContext {
    /// 该语法规则中的 `token` 子节点、终结符或节点集合。
    pub token: TerminalNode,
}

/// 语法树节点 PrimaryContext。对应 Java: com.alibaba.qlexpress4.aparser.QLParser 内部类 PrimaryContext
/// Java `PrimaryContext`.
#[derive(Clone, Debug)]
pub struct PrimaryContext {
    /// 该语法规则中的 `prefix` 子节点、终结符或节点集合。
    pub prefix: Option<Box<Node>>,
    /// 该语法规则中的 `pathable` 子节点、终结符或节点集合。
    pub pathable: Option<Box<Node>>,
    /// 该语法规则中的 `path_parts` 子节点、终结符或节点集合。
    pub path_parts: Vec<Node>,
    /// 该语法规则中的 `suffix` 子节点、终结符或节点集合。
    pub suffix: Option<Box<Node>>,
    /// 该语法规则中的 `non_pathable` 子节点、终结符或节点集合。
    pub non_pathable: Option<Box<Node>>,
}

/// 语法树节点 PrefixExpressContext。对应 Java: com.alibaba.qlexpress4.aparser.QLParser 内部类 PrefixExpressContext
/// Java `PrefixExpressContext`.
#[derive(Clone, Debug)]
pub struct PrefixExpressContext {
    /// 该语法规则中的 `op_id` 子节点、终结符或节点集合。
    pub op_id: Box<Node>,
}

/// 语法树节点 SuffixExpressContext。对应 Java: com.alibaba.qlexpress4.aparser.QLParser 内部类 SuffixExpressContext
/// Java `SuffixExpressContext`.
#[derive(Clone, Debug)]
pub struct SuffixExpressContext {
    /// 该语法规则中的 `op_id` 子节点、终结符或节点集合。
    pub op_id: Box<Node>,
}

/// 语法树节点 ConstExprContext。对应 Java: com.alibaba.qlexpress4.aparser.QLParser 内部类 ConstExprContext
/// Java `ConstExprContext`.
#[derive(Clone, Debug)]
pub struct ConstExprContext {
    /// 该语法规则中的 `literal` 子节点、终结符或节点集合。
    pub literal: Box<Node>,
}

/// 语法树节点 CastExprContext。对应 Java: com.alibaba.qlexpress4.aparser.QLParser 内部类 CastExprContext
/// Java `CastExprContext`.
#[derive(Clone, Debug)]
pub struct CastExprContext {
    /// 该语法规则中的 `lparen` 子节点、终结符或节点集合。
    pub lparen: TerminalNode,
    /// 该语法规则中的 `decl_type` 子节点、终结符或节点集合。
    pub decl_type: Box<Node>,
    /// 该语法规则中的 `primary` 子节点、终结符或节点集合。
    pub primary: Box<Node>,
}

/// 语法树节点 GroupExprContext。对应 Java: com.alibaba.qlexpress4.aparser.QLParser 内部类 GroupExprContext
/// Java `GroupExprContext` (parenthesised expression).
#[derive(Clone, Debug)]
pub struct GroupExprContext {
    /// 该语法规则中的 `lparen` 子节点、终结符或节点集合。
    pub lparen: TerminalNode,
    /// 该语法规则中的 `expression` 子节点、终结符或节点集合。
    pub expression: Box<Node>,
}

/// 语法树节点 NewObjExprContext。对应 Java: com.alibaba.qlexpress4.aparser.QLParser 内部类 NewObjExprContext
/// Java `NewObjExprContext`.
#[derive(Clone, Debug)]
pub struct NewObjExprContext {
    /// 该语法规则中的 `new_token` 子节点、终结符或节点集合。
    pub new_token: TerminalNode,
    /// 该语法规则中的 `var_ids` 子节点、终结符或节点集合。
    pub var_ids: Vec<Node>,
    /// 该语法规则中的 `argument_list` 子节点、终结符或节点集合。
    pub argument_list: Option<Box<Node>>,
}

/// 语法树节点 NewEmptyArrExprContext。对应 Java: com.alibaba.qlexpress4.aparser.QLParser 内部类 NewEmptyArrExprContext
/// Java `NewEmptyArrExprContext` (`new int[3]`).
#[derive(Clone, Debug)]
pub struct NewEmptyArrExprContext {
    /// 该语法规则中的 `new_token` 子节点、终结符或节点集合。
    pub new_token: TerminalNode,
    /// 该语法规则中的 `decl_type_no_arr` 子节点、终结符或节点集合。
    pub decl_type_no_arr: Box<Node>,
    /// 该语法规则中的 `dim_exprs` 子节点、终结符或节点集合。
    pub dim_exprs: Box<Node>,
}

/// 语法树节点 NewInitArrExprContext。对应 Java: com.alibaba.qlexpress4.aparser.QLParser 内部类 NewInitArrExprContext
/// Java `NewInitArrExprContext` (`new int[]{1,2}`).
#[derive(Clone, Debug)]
pub struct NewInitArrExprContext {
    /// 该语法规则中的 `new_token` 子节点、终结符或节点集合。
    pub new_token: TerminalNode,
    /// 该语法规则中的 `decl_type_no_arr` 子节点、终结符或节点集合。
    pub decl_type_no_arr: Box<Node>,
    /// 该语法规则中的 `dims` 子节点、终结符或节点集合。
    pub dims: Box<Node>,
    /// 该语法规则中的 `array_initializer` 子节点、终结符或节点集合。
    pub array_initializer: Box<Node>,
}

/// 语法树节点 VarIdExprContext。对应 Java: com.alibaba.qlexpress4.aparser.QLParser 内部类 VarIdExprContext
/// Java `VarIdExprContext` (variable reference or function call head).
#[derive(Clone, Debug)]
pub struct VarIdExprContext {
    /// 该语法规则中的 `var_id` 子节点、终结符或节点集合。
    pub var_id: Box<Node>,
    /// 该语法规则中的 `lparen` 子节点、终结符或节点集合。
    pub lparen: Option<TerminalNode>,
    /// 该语法规则中的 `argument_list` 子节点、终结符或节点集合。
    pub argument_list: Option<Box<Node>>,
}

/// 语法树节点 TypeExprContext。对应 Java: com.alibaba.qlexpress4.aparser.QLParser 内部类 TypeExprContext
/// Java `TypeExprContext`（作为值使用的类型，含原语、具名类和数组类型）。
#[derive(Clone, Debug)]
pub struct TypeExprContext {
    /// 该语法规则中的 `decl_type` 子节点、终结符或节点集合。
    pub decl_type: Box<Node>,
}

/// 语法树节点 ListItemsContext。对应 Java: com.alibaba.qlexpress4.aparser.QLParser 内部类 ListItemsContext
/// Java `ListItemsContext`.
#[derive(Clone, Debug)]
pub struct ListItemsContext {
    /// 该语法规则中的 `expressions` 子节点、终结符或节点集合。
    pub expressions: Vec<Node>,
}

/// 语法树节点 ListExprContext。对应 Java: com.alibaba.qlexpress4.aparser.QLParser 内部类 ListExprContext
/// Java `ListExprContext`.
#[derive(Clone, Debug)]
pub struct ListExprContext {
    /// 该语法规则中的 `lbrack` 子节点、终结符或节点集合。
    pub lbrack: TerminalNode,
    /// 该语法规则中的 `list_items` 子节点、终结符或节点集合。
    pub list_items: Option<Box<Node>>,
}

/// 语法树节点 MapExprContext。对应 Java: com.alibaba.qlexpress4.aparser.QLParser 内部类 MapExprContext
/// Java `MapExprContext`.
#[derive(Clone, Debug)]
pub struct MapExprContext {
    /// 该语法规则中的 `lbrace` 子节点、终结符或节点集合。
    pub lbrace: TerminalNode,
    /// 该语法规则中的 `map_entries` 子节点、终结符或节点集合。
    pub map_entries: Box<Node>,
}

/// 语法树节点 BlockExprContext。对应 Java: com.alibaba.qlexpress4.aparser.QLParser 内部类 BlockExprContext
/// Java `BlockExprContext` (a block used as an expression).
#[derive(Clone, Debug)]
pub struct BlockExprContext {
    /// 该语法规则中的 `lbrace` 子节点、终结符或节点集合。
    pub lbrace: TerminalNode,
    /// 该语法规则中的 `block_statements` 子节点、终结符或节点集合。
    pub block_statements: Option<Box<Node>>,
}

/// 语法树节点 ContextSelectExprContext。对应 Java: com.alibaba.qlexpress4.aparser.QLParser 内部类 ContextSelectExprContext
/// Java `ContextSelectExprContext` (selector expression).
#[derive(Clone, Debug)]
pub struct ContextSelectExprContext {
    /// 该语法规则中的 `selector_start` 子节点、终结符或节点集合。
    pub selector_start: TerminalNode,
    /// 该语法规则中的 `selector_variable` 子节点、终结符或节点集合。
    pub selector_variable: TerminalNode,
}

/// 语法树节点 QlIfContext。对应 Java: com.alibaba.qlexpress4.aparser.QLParser 内部类 QlIfContext
/// Java `QlIfContext`.
#[derive(Clone, Debug)]
pub struct QlIfContext {
    /// 该语法规则中的 `if_token` 子节点、终结符或节点集合。
    pub if_token: TerminalNode,
    /// 该语法规则中的 `then_keyword` 子节点、终结符或节点集合。
    pub then_keyword: Option<TerminalNode>,
    /// 该语法规则中的 `condition` 子节点、终结符或节点集合。
    pub condition: Box<Node>,
    /// 该语法规则中的 `then_body` 子节点、终结符或节点集合。
    pub then_body: Box<Node>,
    /// 该语法规则中的 `else_body` 子节点、终结符或节点集合。
    pub else_body: Option<Box<Node>>,
}

/// 语法树节点 ThenBodyContext。对应 Java: com.alibaba.qlexpress4.aparser.QLParser 内部类 ThenBodyContext
/// Java `ThenBodyContext`: exactly one of the optionals is `Some`.
#[derive(Clone, Debug)]
pub struct ThenBodyContext {
    /// 该语法规则中的 `lbrace` 子节点、终结符或节点集合。
    pub lbrace: Option<TerminalNode>,
    /// 该语法规则中的 `block_statements` 子节点、终结符或节点集合。
    pub block_statements: Option<Box<Node>>,
    /// 该语法规则中的 `non_expression_statement` 子节点、终结符或节点集合。
    pub non_expression_statement: Option<Box<Node>>,
    /// 该语法规则中的 `expression` 子节点、终结符或节点集合。
    pub expression: Option<Box<Node>>,
}

/// 语法树节点 ElseBodyContext。对应 Java: com.alibaba.qlexpress4.aparser.QLParser 内部类 ElseBodyContext
/// Java `ElseBodyContext`: exactly one of the optionals is `Some`.
#[derive(Clone, Debug)]
pub struct ElseBodyContext {
    /// 该语法规则中的 `lbrace` 子节点、终结符或节点集合。
    pub lbrace: Option<TerminalNode>,
    /// 该语法规则中的 `block_statements` 子节点、终结符或节点集合。
    pub block_statements: Option<Box<Node>>,
    /// 该语法规则中的 `ql_if` 子节点、终结符或节点集合。
    pub ql_if: Option<Box<Node>>,
    /// 该语法规则中的 `non_expression_statement` 子节点、终结符或节点集合。
    pub non_expression_statement: Option<Box<Node>>,
    /// 该语法规则中的 `expression` 子节点、终结符或节点集合。
    pub expression: Option<Box<Node>>,
}

/// 语法树节点 SwitchExprContext。对应 Java: com.alibaba.qlexpress4.aparser.QLParser 内部类 SwitchExprContext
/// Java `SwitchExprContext`.
#[derive(Clone, Debug)]
pub struct SwitchExprContext {
    /// 该语法规则中的 `switch_token` 子节点、终结符或节点集合。
    pub switch_token: TerminalNode,
    /// 该语法规则中的 `expression` 子节点、终结符或节点集合。
    pub expression: Box<Node>,
    /// 该语法规则中的 `groups` 子节点、终结符或节点集合。
    pub groups: Option<Box<Node>>,
}

/// 语法树节点 SwitchCaseGroupsContext。对应 Java: com.alibaba.qlexpress4.aparser.QLParser 内部类 SwitchCaseGroupsContext
/// Java `SwitchCaseGroupsContext`.
#[derive(Clone, Debug)]
pub struct SwitchCaseGroupsContext {
    /// 该语法规则中的 `groups` 子节点、终结符或节点集合。
    pub groups: Vec<Node>,
}

/// 语法树节点 SwitchStatementGroupContext。对应 Java: com.alibaba.qlexpress4.aparser.QLParser 内部类 SwitchStatementGroupContext
/// Java `SwitchStatementGroupContext`.
#[derive(Clone, Debug)]
pub struct SwitchStatementGroupContext {
    /// 该语法规则中的 `labels` 子节点、终结符或节点集合。
    pub labels: Box<Node>,
    /// 该语法规则中的 `block_statements` 子节点、终结符或节点集合。
    pub block_statements: Option<Box<Node>>,
}

/// 语法树节点 SwitchExprGroupContext。对应 Java: com.alibaba.qlexpress4.aparser.QLParser 内部类 SwitchExprGroupContext
/// Java `SwitchExprGroupContext` (`case a -> expr`).
#[derive(Clone, Debug)]
pub struct SwitchExprGroupContext {
    /// 该语法规则中的 `label` 子节点、终结符或节点集合。
    pub label: Box<Node>,
    /// 该语法规则中的 `expression` 子节点、终结符或节点集合。
    pub expression: Box<Node>,
}

/// 语法树节点 SwitchLabelsContext。对应 Java: com.alibaba.qlexpress4.aparser.QLParser 内部类 SwitchLabelsContext
/// Java `SwitchLabelsContext`.
#[derive(Clone, Debug)]
pub struct SwitchLabelsContext {
    /// 该语法规则中的 `labels` 子节点、终结符或节点集合。
    pub labels: Vec<Node>,
}

/// 语法树节点 SwitchLabelContext。对应 Java: com.alibaba.qlexpress4.aparser.QLParser 内部类 SwitchLabelContext
/// Java `SwitchLabelContext`.
#[derive(Clone, Debug)]
pub struct SwitchLabelContext {
    /// 该语法规则中的 `case_token` 子节点、终结符或节点集合。
    pub case_token: Option<TerminalNode>,
    /// 该语法规则中的 `default_token` 子节点、终结符或节点集合。
    pub default_token: Option<TerminalNode>,
    /// 该语法规则中的 `expression` 子节点、终结符或节点集合。
    pub expression: Option<Box<Node>>,
}

/// 语法树节点 SwitchExpressionLabelContext。对应 Java: com.alibaba.qlexpress4.aparser.QLParser 内部类 SwitchExpressionLabelContext
/// Java `SwitchExpressionLabelContext`.
#[derive(Clone, Debug)]
pub struct SwitchExpressionLabelContext {
    /// 该语法规则中的 `case_token` 子节点、终结符或节点集合。
    pub case_token: Option<TerminalNode>,
    /// 该语法规则中的 `default_token` 子节点、终结符或节点集合。
    pub default_token: Option<TerminalNode>,
    /// 该语法规则中的 `expression_list` 子节点、终结符或节点集合。
    pub expression_list: Option<Box<Node>>,
    /// 该语法规则中的 `arrow` 子节点、终结符或节点集合。
    pub arrow: TerminalNode,
}

/// 语法树节点 ExpressionListContext。对应 Java: com.alibaba.qlexpress4.aparser.QLParser 内部类 ExpressionListContext
/// Java `ExpressionListContext`.
#[derive(Clone, Debug)]
pub struct ExpressionListContext {
    /// 该语法规则中的 `expressions` 子节点、终结符或节点集合。
    pub expressions: Vec<Node>,
}

/// 语法树节点 TryCatchExprContext。对应 Java: com.alibaba.qlexpress4.aparser.QLParser 内部类 TryCatchExprContext
/// Java `TryCatchExprContext`.
#[derive(Clone, Debug)]
pub struct TryCatchExprContext {
    /// 该语法规则中的 `try_token` 子节点、终结符或节点集合。
    pub try_token: TerminalNode,
    /// 该语法规则中的 `block_statements` 子节点、终结符或节点集合。
    pub block_statements: Option<Box<Node>>,
    /// 该语法规则中的 `try_catches` 子节点、终结符或节点集合。
    pub try_catches: Option<Box<Node>>,
    /// 该语法规则中的 `try_finally` 子节点、终结符或节点集合。
    pub try_finally: Option<Box<Node>>,
}

/// 语法树节点 TryCatchesContext。对应 Java: com.alibaba.qlexpress4.aparser.QLParser 内部类 TryCatchesContext
/// Java `TryCatchesContext`.
#[derive(Clone, Debug)]
pub struct TryCatchesContext {
    /// 该语法规则中的 `catches` 子节点、终结符或节点集合。
    pub catches: Vec<Node>,
}

/// 语法树节点 TryCatchContext。对应 Java: com.alibaba.qlexpress4.aparser.QLParser 内部类 TryCatchContext
/// Java `TryCatchContext` (one catch clause).
#[derive(Clone, Debug)]
pub struct TryCatchContext {
    /// 该语法规则中的 `catch_token` 子节点、终结符或节点集合。
    pub catch_token: TerminalNode,
    /// 该语法规则中的 `catch_params` 子节点、终结符或节点集合。
    pub catch_params: Box<Node>,
    /// 该语法规则中的 `block_statements` 子节点、终结符或节点集合。
    pub block_statements: Option<Box<Node>>,
}

/// 语法树节点 CatchParamsContext。对应 Java: com.alibaba.qlexpress4.aparser.QLParser 内部类 CatchParamsContext
/// Java `CatchParamsContext`.
#[derive(Clone, Debug)]
pub struct CatchParamsContext {
    /// 该语法规则中的 `decl_types` 子节点、终结符或节点集合。
    pub decl_types: Vec<Node>,
    /// 该语法规则中的 `var_id` 子节点、终结符或节点集合。
    pub var_id: Box<Node>,
}

/// 语法树节点 TryFinallyContext。对应 Java: com.alibaba.qlexpress4.aparser.QLParser 内部类 TryFinallyContext
/// Java `TryFinallyContext`.
#[derive(Clone, Debug)]
pub struct TryFinallyContext {
    /// 该语法规则中的 `finally_token` 子节点、终结符或节点集合。
    pub finally_token: TerminalNode,
    /// 该语法规则中的 `block_statements` 子节点、终结符或节点集合。
    pub block_statements: Option<Box<Node>>,
}

/// 语法树节点 MapEntriesContext。对应 Java: com.alibaba.qlexpress4.aparser.QLParser 内部类 MapEntriesContext
/// Java `MapEntriesContext`. `empty_colon` is `Some` for the empty-map
/// literal `{:}`.
#[derive(Clone, Debug)]
pub struct MapEntriesContext {
    /// 该语法规则中的 `empty_colon` 子节点、终结符或节点集合。
    pub empty_colon: Option<TerminalNode>,
    /// 该语法规则中的 `entries` 子节点、终结符或节点集合。
    pub entries: Vec<Node>,
}

/// 语法树节点 MapEntryContext。对应 Java: com.alibaba.qlexpress4.aparser.QLParser 内部类 MapEntryContext
/// Java `MapEntryContext`.
#[derive(Clone, Debug)]
pub struct MapEntryContext {
    /// 该语法规则中的 `map_key` 子节点、终结符或节点集合。
    pub map_key: Box<Node>,
    /// 该语法规则中的 `map_value` 子节点、终结符或节点集合。
    pub map_value: Box<Node>,
}

/// 语法树节点 ClsValueContext。对应 Java: com.alibaba.qlexpress4.aparser.QLParser 内部类 ClsValueContext
/// Java `ClsValueContext` (map value of the special `'@class'` key).
#[derive(Clone, Debug)]
pub struct ClsValueContext {
    /// 该语法规则中的 `quote` 子节点、终结符或节点集合。
    pub quote: TerminalNode,
}

/// 语法树节点 EValueContext。对应 Java: com.alibaba.qlexpress4.aparser.QLParser 内部类 EValueContext
/// Java `EValueContext` (ordinary map value).
#[derive(Clone, Debug)]
pub struct EValueContext {
    /// 该语法规则中的 `expression` 子节点、终结符或节点集合。
    pub expression: Box<Node>,
}

/// 语法树节点 IdKeyContext。对应 Java: com.alibaba.qlexpress4.aparser.QLParser 内部类 IdKeyContext
/// Java `IdKeyContext`.
#[derive(Clone, Debug)]
pub struct IdKeyContext {
    /// 该语法规则中的 `token` 子节点、终结符或节点集合。
    pub token: TerminalNode,
}

/// 语法树节点 StringKeyContext。对应 Java: com.alibaba.qlexpress4.aparser.QLParser 内部类 StringKeyContext
/// Java `StringKeyContext` (double-quoted map key).
#[derive(Clone, Debug)]
pub struct StringKeyContext {
    /// 该语法规则中的 `double_quote_string` 子节点、终结符或节点集合。
    pub double_quote_string: Box<Node>,
}

/// 语法树节点 QuoteStringKeyContext。对应 Java: com.alibaba.qlexpress4.aparser.QLParser 内部类 QuoteStringKeyContext
/// Java `QuoteStringKeyContext` (single-quoted map key).
#[derive(Clone, Debug)]
pub struct QuoteStringKeyContext {
    /// 该语法规则中的 `token` 子节点、终结符或节点集合。
    pub token: TerminalNode,
}

/// `ChainKind` 枚举的 Rust 实现，保留对应对象的领域职责与公开契约。
/// 对应或承接 Java 源文件：`com/alibaba/qlexpress4/aparser/SyntaxTreeFactory.java`；具体对象路径见 `docs/对象级对照表.md`。
/// How a path part is chained, mirroring the Java `Optional*`/`Spread*`
/// subclasses of `MethodInvokeContext`/`FieldAccessContext`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
/// 对应 Java: com.alibaba.qlexpress4.aparser.SyntaxTreeFactory。
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
    /// 该语法规则中的 `var_id` 子节点、终结符或节点集合。
    pub var_id: Box<Node>,
    /// 该语法规则中的 `argument_list` 子节点、终结符或节点集合。
    pub argument_list: Option<Box<Node>>,
    /// 该语法规则中的 `chain` 子节点、终结符或节点集合。
    pub chain: ChainKind,
}

/// 语法树节点 FieldAccessContext。对应 Java: com.alibaba.qlexpress4.aparser.QLParser 内部类 FieldAccessContext
/// Java `FieldAccessContext` (plus the optional/spread subclasses).
#[derive(Clone, Debug)]
pub struct FieldAccessContext {
    /// The `.` / `?.` / `*.` token.
    pub dot: TerminalNode,
    /// 该语法规则中的 `field_id` 子节点、终结符或节点集合。
    pub field_id: Box<Node>,
    /// 该语法规则中的 `chain` 子节点、终结符或节点集合。
    pub chain: ChainKind,
}

/// 语法树节点 MethodAccessContext。对应 Java: com.alibaba.qlexpress4.aparser.QLParser 内部类 MethodAccessContext
/// Java `MethodAccessContext` (`Cls::method`).
#[derive(Clone, Debug)]
pub struct MethodAccessContext {
    /// 该语法规则中的 `dcolon` 子节点、终结符或节点集合。
    pub dcolon: TerminalNode,
    /// 该语法规则中的 `var_id` 子节点、终结符或节点集合。
    pub var_id: Box<Node>,
}

/// 语法树节点 IndexExprContext。对应 Java: com.alibaba.qlexpress4.aparser.QLParser 内部类 IndexExprContext
/// Java `IndexExprContext` (`a[i]` / `a[i:j]`); `None` index for `a[]`.
#[derive(Clone, Debug)]
pub struct IndexExprContext {
    /// 该语法规则中的 `lbrack` 子节点、终结符或节点集合。
    pub lbrack: TerminalNode,
    /// 该语法规则中的 `index_value_expr` 子节点、终结符或节点集合。
    pub index_value_expr: Option<Box<Node>>,
    /// 右方括号；Java `ParserRuleContext#getStop()` 在 `a[]` 上返回此 token。
    pub rbrack: TerminalNode,
}

/// 语法树节点 CustomPathContext。对应 Java: com.alibaba.qlexpress4.aparser.QLParser 内部类 CustomPathContext
/// Java `CustomPathContext` (custom operator path, e.g. `a %% 'path'`).
#[derive(Clone, Debug)]
pub struct CustomPathContext {
    /// 该语法规则中的 `op_id` 子节点、终结符或节点集合。
    pub op_id: Box<Node>,
    /// 该语法规则中的 `var_id` 子节点、终结符或节点集合。
    pub var_id: Option<Box<Node>>,
    /// 该语法规则中的 `quote` 子节点、终结符或节点集合。
    pub quote: Option<TerminalNode>,
    /// 该语法规则中的 `path_text` 子节点、终结符或节点集合。
    pub path_text: String,
}

/// 语法树节点 FieldIdContext。对应 Java: com.alibaba.qlexpress4.aparser.QLParser 内部类 FieldIdContext
/// Java `FieldIdContext`.
#[derive(Clone, Debug)]
pub struct FieldIdContext {
    /// 该语法规则中的 `token` 子节点、终结符或节点集合。
    pub token: Option<TerminalNode>,
    /// 该语法规则中的 `quote` 子节点、终结符或节点集合。
    pub quote: Option<TerminalNode>,
}

/// 语法树节点 SingleIndexContext。对应 Java: com.alibaba.qlexpress4.aparser.QLParser 内部类 SingleIndexContext
/// Java `SingleIndexContext`.
#[derive(Clone, Debug)]
pub struct SingleIndexContext {
    /// 该语法规则中的 `expression` 子节点、终结符或节点集合。
    pub expression: Box<Node>,
}

/// 语法树节点 SliceIndexContext。对应 Java: com.alibaba.qlexpress4.aparser.QLParser 内部类 SliceIndexContext
/// Java `SliceIndexContext` (`a[start:end]`).
#[derive(Clone, Debug)]
pub struct SliceIndexContext {
    /// 该语法规则中的 `start` 子节点、终结符或节点集合。
    pub start: Option<Box<Node>>,
    /// 该语法规则中的 `colon` 子节点、终结符或节点集合。
    pub colon: TerminalNode,
    /// 该语法规则中的 `end` 子节点、终结符或节点集合。
    pub end: Option<Box<Node>>,
}

/// 语法树节点 ArgumentListContext。对应 Java: com.alibaba.qlexpress4.aparser.QLParser 内部类 ArgumentListContext
/// Java `ArgumentListContext`.
#[derive(Clone, Debug)]
pub struct ArgumentListContext {
    /// 该语法规则中的 `expressions` 子节点、终结符或节点集合。
    pub expressions: Vec<Node>,
}

/// 语法树节点 LiteralContext。对应 Java: com.alibaba.qlexpress4.aparser.QLParser 内部类 LiteralContext
/// Java `LiteralContext`: exactly one of the fields is `Some`.
#[derive(Clone, Debug)]
pub struct LiteralContext {
    /// Number / single-quoted string / `null` token.
    pub token: Option<TerminalNode>,
    /// 该语法规则中的 `boolen` 子节点、终结符或节点集合。
    pub boolen: Option<Box<Node>>,
    /// 该语法规则中的 `double_quote_string` 子节点、终结符或节点集合。
    pub double_quote_string: Option<Box<Node>>,
}

/// `DyStrPart` 枚举的 Rust 实现，保留对应对象的领域职责与公开契约。
/// 对应或承接 Java 源文件：`com/alibaba/qlexpress4/aparser/SyntaxTreeFactory.java`；具体对象路径见 `docs/对象级对照表.md`。
/// One piece of a double-quoted string: literal text or an interpolation.
#[derive(Clone, Debug)]
/// 对应 Java: com.alibaba.qlexpress4.aparser.SyntaxTreeFactory。
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
    /// 该语法规则中的 `open_quote` 子节点、终结符或节点集合。
    pub open_quote: TerminalNode,
    /// 该语法规则中的 `static_characters` 子节点、终结符或节点集合。
    pub static_characters: Option<TerminalNode>,
    /// 该语法规则中的 `parts` 子节点、终结符或节点集合。
    pub parts: Vec<DyStrPart>,
    /// 该语法规则中的 `close_quote` 子节点、终结符或节点集合。
    pub close_quote: TerminalNode,
}

/// 语法树节点 StringExpressionContext。对应 Java: com.alibaba.qlexpress4.aparser.QLParser 内部类 StringExpressionContext
/// Java `StringExpressionContext` (`${expr}` or `${#var}` inside a string).
#[derive(Clone, Debug)]
pub struct StringExpressionContext {
    /// 该语法规则中的 `start` 子节点、终结符或节点集合。
    pub start: TerminalNode,
    /// 该语法规则中的 `selector_variable` 子节点、终结符或节点集合。
    pub selector_variable: Option<TerminalNode>,
    /// 该语法规则中的 `expression` 子节点、终结符或节点集合。
    pub expression: Option<Box<Node>>,
}

/// 语法树节点 BoolenLiteralContext。对应 Java: com.alibaba.qlexpress4.aparser.QLParser 内部类 BoolenLiteralContext
/// Java `BoolenLiteralContext` (sic, Java spelling).
#[derive(Clone, Debug)]
pub struct BoolenLiteralContext {
    /// 该语法规则中的 `token` 子节点、终结符或节点集合。
    pub token: TerminalNode,
}

/// 语法树节点 LambdaExprContext。对应 Java: com.alibaba.qlexpress4.aparser.QLParser 内部类 LambdaExprContext
/// Java `LambdaExprContext`.
#[derive(Clone, Debug)]
pub struct LambdaExprContext {
    /// 该语法规则中的 `lambda_parameters` 子节点、终结符或节点集合。
    pub lambda_parameters: Box<Node>,
    /// 该语法规则中的 `arrow` 子节点、终结符或节点集合。
    pub arrow: TerminalNode,
    /// 该语法规则中的 `lbrace` 子节点、终结符或节点集合。
    pub lbrace: Option<TerminalNode>,
    /// 该语法规则中的 `block_statements` 子节点、终结符或节点集合。
    pub block_statements: Option<Box<Node>>,
    /// 该语法规则中的 `expression` 子节点、终结符或节点集合。
    pub expression: Option<Box<Node>>,
}

/// 语法树节点 LambdaParametersContext。对应 Java: com.alibaba.qlexpress4.aparser.QLParser 内部类 LambdaParametersContext
/// Java `LambdaParametersContext`: single id (`x -> ..`) or parameter list.
#[derive(Clone, Debug)]
pub struct LambdaParametersContext {
    /// 该语法规则中的 `var_id` 子节点、终结符或节点集合。
    pub var_id: Option<Box<Node>>,
    /// 该语法规则中的 `params` 子节点、终结符或节点集合。
    pub params: Option<Box<Node>>,
}

/// 语法树节点 FormalOrInferredParameterListContext。对应 Java: com.alibaba.qlexpress4.aparser.QLParser 内部类 FormalOrInferredParameterListContext
/// Java `FormalOrInferredParameterListContext`.
#[derive(Clone, Debug)]
pub struct FormalOrInferredParameterListContext {
    /// 该语法规则中的 `params` 子节点、终结符或节点集合。
    pub params: Vec<Node>,
}

/// 语法树节点 FormalOrInferredParameterContext。对应 Java: com.alibaba.qlexpress4.aparser.QLParser 内部类 FormalOrInferredParameterContext
/// Java `FormalOrInferredParameterContext`.
#[derive(Clone, Debug)]
pub struct FormalOrInferredParameterContext {
    /// 该语法规则中的 `decl_type` 子节点、终结符或节点集合。
    pub decl_type: Option<Box<Node>>,
    /// 该语法规则中的 `var_id` 子节点、终结符或节点集合。
    pub var_id: Box<Node>,
}

/// 语法树节点 ImportClsContext。对应 Java: com.alibaba.qlexpress4.aparser.QLParser 内部类 ImportClsContext
/// Java `ImportClsContext` (`import a.b.C;`).
#[derive(Clone, Debug)]
pub struct ImportClsContext {
    /// 该语法规则中的 `import_token` 子节点、终结符或节点集合。
    pub import_token: TerminalNode,
    /// 该语法规则中的 `var_ids` 子节点、终结符或节点集合。
    pub var_ids: Vec<Node>,
}

/// 语法树节点 ImportPackContext。对应 Java: com.alibaba.qlexpress4.aparser.QLParser 内部类 ImportPackContext
/// Java `ImportPackContext` (`import a.b.*;` / `import a.b.*;`).
#[derive(Clone, Debug)]
pub struct ImportPackContext {
    /// 该语法规则中的 `import_token` 子节点、终结符或节点集合。
    pub import_token: TerminalNode,
    /// 该语法规则中的 `var_ids` 子节点、终结符或节点集合。
    pub var_ids: Vec<Node>,
}

/// 语法树节点 OpIdContext。对应 Java: com.alibaba.qlexpress4.aparser.QLParser 内部类 OpIdContext
/// Java `OpIdContext` (prefix/suffix/custom operator token).
#[derive(Clone, Debug)]
pub struct OpIdContext {
    /// 该语法规则中的 `token` 子节点、终结符或节点集合。
    pub token: TerminalNode,
}

/// 语法树节点 VarIdContext。对应 Java: com.alibaba.qlexpress4.aparser.QLParser 内部类 VarIdContext
/// Java `VarIdContext` (an identifier token).
#[derive(Clone, Debug)]
pub struct VarIdContext {
    /// 该语法规则中的 `token` 子节点、终结符或节点集合。
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
        vec![n(&self.decl_type)]
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
