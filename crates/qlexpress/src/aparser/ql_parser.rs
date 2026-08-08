//! Recursive descent parser, a line-faithful port of Java `QLParser` plus
//! the `SyntaxTreeFactory.buildTree` entry point.
//!
//! Error handling: Java uses two control-flow exceptions —
//! `QLParseBacktrack` (caught only by `tryParseLocalVariableDeclaration`)
//! and `QLSyntaxException` (fatal, reported to the user). Rust models them
//! as the two variants of [`ParseFail`]; speculative helpers
//! (`isCastStart`, `isMapExprAhead`, `tryParseSwitchExpressionLabel`,
//! `parseFormalOrInferredParameter`) catch *both*, exactly like Java's
//! `catch (RuntimeException e)`.

use super::interpolation_mode::InterpolationMode;
use super::parser_operator_manager::{OpType, ParserOperatorManager};
use super::qlexer;
use super::syntax_tree_factory::{
    ArgumentListContext, ArrayInitializerContext, AssignOperatorContext, BaseExprContext,
    BinaryopContext, BlockExprContext, BlockStatementsContext, BoolenLiteralContext,
    BreakContinueStatementContext, CastExprContext, CatchParamsContext, ChainKind, ClsTypeContext,
    ClsValueContext, ConstExprContext, ContextSelectExprContext, CustomPathContext,
    DeclTypeContext, DeclTypeNoArrContext, DimExprsContext, DimsContext,
    DoubleQuoteStringLiteralContext, DyStrPart, EValueContext, ElseBodyContext,
    EmptyStatementContext, ExpressionContext, ExpressionListContext, ExpressionStatementContext,
    FieldAccessContext, FieldIdContext, ForEachStatementContext, ForInitContext,
    FormalOrInferredParameterContext, FormalOrInferredParameterListContext,
    FunctionStatementContext, GroupExprContext, IdKeyContext, ImportClsContext, ImportPackContext,
    IndexExprContext, LambdaExprContext, LambdaParametersContext, LeftAssoContext,
    LeftHandSideContext, ListExprContext, ListItemsContext, LiteralContext,
    LocalVariableDeclarationContext, LocalVariableDeclarationStatementContext,
    MacroStatementContext, MapEntriesContext, MapEntryContext, MapExprContext, MethodAccessContext,
    MethodInvokeContext, NewEmptyArrExprContext, NewInitArrExprContext, NewObjExprContext, Node,
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
use super::token::{self, Token};
use crate::exception::error_codes;
use crate::exception::ql_syntax_exception::QLSyntaxException;
use crate::exception::QLException;
use crate::ql_precedences;

// Token type constants as `i32`, mirroring the `public static final int`
// block at the top of Java `QLParser` (values come from `QLexer`).
macro_rules! token_consts {
    ($($name:ident),* $(,)?) => {
        $(#[allow(dead_code)] const $name: i32 = token::$name as i32;)*
    };
}

token_consts!(
    FOR,
    IF,
    ELSE,
    WHILE,
    BREAK,
    CONTINUE,
    RETURN,
    FUNCTION,
    MACRO,
    IMPORT,
    STATIC,
    NEW,
    SWITCH,
    CASE,
    DEFAULT,
    BYTE,
    SHORT,
    INT,
    LONG,
    FLOAT,
    DOUBLE,
    CHAR,
    BOOL,
    NULL,
    TRUE,
    FALSE,
    EXTENDS,
    SUPER,
    TRY,
    CATCH,
    FINALLY,
    THROW,
    THEN,
    CLASS,
    THIS,
    QUOTE_STRING_LITERAL,
    INTEGER_LITERAL,
    FLOATING_POINT_LITERAL,
    INTEGER_OR_FLOATING_LITERAL,
    LPAREN,
    RPAREN,
    LBRACE,
    RBRACE,
    LBRACK,
    RBRACK,
    DOT,
    ARROW,
    SEMI,
    COMMA,
    QUESTION,
    COLON,
    DCOLON,
    GT,
    LT,
    EQ,
    NOEQ,
    RIGHSHIFT_ASSGIN,
    RIGHSHIFT,
    OPTIONAL_CHAINING,
    SPREAD_CHAINING,
    URSHIFT_ASSGIN,
    URSHIFT,
    LSHIFT_ASSGIN,
    LEFTSHIFT,
    GE,
    LE,
    DOTMUL,
    CARET,
    ADD_ASSIGN,
    SUB_ASSIGN,
    AND_ASSIGN,
    OR_ASSIGN,
    MUL_ASSIGN,
    MOD_ASSIGN,
    DIV_ASSIGN,
    XOR_ASSIGN,
    BANG,
    TILDE,
    ADD,
    SUB,
    MUL,
    DIV,
    BIT_AND,
    BIT_OR,
    MOD,
    INC,
    DEC,
    NEWLINE,
    OPID,
    SELECTOR_START,
    ID,
    DOUBLE_QUOTE,
    STATIC_STRING_CHARACTERS,
    DY_STR_EXPR_START,
    DY_STR_TEXT,
    SELECTOR_VARIABLE_VANME
);

const EOF: i32 = token::EOF;

pub use super::parse_fail::ParseFail;

impl From<ParseFail> for QLSyntaxException {
    fn from(fail: ParseFail) -> Self {
        match fail {
            ParseFail::Syntax(e) => e,
            // A backtrack request escaping the parser means the caller did
            // not guard a speculative path; treat as an internal error.
            ParseFail::Backtrack => QLException::report_scanner_err(
                "",
                0,
                1,
                1,
                "",
                error_codes::SYNTAX_ERROR,
                "internal parser backtrack",
            ),
        }
    }
}

type PResult<T> = Result<T, ParseFail>;

/// 构建或解析 tree。
/// 参数：`script`、`operator_manager`、`print_tree`、`printer`、`interpolation_mode`、`selector_start`、`selector_end`、`strict_new_lines`；返回：`Result<Node, QLSyntaxException>`。
/// 对应或承接 Java 源文件：`com/alibaba/qlexpress4/aparser/SyntaxTreeFactory.java`，方法 `buildTree`；Rust 侧按所有权与 `Result` 语义适配。
/// Java `SyntaxTreeFactory.buildTree`.
///
/// Tokenizes `script` with [`qlexer::tokenize`] and parses it into a
/// [`Node::Program`] syntax tree. When `print_tree` is set, `printer`
/// receives the token stream and Java `RuleContext#toStringTree()` output.
#[allow(clippy::too_many_arguments)]
/// 对应 Java：`com.alibaba.qlexpress4.aparser.SyntaxTreeFactory#buildTree`。
pub fn build_tree(
    script: &str,
    operator_manager: Option<&dyn ParserOperatorManager>,
    print_tree: bool,
    printer: impl FnMut(String),
    interpolation_mode: InterpolationMode,
    selector_start: &str,
    selector_end: &str,
    strict_new_lines: bool,
) -> Result<Node, QLSyntaxException> {
    let tokens = qlexer::tokenize(
        script,
        operator_manager,
        interpolation_mode,
        selector_start,
        selector_end,
        strict_new_lines,
    )?;
    build_tree_from_tokens(
        script,
        &tokens,
        operator_manager,
        print_tree,
        printer,
        strict_new_lines,
    )
}

/// 使用已经过预算限制的 Token 流构建 AST，避免安全入口再次无界词法分配。
/// 对应 Java：`SyntaxTreeFactory#buildTree`（Rust 安全增强的预词法 Token 入口）。
pub fn build_tree_from_tokens(
    script: &str,
    tokens: &[Token],
    operator_manager: Option<&dyn ParserOperatorManager>,
    print_tree: bool,
    mut printer: impl FnMut(String),
    strict_new_lines: bool,
) -> Result<Node, QLSyntaxException> {
    let mut parser = QLParser::new(script, tokens, operator_manager, strict_new_lines);
    let program = parser.program()?;
    if print_tree {
        let token_texts: Vec<&str> = tokens.iter().map(Token::text).collect();
        printer(token_texts.join(" | "));
        printer(program.to_string_tree());
    }
    Ok(program)
}

/// 消费 Token 流并按 QLExpress4 语法生成 AST 的递归下降解析器。
/// 对应或承接 Java 源文件：`com/alibaba/qlexpress4/aparser/QLParser.java`；具体对象路径见 `docs/对象级对照表.md`。
/// Java `QLParser`.
/// 对应 Java: com.alibaba.qlexpress4.aparser.QLParser。
pub struct QLParser<'a> {
    script: &'a str,
    tokens: &'a [Token],
    operator_manager: Option<&'a dyn ParserOperatorManager>,
    strict_new_lines: bool,
    p: usize,
}

include!("ql_parser/statements.rs");
include!("ql_parser/expressions.rs");
include!("ql_parser/control_flow.rs");
include!("ql_parser/paths_and_types.rs");
include!("ql_parser/lookahead.rs");

// ---------------------------------------------------------------------------
// Free classification helpers (Java private static-ish predicates)
// ---------------------------------------------------------------------------

fn token_text(tok: &Token) -> String {
    if tok.token_type() == EOF {
        "<EOF>".to_string()
    } else {
        tok.text().to_string()
    }
}

fn is_literal_start(ty: i32) -> bool {
    ty == INTEGER_LITERAL
        || ty == FLOATING_POINT_LITERAL
        || ty == INTEGER_OR_FLOATING_LITERAL
        || ty == TRUE
        || ty == FALSE
        || ty == QUOTE_STRING_LITERAL
        || ty == DOUBLE_QUOTE
        || ty == NULL
}

fn is_assign_operator(ty: i32) -> bool {
    ty == EQ
        || ty == RIGHSHIFT_ASSGIN
        || ty == URSHIFT_ASSGIN
        || ty == LSHIFT_ASSGIN
        || ty == ADD_ASSIGN
        || ty == SUB_ASSIGN
        || ty == AND_ASSIGN
        || ty == OR_ASSIGN
        || ty == MUL_ASSIGN
        || ty == MOD_ASSIGN
        || ty == DIV_ASSIGN
        || ty == XOR_ASSIGN
}

fn is_primitive_type(ty: i32) -> bool {
    ty == BYTE
        || ty == SHORT
        || ty == INT
        || ty == LONG
        || ty == FLOAT
        || ty == DOUBLE
        || ty == BOOL
        || ty == CHAR
}

fn is_var_id_token(ty: i32) -> bool {
    ty == ID || ty == FUNCTION || ty == CASE || ty == DEFAULT || ty == SWITCH
}

fn is_id_map_key(ty: i32) -> bool {
    is_var_id_token(ty)
        || ty == FOR
        || ty == IF
        || ty == ELSE
        || ty == WHILE
        || ty == BREAK
        || ty == CONTINUE
        || ty == RETURN
        || ty == MACRO
        || ty == IMPORT
        || ty == STATIC
        || ty == NEW
        || ty == BYTE
        || ty == SHORT
        || ty == INT
        || ty == LONG
        || ty == FLOAT
        || ty == DOUBLE
        || ty == CHAR
        || ty == BOOL
        || ty == NULL
        || ty == TRUE
        || ty == FALSE
        || ty == EXTENDS
        || ty == SUPER
        || ty == TRY
        || ty == CATCH
        || ty == FINALLY
        || ty == THROW
        || ty == CLASS
        || ty == THIS
}

fn is_op_id_token(ty: i32) -> bool {
    ty == BANG
        || ty == TILDE
        || ty == ADD
        || ty == SUB
        || ty == INC
        || ty == DEC
        || ty == DOTMUL
        || is_assign_operator(ty)
        || ty == OPID
}

#[cfg(test)]
#[path = "ql_parser_tests.rs"]
mod tests;
