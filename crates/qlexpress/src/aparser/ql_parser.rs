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

impl<'a> QLParser<'a> {
    /// 创建对象实例。
    /// 参数：`script`、`tokens`、`operator_manager`、`strict_new_lines`；返回：`Self`。
    /// 对应或承接 Java 源文件：`com/alibaba/qlexpress4/aparser/QLParser.java`，构造器 `<init>`；Rust 侧按所有权与 `Result` 语义适配。
    /// Java `new QLParser(script, tokens, operatorManager, strictNewLines)`.
    /// 对应 Java: com.alibaba.qlexpress4.aparser.QLParser#new。
    pub fn new(
        script: &'a str,
        tokens: &'a [Token],
        operator_manager: Option<&'a dyn ParserOperatorManager>,
        strict_new_lines: bool,
    ) -> Self {
        QLParser {
            script,
            tokens,
            operator_manager,
            strict_new_lines,
            p: 0,
        }
    }

    /// 解析完整脚本并返回程序根节点。
    /// 无显式参数；返回：`Result<Node, QLSyntaxException>`。
    /// 对应或承接 Java 源文件：`com/alibaba/qlexpress4/aparser/QLParser.java`，方法 `program`；Rust 侧按所有权与 `Result` 语义适配。
    /// Java `program()`.
    /// 对应 Java: com.alibaba.qlexpress4.aparser.QLParser#program。
    pub fn program(&mut self) -> Result<Node, QLSyntaxException> {
        self.program_internal().map_err(QLSyntaxException::from)
    }

    fn program_internal(&mut self) -> PResult<Node> {
        let mut ctx = ProgramContext {
            imports: Vec::new(),
            block_statements: None,
        };
        self.skip_newlines();
        while self.la(IMPORT) {
            let import = self.parse_import_declaration()?;
            ctx.imports.push(import);
            self.skip_newlines();
        }
        if !self.la(EOF) {
            ctx.block_statements = Some(Box::new(self.parse_block_statements_until(EOF)?));
        }
        self.expect(EOF, "<EOF>")?;
        Ok(Node::Program(ctx))
    }

    // ------------------------------------------------------------------
    // Token stream plumbing (Java lt/token/la/consume/expect/syntax).
    // ------------------------------------------------------------------

    fn lt(&self) -> &'a Token {
        self.token(self.p)
    }

    fn token(&self, index: usize) -> &'a Token {
        if index >= self.tokens.len() {
            return &self.tokens[self.tokens.len() - 1];
        }
        &self.tokens[index]
    }

    fn la(&self, token_type: i32) -> bool {
        self.lt().token_type() == token_type
    }

    fn la_at(&self, offset: usize, token_type: i32) -> bool {
        self.token(self.p + offset).token_type() == token_type
    }

    fn consume(&mut self) -> &'a Token {
        let tok = self.token(self.p);
        self.p += 1;
        tok
    }

    fn consume_node(&mut self) -> TerminalNode {
        TerminalNode::new(self.consume().clone())
    }

    fn expect(&mut self, token_type: i32, display: &str) -> PResult<TerminalNode> {
        if !self.la(token_type) {
            let tok = self.lt();
            return Err(self.syntax(
                tok,
                format!(
                    "mismatched input '{}' expecting {}",
                    token_text(tok),
                    display
                ),
            ));
        }
        Ok(self.consume_node())
    }

    /// Java `syntax(token, reason)` — fatal syntax error.
    fn syntax(&self, tok: &Token, reason: String) -> ParseFail {
        let report_script = if tok.token_type() == EOF {
            format!("{}<EOF>", self.script)
        } else {
            self.script.to_string()
        };
        ParseFail::Syntax(QLException::report_scanner_err(
            &report_script,
            tok.start_index(),
            tok.line(),
            tok.char_position_in_line() + 1,
            &token_text(tok),
            error_codes::SYNTAX_ERROR,
            &reason,
        ))
    }

    fn backtrack<T>() -> PResult<T> {
        Err(ParseFail::Backtrack)
    }

    // ------------------------------------------------------------------
    // Statements
    // ------------------------------------------------------------------

    fn parse_block_statements_until(&mut self, end_type: i32) -> PResult<Node> {
        let mut ctx = BlockStatementsContext {
            statements: Vec::new(),
        };
        while !self.la(end_type) && !self.la(EOF) {
            if end_type == RBRACE && self.la(RBRACE) {
                break;
            }
            let statement = self.parse_block_statement()?;
            ctx.statements.push(statement);
        }
        Ok(Node::BlockStatements(ctx))
    }

    fn parse_block_statement(&mut self) -> PResult<Node> {
        if self.la(NEWLINE) || self.la(SEMI) {
            return Ok(Node::EmptyStatement(EmptyStatementContext {
                token: self.consume_node(),
            }));
        }
        if self.la(IMPORT) {
            let tok = self.lt();
            return Err(self.syntax(
                tok,
                "Import statement is not at the beginning of the file.".to_string(),
            ));
        }
        if self.la(THROW) {
            let throw_token = self.consume_node();
            let expression = self.parse_expression()?;
            self.consume_next_statement()?;
            return Ok(Node::ThrowStatement(ThrowStatementContext {
                throw_token,
                expression: Box::new(expression),
            }));
        }
        if self.la(WHILE) {
            return self.parse_while_statement();
        }
        if self.la(FOR) {
            return self.parse_for_statement();
        }
        if self.la(FUNCTION) {
            return self.parse_function_statement();
        }
        if self.la(MACRO) {
            return self.parse_macro_statement();
        }
        if let Some(local) = self.try_parse_local_variable_declaration()? {
            let semi = self.expect(SEMI, "';'")?;
            return Ok(Node::LocalVariableDeclarationStatement(
                LocalVariableDeclarationStatementContext {
                    local_variable_declaration: Box::new(local),
                    semi,
                },
            ));
        }
        if self.la(BREAK) || self.la(CONTINUE) {
            let token = self.consume_node();
            self.consume_next_statement()?;
            return Ok(Node::BreakContinueStatement(
                BreakContinueStatementContext { token },
            ));
        }
        if self.la(RETURN) {
            let return_token = self.consume_node();
            let expression = if !self.is_next_statement_start() {
                Some(Box::new(self.parse_expression()?))
            } else {
                None
            };
            self.consume_next_statement()?;
            return Ok(Node::ReturnStatement(ReturnStatementContext {
                return_token,
                expression,
            }));
        }
        let expression = self.parse_expression()?;
        self.consume_next_statement()?;
        Ok(Node::ExpressionStatement(ExpressionStatementContext {
            expression: Box::new(expression),
        }))
    }

    fn parse_while_statement(&mut self) -> PResult<Node> {
        let while_token = self.consume_node();
        let lparen = self.expect(LPAREN, "'('")?;
        self.skip_newlines();
        let expression = self.parse_expression()?;
        self.skip_newlines();
        let rparen = self.expect(RPAREN, "')'")?;
        let (lbrace, block_statements, rbrace) = self.parse_braced_block()?;
        Ok(Node::WhileStatement(WhileStatementContext {
            while_token,
            lparen,
            expression: Box::new(expression),
            rparen,
            lbrace,
            block_statements: block_statements.map(Box::new),
            rbrace,
        }))
    }

    /// Java `parseBracedBlock`: `{ ... }`, `Ok(None)` for an empty block.
    fn parse_braced_block(&mut self) -> PResult<(TerminalNode, Option<Node>, TerminalNode)> {
        let lbrace = self.expect(LBRACE, "'{'")?;
        self.skip_newlines();
        let mut block = None;
        if !self.la(RBRACE) {
            block = Some(self.parse_block_statements_until(RBRACE)?);
        }
        self.skip_newlines();
        let rbrace = self.expect(RBRACE, "'}'")?;
        Ok((lbrace, block, rbrace))
    }

    fn parse_for_statement(&mut self) -> PResult<Node> {
        let save = self.p;
        self.consume();
        self.expect(LPAREN, "'('")?;
        let for_each = self.scan_for_each_header();
        self.p = save;
        if for_each {
            return self.parse_for_each_statement();
        }
        self.parse_traditional_for_statement()
    }

    fn scan_for_each_header(&self) -> bool {
        let mut depth = 0i32;
        let mut i = self.p;
        while i < self.tokens.len() {
            let ty = self.tokens[i].token_type();
            if ty == LPAREN || ty == LBRACK || ty == LBRACE {
                depth += 1;
            } else if ty == RPAREN || ty == RBRACK || ty == RBRACE {
                depth -= 1;
                if depth <= 0 && ty == RPAREN {
                    return false;
                }
            } else if depth == 0 && ty == SEMI {
                return false;
            } else if depth == 0 && ty == COLON {
                return true;
            }
            i += 1;
        }
        false
    }

    fn parse_traditional_for_statement(&mut self) -> PResult<Node> {
        let for_token = self.consume_node();
        let lparen = self.expect(LPAREN, "'('")?;
        self.skip_newlines();
        let for_init = self.parse_for_init()?;
        self.skip_newlines();
        let for_condition = if !self.la(SEMI) {
            Some(Box::new(self.parse_expression()?))
        } else {
            None
        };
        let condition_semi = self.expect(SEMI, "';'")?;
        self.skip_newlines();
        let for_update = if !self.la(RPAREN) {
            Some(Box::new(self.parse_expression()?))
        } else {
            None
        };
        self.skip_newlines();
        let rparen = self.expect(RPAREN, "')'")?;
        let (lbrace, block_statements, rbrace) = self.parse_braced_block()?;
        Ok(Node::TraditionalForStatement(
            TraditionalForStatementContext {
                for_token,
                lparen,
                for_init: Box::new(for_init),
                for_condition,
                condition_semi,
                for_update,
                rparen,
                lbrace,
                block_statements: block_statements.map(Box::new),
                rbrace,
            },
        ))
    }

    fn parse_for_init(&mut self) -> PResult<Node> {
        if self.la(SEMI) {
            let semi = self.consume_node();
            return Ok(Node::ForInit(ForInitContext {
                local_variable_declaration: None,
                expression: None,
                semi,
            }));
        }
        if let Some(local) = self.try_parse_local_variable_declaration()? {
            let semi = self.expect(SEMI, "';'")?;
            return Ok(Node::ForInit(ForInitContext {
                local_variable_declaration: Some(Box::new(local)),
                expression: None,
                semi,
            }));
        }
        let expression = self.parse_expression()?;
        let semi = self.expect(SEMI, "';'")?;
        Ok(Node::ForInit(ForInitContext {
            local_variable_declaration: None,
            expression: Some(Box::new(expression)),
            semi,
        }))
    }

    fn parse_for_each_statement(&mut self) -> PResult<Node> {
        let for_token = self.consume_node();
        let lparen = self.expect(LPAREN, "'('")?;
        self.skip_newlines();
        let decl_type = if !self.single_for_each_var_before_colon() {
            Some(Box::new(self.parse_decl_type()?))
        } else {
            None
        };
        let var_id = self.parse_var_id()?;
        let colon = self.expect(COLON, "':'")?;
        let expression = self.parse_expression()?;
        self.skip_newlines();
        let rparen = self.expect(RPAREN, "')'")?;
        let (lbrace, block_statements, rbrace) = self.parse_braced_block()?;
        Ok(Node::ForEachStatement(ForEachStatementContext {
            for_token,
            lparen,
            decl_type,
            var_id: Box::new(var_id),
            colon,
            expression: Box::new(expression),
            rparen,
            lbrace,
            block_statements: block_statements.map(Box::new),
            rbrace,
        }))
    }

    fn parse_function_statement(&mut self) -> PResult<Node> {
        let function_token = self.consume_node();
        let var_id = self.parse_var_id()?;
        let lparen = self.expect(LPAREN, "'('")?;
        self.skip_newlines();
        let params = if !self.la(RPAREN) {
            Some(Box::new(self.parse_formal_or_inferred_parameter_list()?))
        } else {
            None
        };
        self.skip_newlines();
        let rparen = self.expect(RPAREN, "')'")?;
        let (lbrace, block_statements, rbrace) = self.parse_braced_block()?;
        Ok(Node::FunctionStatement(FunctionStatementContext {
            function_token,
            var_id: Box::new(var_id),
            lparen,
            params,
            rparen,
            lbrace,
            block_statements: block_statements.map(Box::new),
            rbrace,
        }))
    }

    fn parse_macro_statement(&mut self) -> PResult<Node> {
        let macro_token = self.consume_node();
        let var_id = self.parse_var_id()?;
        let (lbrace, block_statements, rbrace) = self.parse_braced_block()?;
        Ok(Node::MacroStatement(MacroStatementContext {
            macro_token,
            var_id: Box::new(var_id),
            lbrace,
            block_statements: block_statements.map(Box::new),
            rbrace,
        }))
    }

    fn parse_non_expression_statement(&mut self) -> PResult<Node> {
        let statement = if self.la(NEWLINE)
            || self.la(SEMI)
            || self.la(THROW)
            || self.la(WHILE)
            || self.la(FOR)
            || self.la(FUNCTION)
            || self.la(MACRO)
            || self.la(BREAK)
            || self.la(CONTINUE)
            || self.la(RETURN)
        {
            self.parse_block_statement()?
        } else {
            match self.try_parse_local_variable_declaration()? {
                None => {
                    let tok = self.lt();
                    return Err(self.syntax(
                        tok,
                        format!("mismatched input '{}' expecting statement", token_text(tok)),
                    ));
                }
                Some(local) => {
                    let semi = self.expect(SEMI, "';'")?;
                    Node::LocalVariableDeclarationStatement(
                        LocalVariableDeclarationStatementContext {
                            local_variable_declaration: Box::new(local),
                            semi,
                        },
                    )
                }
            }
        };
        Ok(Node::NonExpressionStatement(
            NonExpressionStatementContext {
                statement: Box::new(statement),
            },
        ))
    }

    // ------------------------------------------------------------------
    // Local variable declarations
    // ------------------------------------------------------------------

    /// Java `tryParseLocalVariableDeclaration`: catches `QLParseBacktrack`.
    fn try_parse_local_variable_declaration(&mut self) -> PResult<Option<Node>> {
        let save = self.p;
        let decl_type = match self.parse_decl_type() {
            Ok(decl_type) => decl_type,
            Err(ParseFail::Backtrack) => {
                self.p = save;
                return Ok(None);
            }
            Err(fatal) => return Err(fatal),
        };
        if !is_var_id_token(self.lt().token_type()) {
            self.p = save;
            return Ok(None);
        }
        if self.is_middle_operator(self.lt()) {
            self.p = save;
            return Ok(None);
        }
        let variable_declarator_list = self.parse_variable_declarator_list()?;
        Ok(Some(Node::LocalVariableDeclaration(
            LocalVariableDeclarationContext {
                decl_type: Box::new(decl_type),
                variable_declarator_list: Box::new(variable_declarator_list),
            },
        )))
    }

    fn parse_variable_declarator_list(&mut self) -> PResult<Node> {
        let mut ctx = VariableDeclaratorListContext {
            variables: Vec::new(),
            commas: Vec::new(),
        };
        ctx.variables.push(self.parse_variable_declarator()?);
        self.skip_newlines();
        while self.la(COMMA) {
            ctx.commas.push(self.consume_node());
            self.skip_newlines();
            ctx.variables.push(self.parse_variable_declarator()?);
            self.skip_newlines();
        }
        Ok(Node::VariableDeclaratorList(ctx))
    }

    fn parse_variable_declarator(&mut self) -> PResult<Node> {
        let id = self.parse_variable_declarator_id()?;
        let (equals, initializer) = if self.la(EQ) {
            let equals = self.consume_node();
            self.skip_newlines();
            (
                Some(equals),
                Some(Box::new(self.parse_variable_initializer()?)),
            )
        } else {
            (None, None)
        };
        Ok(Node::VariableDeclarator(VariableDeclaratorContext {
            id: Box::new(id),
            equals,
            initializer,
        }))
    }

    fn parse_variable_declarator_id(&mut self) -> PResult<Node> {
        let var_id = self.parse_var_id()?;
        let dims = if self.la(LBRACK) && self.la_at(1, RBRACK) {
            Some(Box::new(self.parse_dims()?))
        } else {
            None
        };
        Ok(Node::VariableDeclaratorId(VariableDeclaratorIdContext {
            var_id: Box::new(var_id),
            dims,
        }))
    }

    fn parse_variable_initializer(&mut self) -> PResult<Node> {
        if self.la(LBRACE) && self.is_array_initializer_ahead() {
            let array_initializer = self.parse_array_initializer()?;
            return Ok(Node::VariableInitializer(VariableInitializerContext {
                expression: None,
                array_initializer: Some(Box::new(array_initializer)),
            }));
        }
        let expression = self.parse_expression()?;
        Ok(Node::VariableInitializer(VariableInitializerContext {
            expression: Some(Box::new(expression)),
            array_initializer: None,
        }))
    }

    fn is_array_initializer_ahead(&mut self) -> bool {
        !self.is_map_expr_ahead()
    }

    fn parse_array_initializer(&mut self) -> PResult<Node> {
        let lbrace = self.expect(LBRACE, "'{'")?;
        self.skip_newlines();
        let initializers = if !self.la(RBRACE) {
            Some(Box::new(self.parse_variable_initializer_list()?))
        } else {
            None
        };
        self.skip_newlines();
        let rbrace = self.expect(RBRACE, "'}'")?;
        Ok(Node::ArrayInitializer(ArrayInitializerContext {
            lbrace,
            initializers,
            rbrace,
        }))
    }

    fn parse_variable_initializer_list(&mut self) -> PResult<Node> {
        let mut ctx = VariableInitializerListContext {
            initializers: Vec::new(),
            commas: Vec::new(),
        };
        ctx.initializers.push(self.parse_variable_initializer()?);
        self.skip_newlines();
        while self.la(COMMA) {
            ctx.commas.push(self.consume_node());
            self.skip_newlines();
            if self.la(RBRACE) {
                break;
            }
            ctx.initializers.push(self.parse_variable_initializer()?);
            self.skip_newlines();
        }
        Ok(Node::VariableInitializerList(ctx))
    }
}

// ---------------------------------------------------------------------------
// Expressions
// ---------------------------------------------------------------------------

impl<'a> QLParser<'a> {
    fn parse_expression(&mut self) -> PResult<Node> {
        let save = self.p;
        if self.has_top_level_assign_operator_ahead() {
            let left = self.try_parse_left_hand_side()?;
            if left.is_some() && is_assign_operator(self.lt().token_type()) {
                let assign_operator = self.parse_assign_operator()?;
                self.skip_newlines();
                let expression = self.parse_expression()?;
                return Ok(Node::Expression(ExpressionContext {
                    left: left.map(Box::new),
                    assign_operator: Some(Box::new(assign_operator)),
                    expression: Some(Box::new(expression)),
                    ternary: None,
                }));
            }
            self.p = save;
        }
        let ternary = self.parse_ternary_expr()?;
        Ok(Node::Expression(ExpressionContext {
            left: None,
            assign_operator: None,
            expression: None,
            ternary: Some(Box::new(ternary)),
        }))
    }

    fn try_parse_left_hand_side(&mut self) -> PResult<Option<Node>> {
        if !is_var_id_token(self.lt().token_type()) {
            return Ok(None);
        }
        let var_id = self.parse_var_id()?;
        let mut ctx = LeftHandSideContext {
            var_id: Box::new(var_id),
            lparen: None,
            argument_list: None,
            rparen: None,
            path_parts: Vec::new(),
        };
        if self.la(LPAREN) {
            ctx.lparen = Some(self.consume_node());
            self.skip_newlines();
            if !self.la(RPAREN) {
                ctx.argument_list = Some(Box::new(self.parse_argument_list()?));
            }
            self.skip_newlines();
            ctx.rparen = Some(self.expect(RPAREN, "')'")?);
        }
        loop {
            let before_newlines = self.p;
            self.skip_newlines();
            match self.try_parse_path_part()? {
                None => {
                    self.p = before_newlines;
                    break;
                }
                Some(part) => ctx.path_parts.push(part),
            }
        }
        Ok(Some(Node::LeftHandSide(ctx)))
    }

    fn parse_assign_operator(&mut self) -> PResult<Node> {
        if !is_assign_operator(self.lt().token_type()) {
            let tok = self.lt();
            return Err(self.syntax(
                tok,
                format!(
                    "mismatched input '{}' expecting assignment operator",
                    token_text(tok)
                ),
            ));
        }
        Ok(Node::AssignOperator(AssignOperatorContext {
            token: self.consume_node(),
        }))
    }

    fn parse_ternary_expr(&mut self) -> PResult<Node> {
        let condition = self.parse_base_expr(1)?;
        let mut ctx = TernaryExprContext {
            condition: Box::new(condition),
            question: None,
            then_expr: None,
            colon: None,
            else_expr: None,
        };
        if self.la(QUESTION) {
            ctx.question = Some(self.consume_node());
            self.skip_newlines();
            ctx.then_expr = Some(Box::new(self.parse_base_expr(0)?));
            ctx.colon = Some(self.expect(COLON, "':'")?);
            self.skip_newlines();
            ctx.else_expr = Some(Box::new(self.parse_expression()?));
        }
        Ok(Node::TernaryExpr(ctx))
    }

    fn parse_base_expr(&mut self, min_precedence: i32) -> PResult<Node> {
        let primary = self.parse_primary()?;
        let mut ctx = BaseExprContext {
            primary: Box::new(primary),
            left_assos: Vec::new(),
        };
        while !self.la(EOF)
            && (!self.strict_new_lines || !self.la(NEWLINE))
            && self.is_middle_operator(self.lt())
            && self.precedence(self.lt()) >= min_precedence
        {
            let binaryop = self.parse_binaryop()?;
            let is_instanceof = binaryop.text() == "instanceof";
            let next_min = {
                match &binaryop {
                    Node::Binaryop(op) => self.precedence(op.token.symbol()) + 1,
                    _ => unreachable!("parse_binaryop returns Binaryop"),
                }
            };
            self.skip_newlines();
            let right = if is_instanceof {
                let decl_type = self.parse_decl_type()?;
                Node::Primary(PrimaryContext {
                    prefix: None,
                    pathable: None,
                    path_parts: Vec::new(),
                    suffix: None,
                    non_pathable: Some(Box::new(Node::TypeExpr(TypeExprContext {
                        decl_type: Box::new(decl_type),
                    }))),
                })
            } else {
                self.parse_base_expr(next_min)?
            };
            ctx.left_assos.push(Node::LeftAsso(LeftAssoContext {
                binaryop: Box::new(binaryop),
                right: Box::new(right),
            }));
        }
        Ok(Node::BaseExpr(ctx))
    }

    fn parse_binaryop(&mut self) -> PResult<Node> {
        if !self.is_middle_operator(self.lt()) {
            let tok = self.lt();
            return Err(self.syntax(
                tok,
                format!("mismatched input '{}' expecting operator", token_text(tok)),
            ));
        }
        Ok(Node::Binaryop(BinaryopContext {
            token: self.consume_node(),
        }))
    }

    fn parse_primary(&mut self) -> PResult<Node> {
        if self.is_lambda_start() {
            let non_pathable = self.parse_lambda_expr()?;
            return Ok(Node::Primary(PrimaryContext {
                prefix: None,
                pathable: None,
                path_parts: Vec::new(),
                suffix: None,
                non_pathable: Some(Box::new(non_pathable)),
            }));
        }
        if self.la(IF) {
            let non_pathable = self.parse_ql_if()?;
            return Ok(Node::Primary(PrimaryContext {
                prefix: None,
                pathable: None,
                path_parts: Vec::new(),
                suffix: None,
                non_pathable: Some(Box::new(non_pathable)),
            }));
        }
        if self.la(SWITCH) && self.la_at(1, LPAREN) {
            let non_pathable = self.parse_switch_expr()?;
            return Ok(Node::Primary(PrimaryContext {
                prefix: None,
                pathable: None,
                path_parts: Vec::new(),
                suffix: None,
                non_pathable: Some(Box::new(non_pathable)),
            }));
        }
        if self.la(TRY) {
            let non_pathable = self.parse_try_catch_expr()?;
            return Ok(Node::Primary(PrimaryContext {
                prefix: None,
                pathable: None,
                path_parts: Vec::new(),
                suffix: None,
                non_pathable: Some(Box::new(non_pathable)),
            }));
        }
        let mut ctx = PrimaryContext {
            prefix: None,
            pathable: None,
            path_parts: Vec::new(),
            suffix: None,
            non_pathable: None,
        };
        if self.is_prefix_operator(self.lt()) {
            let op_id = self.parse_op_id()?;
            ctx.prefix = Some(Box::new(Node::PrefixExpress(PrefixExpressContext {
                op_id: Box::new(op_id),
            })));
        }
        ctx.pathable = Some(Box::new(self.parse_primary_no_fix_pathable()?));
        loop {
            let before_newlines = self.p;
            self.skip_newlines();
            match self.try_parse_path_part()? {
                None => {
                    self.p = before_newlines;
                    break;
                }
                Some(part) => ctx.path_parts.push(part),
            }
        }
        if self.is_suffix_operator(self.lt()) {
            let op_id = self.parse_op_id()?;
            ctx.suffix = Some(Box::new(Node::SuffixExpress(SuffixExpressContext {
                op_id: Box::new(op_id),
            })));
        }
        Ok(Node::Primary(ctx))
    }

    fn parse_primary_no_fix_pathable(&mut self) -> PResult<Node> {
        if is_literal_start(self.lt().token_type()) {
            let literal = self.parse_literal()?;
            return Ok(Node::ConstExpr(ConstExprContext {
                literal: Box::new(literal),
            }));
        }
        if self.la(LPAREN) {
            if self.is_cast_start() {
                let lparen = self.expect(LPAREN, "'('")?;
                self.skip_newlines();
                let decl_type = self.parse_decl_type()?;
                self.skip_newlines();
                let rparen = self.expect(RPAREN, "')'")?;
                let primary = self.parse_primary()?;
                return Ok(Node::CastExpr(CastExprContext {
                    lparen,
                    decl_type: Box::new(decl_type),
                    rparen,
                    primary: Box::new(primary),
                }));
            }
            let lparen = self.expect(LPAREN, "'('")?;
            self.skip_newlines();
            let expression = self.parse_expression()?;
            self.skip_newlines();
            let rparen = self.expect(RPAREN, "')'")?;
            return Ok(Node::GroupExpr(GroupExprContext {
                lparen,
                expression: Box::new(expression),
                rparen,
            }));
        }
        if self.la(NEW) {
            return self.parse_new_expr();
        }
        if is_primitive_type(self.lt().token_type()) {
            let decl_type = self.parse_decl_type()?;
            return Ok(Node::TypeExpr(TypeExprContext {
                decl_type: Box::new(decl_type),
            }));
        }
        if self.la(LBRACK) {
            return self.parse_list_expr();
        }
        if self.la(LBRACE) {
            return if self.is_map_expr_ahead() {
                self.parse_map_expr()
            } else {
                self.parse_block_expr()
            };
        }
        if self.la(SELECTOR_START) {
            let selector_start = self.consume_node();
            let selector_variable = self.expect(SELECTOR_VARIABLE_VANME, "selector variable")?;
            return Ok(Node::ContextSelectExpr(ContextSelectExprContext {
                selector_start,
                selector_variable,
            }));
        }
        if is_var_id_token(self.lt().token_type()) {
            let var_id = self.parse_var_id()?;
            let mut ctx = VarIdExprContext {
                var_id: Box::new(var_id),
                lparen: None,
                argument_list: None,
                rparen: None,
            };
            if self.la(LPAREN) {
                ctx.lparen = Some(self.consume_node());
                self.skip_newlines();
                if !self.la(RPAREN) {
                    ctx.argument_list = Some(Box::new(self.parse_argument_list()?));
                }
                self.skip_newlines();
                ctx.rparen = Some(self.expect(RPAREN, "')'")?);
            }
            return Ok(Node::VarIdExpr(ctx));
        }
        let tok = self.lt();
        Err(self.syntax(
            tok,
            format!(
                "mismatched input '{}' expecting expression",
                token_text(tok)
            ),
        ))
    }

    /// Java `isCastStart`: speculative, catches all errors.
    fn is_cast_start(&mut self) -> bool {
        let save = self.p;
        let result = (|| -> PResult<bool> {
            self.expect(LPAREN, "'('")?;
            self.skip_newlines();
            self.parse_decl_type()?;
            self.skip_newlines();
            if !self.la(RPAREN) {
                return Ok(false);
            }
            self.consume();
            Ok(self.is_primary_start(self.lt()))
        })();
        self.p = save;
        matches!(result, Ok(true))
    }

    fn parse_new_expr(&mut self) -> PResult<Node> {
        let new_token = self.consume_node();
        let decl_type = self.parse_decl_type_no_arr()?;
        if self.la(LPAREN) {
            let var_ids = match &decl_type {
                Node::DeclTypeNoArr(d) => match &d.cls_type {
                    None => {
                        return Err(self.syntax(
                            new_token.symbol(),
                            "primitive type can not be constructed".to_string(),
                        ));
                    }
                    Some(cls_type) => match cls_type.as_ref() {
                        Node::ClsType(c) => c.var_ids.clone(),
                        _ => unreachable!("cls_type holds ClsType"),
                    },
                },
                _ => unreachable!("parse_decl_type_no_arr returns DeclTypeNoArr"),
            };
            let lparen = self.expect(LPAREN, "'('")?;
            self.skip_newlines();
            let argument_list = if !self.la(RPAREN) {
                Some(Box::new(self.parse_argument_list()?))
            } else {
                None
            };
            self.skip_newlines();
            let rparen = self.expect(RPAREN, "')'")?;
            return Ok(Node::NewObjExpr(NewObjExprContext {
                new_token,
                var_ids,
                lparen,
                argument_list,
                rparen,
            }));
        }
        if self.la(LBRACK) && !self.la_at(1, RBRACK) {
            let dim_exprs = self.parse_dim_exprs()?;
            return Ok(Node::NewEmptyArrExpr(NewEmptyArrExprContext {
                new_token,
                decl_type_no_arr: Box::new(decl_type),
                dim_exprs: Box::new(dim_exprs),
            }));
        }
        let dims = self.parse_dims()?;
        let array_initializer = self.parse_array_initializer()?;
        Ok(Node::NewInitArrExpr(NewInitArrExprContext {
            new_token,
            decl_type_no_arr: Box::new(decl_type),
            dims: Box::new(dims),
            array_initializer: Box::new(array_initializer),
        }))
    }

    fn parse_list_expr(&mut self) -> PResult<Node> {
        let lbrack = self.expect(LBRACK, "'['")?;
        self.skip_newlines();
        let list_items = if !self.la(RBRACK) {
            Some(Box::new(self.parse_list_items()?))
        } else {
            None
        };
        self.skip_newlines();
        let rbrack = self.expect(RBRACK, "']'")?;
        Ok(Node::ListExpr(ListExprContext {
            lbrack,
            list_items,
            rbrack,
        }))
    }

    fn parse_list_items(&mut self) -> PResult<Node> {
        let mut expressions = vec![self.parse_expression()?];
        let mut commas = Vec::new();
        self.skip_newlines();
        while self.la(COMMA) {
            commas.push(self.consume_node());
            self.skip_newlines();
            if self.la(RBRACK) {
                break;
            }
            expressions.push(self.parse_expression()?);
            self.skip_newlines();
        }
        Ok(Node::ListItems(ListItemsContext {
            expressions,
            commas,
        }))
    }

    fn parse_map_expr(&mut self) -> PResult<Node> {
        let lbrace = self.expect(LBRACE, "'{'")?;
        self.skip_newlines();
        let mut map_entries = MapEntriesContext {
            empty_colon: None,
            entries: Vec::new(),
            commas: Vec::new(),
        };
        if self.la(COLON) {
            map_entries.empty_colon = Some(self.consume_node());
        } else {
            map_entries.entries.push(self.parse_map_entry()?);
            self.skip_newlines();
            while self.la(COMMA) {
                map_entries.commas.push(self.consume_node());
                self.skip_newlines();
                if self.la(RBRACE) {
                    break;
                }
                map_entries.entries.push(self.parse_map_entry()?);
                self.skip_newlines();
            }
        }
        self.skip_newlines();
        let rbrace = self.expect(RBRACE, "'}'")?;
        Ok(Node::MapExpr(MapExprContext {
            lbrace,
            map_entries: Box::new(Node::MapEntries(map_entries)),
            rbrace,
        }))
    }

    fn parse_map_entry(&mut self) -> PResult<Node> {
        let map_key = self.parse_map_key()?;
        self.skip_newlines();
        let colon = self.expect(COLON, "':'")?;
        self.skip_newlines();
        let map_value = if map_key.text() == "'@class'" && self.la(QUOTE_STRING_LITERAL) {
            Node::ClsValue(ClsValueContext {
                quote: self.consume_node(),
            })
        } else {
            Node::EValue(EValueContext {
                expression: Box::new(self.parse_expression()?),
            })
        };
        Ok(Node::MapEntry(MapEntryContext {
            map_key: Box::new(map_key),
            colon,
            map_value: Box::new(map_value),
        }))
    }

    fn parse_map_key(&mut self) -> PResult<Node> {
        if self.la(QUOTE_STRING_LITERAL) {
            return Ok(Node::QuoteStringKey(QuoteStringKeyContext {
                token: self.consume_node(),
            }));
        }
        if self.la(DOUBLE_QUOTE) {
            let double_quote_string = self.parse_double_quote_string_literal()?;
            return Ok(Node::StringKey(StringKeyContext {
                double_quote_string: Box::new(double_quote_string),
            }));
        }
        if is_id_map_key(self.lt().token_type()) {
            return Ok(Node::IdKey(IdKeyContext {
                token: self.consume_node(),
            }));
        }
        let tok = self.lt();
        Err(self.syntax(
            tok,
            format!("mismatched input '{}' expecting map key", token_text(tok)),
        ))
    }

    fn parse_block_expr(&mut self) -> PResult<Node> {
        let lbrace = self.expect(LBRACE, "'{'")?;
        self.skip_newlines();
        let block_statements = if !self.la(RBRACE) {
            Some(Box::new(self.parse_block_statements_until(RBRACE)?))
        } else {
            None
        };
        self.skip_newlines();
        let rbrace = self.expect(RBRACE, "'}'")?;
        Ok(Node::BlockExpr(BlockExprContext {
            lbrace,
            block_statements,
            rbrace,
        }))
    }
}

// ---------------------------------------------------------------------------
// if / switch / try / lambda
// ---------------------------------------------------------------------------

impl<'a> QLParser<'a> {
    fn parse_ql_if(&mut self) -> PResult<Node> {
        let if_token = self.consume_node();
        let lparen = self.expect(LPAREN, "'('")?;
        self.skip_newlines();
        let condition = self.parse_expression()?;
        self.skip_newlines();
        let rparen = self.expect(RPAREN, "')'")?;
        self.skip_newlines();
        let then_keyword = if self.la(THEN) {
            let kw = self.consume_node();
            self.skip_newlines();
            Some(kw)
        } else {
            None
        };
        let then_body = self.parse_then_body()?;
        let save = self.p;
        self.skip_newlines();
        let (else_keyword, else_body) = if self.la(ELSE) {
            let else_keyword = self.consume_node();
            self.skip_newlines();
            (Some(else_keyword), Some(Box::new(self.parse_else_body()?)))
        } else {
            self.p = save;
            (None, None)
        };
        Ok(Node::QlIf(QlIfContext {
            if_token,
            lparen,
            then_keyword,
            condition: Box::new(condition),
            rparen,
            then_body: Box::new(then_body),
            else_body,
            else_keyword,
        }))
    }

    fn parse_then_body(&mut self) -> PResult<Node> {
        if self.la(LBRACE) {
            let lbrace = self.expect(LBRACE, "'{'")?;
            self.skip_newlines();
            let block_statements = if !self.la(RBRACE) {
                Some(Box::new(self.parse_block_statements_until(RBRACE)?))
            } else {
                None
            };
            self.skip_newlines();
            let rbrace = self.expect(RBRACE, "'}'")?;
            return Ok(Node::ThenBody(ThenBodyContext {
                lbrace: Some(lbrace),
                block_statements,
                rbrace: Some(rbrace),
                non_expression_statement: None,
                expression: None,
            }));
        }
        if self.is_non_expression_statement_start()? {
            let non_expression_statement = self.parse_non_expression_statement()?;
            return Ok(Node::ThenBody(ThenBodyContext {
                lbrace: None,
                block_statements: None,
                rbrace: None,
                non_expression_statement: Some(Box::new(non_expression_statement)),
                expression: None,
            }));
        }
        let expression = self.parse_expression()?;
        Ok(Node::ThenBody(ThenBodyContext {
            lbrace: None,
            block_statements: None,
            rbrace: None,
            non_expression_statement: None,
            expression: Some(Box::new(expression)),
        }))
    }

    fn parse_else_body(&mut self) -> PResult<Node> {
        if self.la(LBRACE) {
            let lbrace = self.expect(LBRACE, "'{'")?;
            self.skip_newlines();
            let block_statements = if !self.la(RBRACE) {
                Some(Box::new(self.parse_block_statements_until(RBRACE)?))
            } else {
                None
            };
            self.skip_newlines();
            let rbrace = self.expect(RBRACE, "'}'")?;
            return Ok(Node::ElseBody(ElseBodyContext {
                lbrace: Some(lbrace),
                block_statements,
                rbrace: Some(rbrace),
                ql_if: None,
                non_expression_statement: None,
                expression: None,
            }));
        }
        if self.la(IF) {
            let ql_if = self.parse_ql_if()?;
            return Ok(Node::ElseBody(ElseBodyContext {
                lbrace: None,
                block_statements: None,
                rbrace: None,
                ql_if: Some(Box::new(ql_if)),
                non_expression_statement: None,
                expression: None,
            }));
        }
        if self.is_non_expression_statement_start()? {
            let non_expression_statement = self.parse_non_expression_statement()?;
            return Ok(Node::ElseBody(ElseBodyContext {
                lbrace: None,
                block_statements: None,
                rbrace: None,
                ql_if: None,
                non_expression_statement: Some(Box::new(non_expression_statement)),
                expression: None,
            }));
        }
        let expression = self.parse_expression()?;
        Ok(Node::ElseBody(ElseBodyContext {
            lbrace: None,
            block_statements: None,
            rbrace: None,
            ql_if: None,
            non_expression_statement: None,
            expression: Some(Box::new(expression)),
        }))
    }

    fn parse_switch_expr(&mut self) -> PResult<Node> {
        let switch_token = self.consume_node();
        let lparen = self.expect(LPAREN, "'('")?;
        self.skip_newlines();
        let expression = self.parse_expression()?;
        self.skip_newlines();
        let rparen = self.expect(RPAREN, "')'")?;
        let lbrace = self.expect(LBRACE, "'{'")?;
        self.skip_newlines();
        let groups = if !self.la(RBRACE) {
            Some(Box::new(self.parse_switch_case_groups()?))
        } else {
            None
        };
        self.skip_newlines();
        let rbrace = self.expect(RBRACE, "'}'")?;
        Ok(Node::SwitchExpr(SwitchExprContext {
            switch_token,
            lparen,
            expression: Box::new(expression),
            rparen,
            lbrace,
            groups,
            rbrace,
        }))
    }

    fn parse_switch_case_groups(&mut self) -> PResult<Node> {
        let mut ctx = SwitchCaseGroupsContext { groups: Vec::new() };
        while self.la(CASE) || self.la(DEFAULT) {
            ctx.groups.push(self.parse_switch_case_group()?);
            self.skip_newlines();
        }
        Ok(Node::SwitchCaseGroups(ctx))
    }

    fn parse_switch_case_group(&mut self) -> PResult<Node> {
        let save = self.p;
        if let Some(expr_label) = self.try_parse_switch_expression_label() {
            self.skip_newlines();
            let expression = self.parse_expression()?;
            self.skip_newlines();
            return Ok(Node::SwitchExprGroup(SwitchExprGroupContext {
                label: Box::new(expr_label),
                expression: Box::new(expression),
            }));
        }
        self.p = save;
        let labels = self.parse_switch_labels()?;
        self.skip_newlines();
        let block_statements = if !self.la(CASE) && !self.la(DEFAULT) && !self.la(RBRACE) {
            Some(Box::new(
                self.parse_block_statements_until_switch_group_end()?,
            ))
        } else {
            None
        };
        self.skip_newlines();
        Ok(Node::SwitchStatementGroup(SwitchStatementGroupContext {
            labels: Box::new(labels),
            block_statements,
        }))
    }

    fn parse_block_statements_until_switch_group_end(&mut self) -> PResult<Node> {
        let mut ctx = BlockStatementsContext {
            statements: Vec::new(),
        };
        while !self.la(CASE) && !self.la(DEFAULT) && !self.la(RBRACE) && !self.la(EOF) {
            ctx.statements.push(self.parse_block_statement()?);
        }
        Ok(Node::BlockStatements(ctx))
    }

    /// Java `tryParseSwitchExpressionLabel`: catches all errors.
    fn try_parse_switch_expression_label(&mut self) -> Option<Node> {
        let save = self.p;
        let parsed = (|| -> PResult<Node> {
            let mut ctx = SwitchExpressionLabelContext {
                case_token: None,
                default_token: None,
                expression_list: None,
                arrow: TerminalNode::new(self.lt().clone()),
            };
            if self.la(CASE) {
                ctx.case_token = Some(self.consume_node());
                ctx.expression_list = Some(Box::new(self.parse_expression_list_until_arrow()?));
                self.skip_newlines();
                ctx.arrow = self.expect(ARROW, "'->'")?;
            } else if self.la(DEFAULT) {
                ctx.default_token = Some(self.consume_node());
                self.skip_newlines();
                ctx.arrow = self.expect(ARROW, "'->'")?;
            } else {
                return Err(ParseFail::Backtrack);
            }
            Ok(Node::SwitchExpressionLabel(ctx))
        })();
        match parsed {
            Ok(node) => {
                self.skip_newlines();
                Some(node)
            }
            Err(_) => {
                self.p = save;
                None
            }
        }
    }

    fn parse_switch_labels(&mut self) -> PResult<Node> {
        let mut ctx = SwitchLabelsContext { labels: Vec::new() };
        while self.la(CASE) || self.la(DEFAULT) {
            let mut label = SwitchLabelContext {
                case_token: None,
                default_token: None,
                expression: None,
                colon: TerminalNode::new(self.lt().clone()),
            };
            if self.la(CASE) {
                label.case_token = Some(self.consume_node());
                label.expression = Some(Box::new(self.parse_expression()?));
            } else {
                label.default_token = Some(self.consume_node());
            }
            label.colon = self.expect(COLON, "':'")?;
            ctx.labels.push(Node::SwitchLabel(label));
            self.skip_newlines();
        }
        Ok(Node::SwitchLabels(ctx))
    }

    fn parse_expression_list_until_arrow(&mut self) -> PResult<Node> {
        let mut ctx = ExpressionListContext {
            expressions: Vec::new(),
            commas: Vec::new(),
        };
        ctx.expressions.push(self.parse_expression()?);
        self.skip_newlines();
        while self.la(COMMA) {
            ctx.commas.push(self.consume_node());
            self.skip_newlines();
            ctx.expressions.push(self.parse_expression()?);
            self.skip_newlines();
        }
        Ok(Node::ExpressionList(ctx))
    }

    fn parse_try_catch_expr(&mut self) -> PResult<Node> {
        let try_token = self.consume_node();
        let (lbrace, block_statements, rbrace) = self.parse_braced_block()?;
        let save = self.p;
        self.skip_newlines();
        let try_catches = if self.la(CATCH) {
            let mut catches = Vec::new();
            while self.la(CATCH) {
                catches.push(self.parse_try_catch()?);
                let save = self.p;
                self.skip_newlines();
                if !self.la(CATCH) {
                    self.p = save;
                    break;
                }
            }
            Some(Box::new(Node::TryCatches(TryCatchesContext { catches })))
        } else {
            self.p = save;
            None
        };
        let save = self.p;
        self.skip_newlines();
        let try_finally = if self.la(FINALLY) {
            Some(Box::new(self.parse_try_finally()?))
        } else {
            self.p = save;
            None
        };
        Ok(Node::TryCatchExpr(TryCatchExprContext {
            try_token,
            lbrace,
            block_statements: block_statements.map(Box::new),
            rbrace,
            try_catches,
            try_finally,
        }))
    }

    fn parse_try_catch(&mut self) -> PResult<Node> {
        let catch_token = self.consume_node();
        let lparen = self.expect(LPAREN, "'('")?;
        let catch_params = self.parse_catch_params()?;
        let rparen = self.expect(RPAREN, "')'")?;
        let (lbrace, block_statements, rbrace) = self.parse_braced_block()?;
        Ok(Node::TryCatch(TryCatchContext {
            catch_token,
            lparen,
            catch_params: Box::new(catch_params),
            rparen,
            lbrace,
            block_statements: block_statements.map(Box::new),
            rbrace,
        }))
    }

    fn parse_catch_params(&mut self) -> PResult<Node> {
        if self.single_var_before(&[RPAREN]) {
            let var_id = self.parse_var_id()?;
            return Ok(Node::CatchParams(CatchParamsContext {
                decl_types: Vec::new(),
                bit_ors: Vec::new(),
                var_id: Box::new(var_id),
            }));
        }
        let mut decl_types = vec![self.parse_decl_type()?];
        let mut bit_ors = Vec::new();
        while self.la(BIT_OR) {
            bit_ors.push(self.consume_node());
            decl_types.push(self.parse_decl_type()?);
        }
        let var_id = self.parse_var_id()?;
        Ok(Node::CatchParams(CatchParamsContext {
            decl_types,
            bit_ors,
            var_id: Box::new(var_id),
        }))
    }

    fn parse_try_finally(&mut self) -> PResult<Node> {
        let finally_token = self.consume_node();
        let (lbrace, block_statements, rbrace) = self.parse_braced_block()?;
        Ok(Node::TryFinally(TryFinallyContext {
            finally_token,
            lbrace,
            block_statements: block_statements.map(Box::new),
            rbrace,
        }))
    }

    fn parse_lambda_expr(&mut self) -> PResult<Node> {
        let lambda_parameters = self.parse_lambda_parameters()?;
        let arrow = self.expect(ARROW, "'->'")?;
        self.skip_newlines();
        let mut ctx = LambdaExprContext {
            lambda_parameters: Box::new(lambda_parameters),
            arrow,
            lbrace: None,
            block_statements: None,
            rbrace: None,
            expression: None,
        };
        if self.la(LBRACE) && !self.is_map_expr_ahead() {
            ctx.lbrace = Some(self.expect(LBRACE, "'{'")?);
            self.skip_newlines();
            if !self.la(RBRACE) {
                ctx.block_statements = Some(Box::new(self.parse_block_statements_until(RBRACE)?));
            }
            self.skip_newlines();
            ctx.rbrace = Some(self.expect(RBRACE, "'}'")?);
        } else {
            ctx.expression = Some(Box::new(self.parse_expression()?));
        }
        Ok(Node::LambdaExpr(ctx))
    }

    fn parse_lambda_parameters(&mut self) -> PResult<Node> {
        if is_var_id_token(self.lt().token_type()) && self.la_at(1, ARROW) {
            let var_id = self.parse_var_id()?;
            return Ok(Node::LambdaParameters(LambdaParametersContext {
                var_id: Some(Box::new(var_id)),
                lparen: None,
                params: None,
                rparen: None,
            }));
        }
        let lparen = self.expect(LPAREN, "'('")?;
        let params = if !self.la(RPAREN) {
            Some(Box::new(self.parse_formal_or_inferred_parameter_list()?))
        } else {
            None
        };
        let rparen = self.expect(RPAREN, "')'")?;
        Ok(Node::LambdaParameters(LambdaParametersContext {
            var_id: None,
            lparen: Some(lparen),
            params,
            rparen: Some(rparen),
        }))
    }

    fn parse_formal_or_inferred_parameter_list(&mut self) -> PResult<Node> {
        let mut ctx = FormalOrInferredParameterListContext {
            params: Vec::new(),
            commas: Vec::new(),
        };
        ctx.params.push(self.parse_formal_or_inferred_parameter()?);
        self.skip_newlines();
        while self.la(COMMA) {
            ctx.commas.push(self.consume_node());
            self.skip_newlines();
            ctx.params.push(self.parse_formal_or_inferred_parameter()?);
            self.skip_newlines();
        }
        Ok(Node::FormalOrInferredParameterList(ctx))
    }

    fn parse_formal_or_inferred_parameter(&mut self) -> PResult<Node> {
        let mut ctx = FormalOrInferredParameterContext {
            decl_type: None,
            var_id: Box::new(Node::VarId(VarIdContext {
                token: TerminalNode::new(self.lt().clone()),
            })),
        };
        if !self.single_var_before(&[COMMA, RPAREN]) {
            let save = self.p;
            let decl_type = self.parse_decl_type();
            match decl_type {
                Ok(decl_type) if is_var_id_token(self.lt().token_type()) => {
                    ctx.decl_type = Some(Box::new(decl_type));
                }
                Ok(_) => {
                    self.p = save;
                }
                Err(_) => {
                    self.p = save;
                }
            }
        }
        ctx.var_id = Box::new(self.parse_var_id()?);
        Ok(Node::FormalOrInferredParameter(ctx))
    }
}

// ---------------------------------------------------------------------------
// Path parts / index / arguments / literals / import / types
// ---------------------------------------------------------------------------

impl<'a> QLParser<'a> {
    fn try_parse_path_part(&mut self) -> PResult<Option<Node>> {
        if self.la(DOT) {
            let dot = self.consume_node();
            if is_var_id_token(self.lt().token_type()) && self.la_at(1, LPAREN) {
                let var_id = self.parse_var_id()?;
                let mut ctx = MethodInvokeContext {
                    dot,
                    var_id: Box::new(var_id),
                    lparen: TerminalNode::new(self.lt().clone()),
                    argument_list: None,
                    rparen: TerminalNode::new(self.lt().clone()),
                    chain: ChainKind::Plain,
                };
                self.parse_method_arguments(&mut ctx)?;
                return Ok(Some(Node::MethodInvoke(ctx)));
            }
            let field_id = self.parse_field_id()?;
            return Ok(Some(Node::FieldAccess(FieldAccessContext {
                dot,
                field_id: Box::new(field_id),
                chain: ChainKind::Plain,
            })));
        }
        if self.la(OPTIONAL_CHAINING) || self.la(SPREAD_CHAINING) {
            let optional = self.la(OPTIONAL_CHAINING);
            let chain = if optional {
                ChainKind::Optional
            } else {
                ChainKind::Spread
            };
            let token = self.consume_node();
            if is_var_id_token(self.lt().token_type()) && self.la_at(1, LPAREN) {
                let var_id = self.parse_var_id()?;
                let mut ctx = MethodInvokeContext {
                    dot: token,
                    var_id: Box::new(var_id),
                    lparen: TerminalNode::new(self.lt().clone()),
                    argument_list: None,
                    rparen: TerminalNode::new(self.lt().clone()),
                    chain,
                };
                self.parse_method_arguments(&mut ctx)?;
                return Ok(Some(Node::MethodInvoke(ctx)));
            }
            let field_id = self.parse_field_id()?;
            return Ok(Some(Node::FieldAccess(FieldAccessContext {
                dot: token,
                field_id: Box::new(field_id),
                chain,
            })));
        }
        if self.la(DCOLON) {
            let dcolon = self.consume_node();
            let var_id = self.parse_var_id()?;
            return Ok(Some(Node::MethodAccess(MethodAccessContext {
                dcolon,
                var_id: Box::new(var_id),
            })));
        }
        if self.la(LBRACK) {
            let lbrack = self.expect(LBRACK, "'['")?;
            self.skip_newlines();
            let index_value_expr = if !self.la(RBRACK) {
                Some(Box::new(self.parse_index_value_expr()?))
            } else {
                None
            };
            self.skip_newlines();
            let rbrack = self.expect(RBRACK, "']'")?;
            return Ok(Some(Node::IndexExpr(IndexExprContext {
                lbrack,
                index_value_expr,
                rbrack,
            })));
        }
        if self.is_group_operator(self.lt()) {
            let op_id = self.parse_op_id()?;
            self.skip_newlines();
            let mut ctx = CustomPathContext {
                op_id: Box::new(op_id),
                var_id: None,
                quote: None,
                path_text: String::new(),
            };
            if is_var_id_token(self.lt().token_type()) {
                let var_id = self.parse_var_id()?;
                ctx.path_text = var_id.text();
                ctx.var_id = Some(Box::new(var_id));
            } else if self.la(QUOTE_STRING_LITERAL) {
                let quote = self.consume_node();
                let text = quote.text();
                ctx.path_text = text[1..text.len() - 1].to_string();
                ctx.quote = Some(quote);
            } else {
                let tok = self.lt();
                return Err(self.syntax(
                    tok,
                    format!(
                        "mismatched input '{}' expecting custom path",
                        token_text(tok)
                    ),
                ));
            }
            return Ok(Some(Node::CustomPath(ctx)));
        }
        Ok(None)
    }

    fn parse_method_arguments(&mut self, ctx: &mut MethodInvokeContext) -> PResult<()> {
        ctx.lparen = self.expect(LPAREN, "'('")?;
        self.skip_newlines();
        if !self.la(RPAREN) {
            ctx.argument_list = Some(Box::new(self.parse_argument_list()?));
        }
        self.skip_newlines();
        ctx.rparen = self.expect(RPAREN, "')'")?;
        Ok(())
    }

    fn parse_field_id(&mut self) -> PResult<Node> {
        if is_var_id_token(self.lt().token_type()) || self.la(CLASS) {
            return Ok(Node::FieldId(FieldIdContext {
                token: Some(self.consume_node()),
                quote: None,
            }));
        }
        if self.la(QUOTE_STRING_LITERAL) {
            return Ok(Node::FieldId(FieldIdContext {
                token: None,
                quote: Some(self.consume_node()),
            }));
        }
        let tok = self.lt();
        Err(self.syntax(
            tok,
            format!("mismatched input '{}' expecting field", token_text(tok)),
        ))
    }

    fn parse_index_value_expr(&mut self) -> PResult<Node> {
        if self.la(COLON) {
            let colon = self.consume_node();
            self.skip_newlines();
            let end = if !self.la(RBRACK) {
                Some(Box::new(self.parse_expression()?))
            } else {
                None
            };
            return Ok(Node::SliceIndex(SliceIndexContext {
                start: None,
                colon,
                end,
            }));
        }
        let first = self.parse_expression()?;
        self.skip_newlines();
        if self.la(COLON) {
            let colon = self.consume_node();
            self.skip_newlines();
            let end = if !self.la(RBRACK) {
                Some(Box::new(self.parse_expression()?))
            } else {
                None
            };
            return Ok(Node::SliceIndex(SliceIndexContext {
                start: Some(Box::new(first)),
                colon,
                end,
            }));
        }
        Ok(Node::SingleIndex(SingleIndexContext {
            expression: Box::new(first),
        }))
    }

    fn parse_argument_list(&mut self) -> PResult<Node> {
        let mut ctx = ArgumentListContext {
            expressions: Vec::new(),
            commas: Vec::new(),
        };
        ctx.expressions.push(self.parse_expression()?);
        self.skip_newlines();
        while self.la(COMMA) {
            ctx.commas.push(self.consume_node());
            self.skip_newlines();
            ctx.expressions.push(self.parse_expression()?);
            self.skip_newlines();
        }
        Ok(Node::ArgumentList(ctx))
    }

    fn parse_literal(&mut self) -> PResult<Node> {
        let ty = self.lt().token_type();
        if ty == INTEGER_LITERAL
            || ty == FLOATING_POINT_LITERAL
            || ty == INTEGER_OR_FLOATING_LITERAL
            || ty == QUOTE_STRING_LITERAL
            || ty == NULL
        {
            return Ok(Node::Literal(LiteralContext {
                token: Some(self.consume_node()),
                boolen: None,
                double_quote_string: None,
            }));
        }
        if ty == TRUE || ty == FALSE {
            return Ok(Node::Literal(LiteralContext {
                token: None,
                boolen: Some(Box::new(Node::BoolenLiteral(BoolenLiteralContext {
                    token: self.consume_node(),
                }))),
                double_quote_string: None,
            }));
        }
        if self.la(DOUBLE_QUOTE) {
            let double_quote_string = self.parse_double_quote_string_literal()?;
            return Ok(Node::Literal(LiteralContext {
                token: None,
                boolen: None,
                double_quote_string: Some(Box::new(double_quote_string)),
            }));
        }
        let tok = self.lt();
        Err(self.syntax(
            tok,
            format!("mismatched input '{}' expecting literal", token_text(tok)),
        ))
    }

    fn parse_double_quote_string_literal(&mut self) -> PResult<Node> {
        let open_quote = self.expect(DOUBLE_QUOTE, "'\"'")?;
        let mut ctx = DoubleQuoteStringLiteralContext {
            open_quote,
            static_characters: None,
            parts: Vec::new(),
            close_quote: TerminalNode::new(self.lt().clone()),
        };
        if self.la(STATIC_STRING_CHARACTERS) {
            ctx.static_characters = Some(self.consume_node());
        }
        while !self.la(DOUBLE_QUOTE) && !self.la(EOF) {
            if self.la(DY_STR_TEXT) {
                ctx.parts.push(DyStrPart::Text(self.consume_node()));
            } else if self.la(DY_STR_EXPR_START) {
                let start = self.consume_node();
                let expr = if self.la(SELECTOR_VARIABLE_VANME) {
                    StringExpressionContext {
                        start,
                        selector_variable: Some(self.consume_node()),
                        expression: None,
                        rbrace: None,
                    }
                } else {
                    self.skip_newlines();
                    let expression = self.parse_expression()?;
                    self.skip_newlines();
                    let rbrace = self.expect(RBRACE, "'}'")?;
                    StringExpressionContext {
                        start,
                        selector_variable: None,
                        expression: Some(Box::new(expression)),
                        rbrace: Some(rbrace),
                    }
                };
                ctx.parts
                    .push(DyStrPart::Expr(Box::new(Node::StringExpression(expr))));
            } else {
                let tok = self.lt();
                return Err(self.syntax(
                    tok,
                    format!(
                        "mismatched input '{}' expecting string content",
                        token_text(tok)
                    ),
                ));
            }
        }
        ctx.close_quote = self.expect(DOUBLE_QUOTE, "'\"'")?;
        Ok(Node::DoubleQuoteStringLiteral(ctx))
    }

    fn parse_import_declaration(&mut self) -> PResult<Node> {
        let import_token = self.expect(IMPORT, "'import'")?;
        let mut ids = vec![self.parse_var_id()?];
        while self.la(DOT) && !self.la_at(1, MUL) && !self.la_at(1, EOF) {
            self.consume();
            ids.push(self.parse_var_id()?);
        }
        let is_pack = self.la(DOTMUL) || (self.la(DOT) && self.la_at(1, MUL));
        if is_pack {
            if self.la(DOTMUL) {
                self.consume();
            } else {
                self.consume();
                self.consume();
            }
        }
        let semi = self.expect(SEMI, "';'")?;
        if is_pack {
            Ok(Node::ImportPack(ImportPackContext {
                import_token,
                var_ids: ids,
                semi,
            }))
        } else {
            Ok(Node::ImportCls(ImportClsContext {
                import_token,
                var_ids: ids,
                semi,
            }))
        }
    }

    // ------------------------------------------------------------------
    // Types
    // ------------------------------------------------------------------

    /// Java `parseDeclType` (throws `QLParseBacktrack` on non-type start).
    fn parse_decl_type(&mut self) -> PResult<Node> {
        let mut ctx = DeclTypeContext {
            primitive_type: None,
            cls_type: None,
            dims: None,
        };
        if is_primitive_type(self.lt().token_type()) {
            ctx.primitive_type = Some(Box::new(self.parse_primitive_type()?));
        } else if is_var_id_token(self.lt().token_type()) {
            ctx.cls_type = Some(Box::new(self.parse_cls_type()?));
        } else {
            return Self::backtrack();
        }
        if self.la(LBRACK) && self.la_at(1, RBRACK) {
            ctx.dims = Some(Box::new(self.parse_dims()?));
        }
        Ok(Node::DeclType(ctx))
    }

    fn parse_decl_type_no_arr(&mut self) -> PResult<Node> {
        let mut ctx = DeclTypeNoArrContext {
            primitive_type: None,
            cls_type: None,
        };
        if is_primitive_type(self.lt().token_type()) {
            ctx.primitive_type = Some(Box::new(self.parse_primitive_type()?));
        } else if is_var_id_token(self.lt().token_type()) {
            ctx.cls_type = Some(Box::new(self.parse_cls_type()?));
        } else {
            let tok = self.lt();
            return Err(self.syntax(
                tok,
                format!("mismatched input '{}' expecting type", token_text(tok)),
            ));
        }
        Ok(Node::DeclTypeNoArr(ctx))
    }

    fn parse_primitive_type(&mut self) -> PResult<Node> {
        if !is_primitive_type(self.lt().token_type()) {
            return Self::backtrack();
        }
        Ok(Node::PrimitiveType(PrimitiveTypeContext {
            token: self.consume_node(),
        }))
    }

    fn parse_cls_type(&mut self) -> PResult<Node> {
        let mut var_ids = vec![self.parse_var_id()?];
        while self.la(DOT) && is_var_id_token(self.token(self.p + 1).token_type()) {
            self.consume();
            var_ids.push(self.parse_var_id()?);
        }
        if self.la(LT) || self.la(NOEQ) {
            self.parse_type_arguments()?;
        }
        Ok(Node::ClsType(ClsTypeContext { var_ids }))
    }

    fn parse_type_arguments(&mut self) -> PResult<()> {
        if self.la(NOEQ) {
            self.consume();
            return Ok(());
        }
        self.expect(LT, "'<'")?;
        self.skip_newlines();
        if !self.is_type_argument_end() && !self.la(EOF) {
            self.parse_type_argument_list()?;
            self.skip_newlines();
        }
        if self.is_type_argument_end() {
            self.consume();
        }
        Ok(())
    }

    fn parse_type_argument_list(&mut self) -> PResult<()> {
        self.parse_type_argument()?;
        self.skip_newlines();
        while self.la(COMMA) {
            self.consume();
            self.skip_newlines();
            self.parse_type_argument()?;
            self.skip_newlines();
        }
        Ok(())
    }

    fn parse_type_argument(&mut self) -> PResult<()> {
        if self.la(QUESTION) {
            self.consume();
            self.skip_newlines();
            if self.la(EXTENDS) || self.la(SUPER) {
                self.consume();
                self.skip_newlines();
                self.parse_reference_type()?;
            }
            return Ok(());
        }
        self.parse_reference_type()
    }

    fn parse_reference_type(&mut self) -> PResult<()> {
        if is_var_id_token(self.lt().token_type()) {
            self.parse_cls_type()?;
            if self.la(LBRACK) && self.la_at(1, RBRACK) {
                self.parse_dims()?;
            }
            return Ok(());
        }
        if is_primitive_type(self.lt().token_type()) {
            self.parse_primitive_type()?;
            if self.la(LBRACK) && self.la_at(1, RBRACK) {
                self.parse_dims()?;
                return Ok(());
            }
        }
        let tok = self.lt();
        Err(self.syntax(
            tok,
            format!(
                "mismatched input '{}' expecting reference type",
                token_text(tok)
            ),
        ))
    }

    fn is_type_argument_end(&self) -> bool {
        self.la(GT) || self.la(RIGHSHIFT) || self.la(URSHIFT)
    }

    fn parse_dims(&mut self) -> PResult<Node> {
        let mut brackets = Vec::new();
        loop {
            brackets.push(self.expect(LBRACK, "'['")?);
            brackets.push(self.expect(RBRACK, "']'")?);
            if !(self.la(LBRACK) && self.la_at(1, RBRACK)) {
                break;
            }
        }
        Ok(Node::Dims(DimsContext { brackets }))
    }

    fn parse_dim_exprs(&mut self) -> PResult<Node> {
        let mut expressions = Vec::new();
        let mut brackets = Vec::new();
        loop {
            brackets.push(self.expect(LBRACK, "'['")?);
            self.skip_newlines();
            expressions.push(self.parse_expression()?);
            self.skip_newlines();
            brackets.push(self.expect(RBRACK, "']'")?);
            if !(self.la(LBRACK) && !self.la_at(1, RBRACK)) {
                break;
            }
        }
        Ok(Node::DimExprs(DimExprsContext {
            expressions,
            brackets,
        }))
    }

    fn parse_op_id(&mut self) -> PResult<Node> {
        if !is_op_id_token(self.lt().token_type()) {
            let tok = self.lt();
            return Err(self.syntax(
                tok,
                format!("mismatched input '{}' expecting operator", token_text(tok)),
            ));
        }
        Ok(Node::OpId(OpIdContext {
            token: self.consume_node(),
        }))
    }

    fn parse_var_id(&mut self) -> PResult<Node> {
        if !is_var_id_token(self.lt().token_type()) {
            let tok = self.lt();
            return Err(self.syntax(
                tok,
                format!(
                    "mismatched input '{}' expecting identifier",
                    token_text(tok)
                ),
            ));
        }
        Ok(Node::VarId(VarIdContext {
            token: self.consume_node(),
        }))
    }
}

// ---------------------------------------------------------------------------
// Newline handling, lookaheads and token classification (Java private helpers)
// ---------------------------------------------------------------------------

impl<'a> QLParser<'a> {
    fn consume_next_statement(&mut self) -> PResult<()> {
        if self.la(EOF) || self.la(RBRACE) {
            return Ok(());
        }
        if self.la(SEMI) {
            self.consume();
            return Ok(());
        }
        if self.strict_new_lines {
            self.expect(NEWLINE, "NEWLINE")?;
            while self.la(NEWLINE) {
                self.consume();
            }
        } else if self.la(NEWLINE) {
            while self.la(NEWLINE) {
                self.consume();
            }
        }
        Ok(())
    }

    fn is_next_statement_start(&self) -> bool {
        self.la(EOF)
            || self.la(RBRACE)
            || self.la(SEMI)
            || (self.strict_new_lines && self.la(NEWLINE))
    }

    fn skip_newlines(&mut self) {
        while self.la(NEWLINE) {
            self.consume();
        }
    }

    fn is_lambda_start(&self) -> bool {
        if is_var_id_token(self.lt().token_type()) && self.la_at(1, ARROW) {
            return true;
        }
        if !self.la(LPAREN) {
            return false;
        }
        let close = self.find_matching_paren(self.p);
        close >= 0 && self.token(close as usize + 1).token_type() == ARROW
    }

    fn find_matching_paren(&self, start: usize) -> i64 {
        let mut depth = 0i64;
        for (i, tok) in self.tokens.iter().enumerate().skip(start) {
            let ty = tok.token_type();
            if ty == LPAREN {
                depth += 1;
            } else if ty == RPAREN {
                depth -= 1;
                if depth == 0 {
                    return i as i64;
                }
            }
        }
        -1
    }

    fn is_primary_start(&self, tok: &Token) -> bool {
        let ty = tok.token_type();
        is_literal_start(ty)
            || ty == LPAREN
            || ty == NEW
            || is_var_id_token(ty)
            || is_primitive_type(ty)
            || ty == LBRACK
            || ty == LBRACE
            || ty == SELECTOR_START
            || ty == IF
            || ty == SWITCH
            || ty == TRY
            || self.is_prefix_operator(tok)
    }

    fn is_non_expression_statement_start(&mut self) -> PResult<bool> {
        if self.la(THROW)
            || self.la(WHILE)
            || self.la(FOR)
            || self.la(FUNCTION)
            || self.la(MACRO)
            || self.la(BREAK)
            || self.la(CONTINUE)
            || self.la(RETURN)
            || self.la(SEMI)
            || self.la(NEWLINE)
        {
            return Ok(true);
        }
        Ok(self.try_parse_local_variable_declaration()?.is_some())
    }

    fn single_var_before(&self, end_types: &[i32]) -> bool {
        if !is_var_id_token(self.lt().token_type()) {
            return false;
        }
        end_types.iter().any(|&end| self.la_at(1, end))
    }

    fn single_for_each_var_before_colon(&self) -> bool {
        if !is_var_id_token(self.lt().token_type()) {
            return false;
        }
        let mut i = self.p + 1;
        while self.token(i).token_type() == NEWLINE {
            i += 1;
        }
        self.token(i).token_type() == COLON
    }

    /// Java `isMapExprAhead`: speculative, catches all errors.
    fn is_map_expr_ahead(&mut self) -> bool {
        let save = self.p;
        let result = (|| -> PResult<bool> {
            self.expect(LBRACE, "'{'")?;
            self.skip_newlines();
            if self.la(COLON) {
                return Ok(true);
            }
            if !is_id_map_key(self.lt().token_type())
                && !self.la(QUOTE_STRING_LITERAL)
                && !self.la(DOUBLE_QUOTE)
            {
                return Ok(false);
            }
            if self.la(DOUBLE_QUOTE) {
                self.parse_double_quote_string_literal()?;
            } else {
                self.consume();
            }
            self.skip_newlines();
            Ok(self.la(COLON))
        })();
        self.p = save;
        matches!(result, Ok(true))
    }

    fn has_top_level_assign_operator_ahead(&self) -> bool {
        let mut depth = 0i32;
        for i in self.p..self.tokens.len() {
            let ty = self.tokens[i].token_type();
            if depth == 0 {
                if is_assign_operator(ty) {
                    return true;
                }
                if ty == EOF
                    || ty == SEMI
                    || ty == COMMA
                    || ty == RPAREN
                    || ty == RBRACK
                    || ty == RBRACE
                    || (self.strict_new_lines && ty == NEWLINE)
                {
                    return false;
                }
            }
            if ty == LPAREN || ty == LBRACK || ty == LBRACE {
                depth += 1;
            } else if ty == RPAREN || ty == RBRACK || ty == RBRACE {
                if depth == 0 {
                    return false;
                }
                depth -= 1;
            }
        }
        false
    }

    // ------------------------------------------------------------------
    // Operator manager queries (Java isMiddleOperator etc.)
    // ------------------------------------------------------------------

    fn is_middle_operator(&self, tok: &Token) -> bool {
        self.operator_manager
            .map(|m| m.is_op_type(tok.text(), OpType::Middle))
            .unwrap_or(false)
    }

    fn is_prefix_operator(&self, tok: &Token) -> bool {
        self.operator_manager
            .map(|m| m.is_op_type(tok.text(), OpType::Prefix))
            .unwrap_or(false)
    }

    fn is_suffix_operator(&self, tok: &Token) -> bool {
        self.operator_manager
            .map(|m| m.is_op_type(tok.text(), OpType::Suffix))
            .unwrap_or(false)
    }

    fn is_group_operator(&self, tok: &Token) -> bool {
        is_op_id_token(tok.token_type())
            && self.is_middle_operator(tok)
            && self.precedence(tok) == ql_precedences::GROUP
    }

    fn precedence(&self, tok: &Token) -> i32 {
        self.operator_manager
            .and_then(|m| m.precedence(tok.text()))
            .unwrap_or(-1)
    }
}

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
mod tests {
    use super::*;
    use crate::aparser::check_visitor::CheckVisitor;
    use crate::aparser::import_manager::ImportManager;
    use crate::aparser::parser_operator_manager::{OpType, ParserOperatorManager};
    use crate::aparser::{OutFunctionVisitor, OutVarAttrsVisitor, OutVarNamesVisitor};
    use crate::operator::operator_check_strategy::OperatorCheckStrategy;
    use crate::ql_precedences as prec;

    /// Default operator table for parser tests, mirroring the built-ins of
    /// Java `OperatorManager` (`DEFAULT_BINARY_OPERATOR_MAP` etc.).
    struct DefaultOps;

    impl ParserOperatorManager for DefaultOps {
        fn is_op_type(&self, lexeme: &str, op_type: OpType) -> bool {
            self.precedence_typed(lexeme, op_type).is_some()
        }

        fn precedence(&self, lexeme: &str) -> Option<i32> {
            self.precedence_typed(lexeme, OpType::Middle)
        }

        fn get_alias(&self, _lexeme: &str) -> Option<i32> {
            // Java `OperatorManager.keyWordAliases` starts empty: keyword
            // aliases are user-registered only.
            None
        }
    }

    impl DefaultOps {
        fn precedence_typed(&self, lexeme: &str, op_type: OpType) -> Option<i32> {
            let middle = [
                ("=", prec::ASSIGN),
                ("+=", prec::ASSIGN),
                ("-=", prec::ASSIGN),
                ("*=", prec::ASSIGN),
                ("/=", prec::ASSIGN),
                ("%=", prec::ASSIGN),
                ("&=", prec::ASSIGN),
                ("|=", prec::ASSIGN),
                ("^=", prec::ASSIGN),
                ("<<=", prec::ASSIGN),
                (">>=", prec::ASSIGN),
                (">>>=", prec::ASSIGN),
                ("||", prec::OR),
                ("or", prec::OR),
                ("&&", prec::AND),
                ("and", prec::AND),
                ("|", prec::BIT_OR),
                ("^", prec::XOR),
                ("&", prec::BIT_AND),
                ("==", prec::EQUAL),
                ("!=", prec::EQUAL),
                ("<>", prec::EQUAL),
                ("<", prec::COMPARE),
                ("<=", prec::COMPARE),
                (">", prec::COMPARE),
                (">=", prec::COMPARE),
                ("instanceof", prec::COMPARE),
                ("<<", prec::BIT_MOVE),
                (">>", prec::BIT_MOVE),
                (">>>", prec::BIT_MOVE),
                ("in", prec::IN_LIKE),
                ("like", prec::IN_LIKE),
                ("+", prec::ADD),
                ("-", prec::ADD),
                ("*", prec::MULTI),
                ("/", prec::MULTI),
                ("%", prec::MULTI),
                // Custom-path operator (group precedence), registered like
                // Java users do with `addOperator(".*", ...)`.
                (".*", prec::GROUP),
            ];
            let prefix = ["!", "~", "+", "-", "++", "--"];
            let suffix = ["++", "--"];
            match op_type {
                OpType::Middle => middle.iter().find(|(op, _)| *op == lexeme).map(|(_, p)| *p),
                OpType::Prefix => {
                    if prefix.contains(&lexeme) {
                        Some(prec::UNARY)
                    } else {
                        None
                    }
                }
                OpType::Suffix => {
                    if suffix.contains(&lexeme) {
                        Some(prec::UNARY_SUFFIX)
                    } else {
                        None
                    }
                }
            }
        }
    }

    fn parse(script: &str) -> Node {
        build_tree(
            script,
            Some(&DefaultOps),
            false,
            |_| {},
            InterpolationMode::Script,
            "${",
            "}",
            true,
        )
        .unwrap_or_else(|e| panic!("parse failed for {script:?}: {}", e.reason()))
    }

    fn parse_err(script: &str) -> QLSyntaxException {
        match build_tree(
            script,
            Some(&DefaultOps),
            false,
            |_| {},
            InterpolationMode::Script,
            "${",
            "}",
            true,
        ) {
            Ok(_) => panic!("expected syntax error for {script:?}"),
            Err(e) => e,
        }
    }

    /// Unwrap the top-level statements of a program.
    fn statements(tree: &Node) -> &[Node] {
        match tree {
            Node::Program(program) => match program.block_statements.as_deref() {
                Some(Node::BlockStatements(block)) => &block.statements,
                _ => &[],
            },
            _ => panic!("expected program"),
        }
    }

    fn expr_statement(stmt: &Node) -> &ExpressionContext {
        match stmt {
            Node::ExpressionStatement(s) => match s.expression.as_ref() {
                Node::Expression(e) => e,
                other => panic!("expected expression, got {other:?}"),
            },
            other => panic!("expected expression statement, got {other:?}"),
        }
    }

    /// Unwrap Expression -> Ternary -> BaseExpr (no assign, no ? :).
    fn base_expr(expr: &ExpressionContext) -> &BaseExprContext {
        let ternary = expr.ternary.as_deref().expect("ternary");
        match ternary {
            Node::TernaryExpr(t) => match t.condition.as_ref() {
                Node::BaseExpr(b) => b,
                other => panic!("expected base expr, got {other:?}"),
            },
            other => panic!("expected ternary, got {other:?}"),
        }
    }

    fn primary_of(base: &BaseExprContext) -> &PrimaryContext {
        match base.primary.as_ref() {
            Node::Primary(p) => p,
            other => panic!("expected primary, got {other:?}"),
        }
    }

    fn literal_of(expr: &ExpressionContext) -> &LiteralContext {
        match primary_of(base_expr(expr)).pathable.as_deref() {
            Some(Node::ConstExpr(constant)) => match constant.literal.as_ref() {
                Node::Literal(literal) => literal,
                other => panic!("expected literal, got {other:?}"),
            },
            other => panic!("expected constant expression, got {other:?}"),
        }
    }

    fn binaryop_text(left_asso: &Node) -> &str {
        match left_asso {
            Node::LeftAsso(l) => match l.binaryop.as_ref() {
                Node::Binaryop(op) => op.token.text(),
                other => panic!("expected binaryop, got {other:?}"),
            },
            other => panic!("expected left asso, got {other:?}"),
        }
    }

    /// `SOURCE_PARITY`：Java `RuleContext#getText()` 会拼接
    /// `expectInto/consumeNode` 写入的全部标点，同时不包含
    /// `consumeNextStatement` 和 import 路径中以普通 `consume` 跳过的 token。
    #[test]
    fn rule_context_text_preserves_exact_java_children() {
        let cases = [
            ("while (a) { b = [1, 2,]; }", "while(a){b=[1,2,]}"),
            (
                "function f(int a, b) { return a ? b : 0; }",
                "functionf(inta,b){returna?b:0}",
            ),
            ("x = {'a': 1, b: 2,};", "x={'a':1,b:2,}"),
            ("x = new Foo(1, 2).bar();", "x=newFoo(1,2).bar()"),
            (
                "try { x = 1; } catch (A | B e) { x = 2; } finally { x = 3; }",
                "try{x=1}catch(A|Be){x=2}finally{x=3}",
            ),
            ("f = (a, b) -> { return a; };", "f=(a,b)->{returna}"),
            ("x = \"${a}\";", "x=\"${a}\""),
            ("import a.b.C; x = 1;", "importabC;x=1"),
        ];

        for (script, expected) in cases {
            assert_eq!(parse(script).text(), expected, "script: {script}");
        }
    }

    /// `SOURCE_PARITY`：覆盖 Java `RuleContext#isEmpty/getChildCount/getChild/
    /// getRuleContexts/tokenNode/tokenNodes/getStart/getStop/toStringTree`。
    /// Rust 解析器在构造强类型 AST 时完成 Java `addChild/addToken/setStart/
    /// setStop` 的职责，所有查询必须观察到完整的括号和规则孩子。
    #[test]
    fn rule_context_queries_include_punctuation_and_bounds() {
        let tree = parse("(a);");
        let expression = expr_statement(&statements(&tree)[0]);
        let group = match primary_of(base_expr(expression)).pathable.as_deref() {
            Some(Node::GroupExpr(group)) => group,
            other => panic!("expected group expression, got {other:?}"),
        };
        let group_node = Node::GroupExpr(group.clone());

        assert!(!group_node.is_empty());
        assert_eq!(group_node.child_count(), 3);
        assert_eq!(group_node.child(0).text(), "(");
        assert_eq!(group_node.child(2).text(), ")");
        assert_eq!(
            group_node
                .rule_child(0, |_| true)
                .map(Node::text)
                .as_deref(),
            Some("a")
        );
        assert_eq!(group_node.rule_contexts(|_| true).len(), 1);
        assert_eq!(
            group_node.token_node(LPAREN).map(TerminalNode::text),
            Some("(")
        );
        assert_eq!(group_node.token_nodes(LPAREN).len(), 1);
        assert_eq!(group_node.token_nodes(RPAREN).len(), 1);
        assert_eq!(group_node.start_token().map(Token::text), Some("("));
        assert_eq!(group_node.stop_token().map(Token::text), Some(")"));
        assert!(group_node.to_string_tree().starts_with("(GroupExpr ("));
    }

    // ------------------------------------------------------------------
    // Statements
    // ------------------------------------------------------------------

    #[test]
    fn parses_while_statement() {
        let tree = parse("while (i < 10) { i = i + 1; }");
        match &statements(&tree)[0] {
            Node::WhileStatement(w) => {
                assert_eq!(w.while_token.text(), "while");
                assert!(matches!(
                    w.block_statements.as_deref(),
                    Some(Node::BlockStatements(_))
                ));
            }
            other => panic!("expected while, got {other:?}"),
        }
    }

    #[test]
    fn parses_traditional_for() {
        let tree = parse("for (int i = 0; i < 10; i = i + 1) { sum = sum + i; }");
        match &statements(&tree)[0] {
            Node::TraditionalForStatement(f) => {
                assert!(matches!(
                    f.for_init.as_ref(),
                    Node::ForInit(init) if init.local_variable_declaration.is_some()
                ));
                assert!(f.for_condition.is_some());
                assert!(f.for_update.is_some());
            }
            other => panic!("expected for, got {other:?}"),
        }
    }

    #[test]
    fn parses_for_each_with_and_without_type() {
        let tree = parse("for (int x : xs) { s = s + x; }");
        match &statements(&tree)[0] {
            Node::ForEachStatement(f) => {
                assert!(f.decl_type.is_some());
                assert_eq!(f.var_id.text(), "x");
            }
            other => panic!("expected foreach, got {other:?}"),
        }
        let tree = parse("for (x : xs) { s = s + x; }");
        match &statements(&tree)[0] {
            Node::ForEachStatement(f) => assert!(f.decl_type.is_none()),
            other => panic!("expected foreach, got {other:?}"),
        }
    }

    #[test]
    fn parses_function_and_macro() {
        let tree = parse("function add(int a, int b) { return a + b; }");
        match &statements(&tree)[0] {
            Node::FunctionStatement(f) => {
                assert_eq!(f.var_id.text(), "add");
                match f.params.as_deref() {
                    Some(Node::FormalOrInferredParameterList(list)) => {
                        assert_eq!(list.params.len(), 2)
                    }
                    other => panic!("expected params, got {other:?}"),
                }
            }
            other => panic!("expected function, got {other:?}"),
        }
        let tree = parse("macro inc { a = a + 1; }");
        match &statements(&tree)[0] {
            Node::MacroStatement(m) => assert_eq!(m.var_id.text(), "inc"),
            other => panic!("expected macro, got {other:?}"),
        }
    }

    #[test]
    fn parses_throw_break_continue_return() {
        let tree = parse("throw 'err';");
        assert!(matches!(statements(&tree)[0], Node::ThrowStatement(_)));
        let tree = parse("while (true) { break; continue; }");
        match &statements(&tree)[0] {
            Node::WhileStatement(w) => {
                let body = block_statements(w.block_statements.as_deref().unwrap());
                assert!(matches!(body[0], Node::BreakContinueStatement(ref b) if b.is_break()));
                assert!(matches!(body[1], Node::BreakContinueStatement(ref b) if !b.is_break()));
            }
            other => panic!("expected while, got {other:?}"),
        }
        let tree = parse("return 1;");
        match &statements(&tree)[0] {
            Node::ReturnStatement(r) => assert!(r.expression.is_some()),
            other => panic!("expected return, got {other:?}"),
        }
        let tree = parse("return;");
        match &statements(&tree)[0] {
            Node::ReturnStatement(r) => assert!(r.expression.is_none()),
            other => panic!("expected return, got {other:?}"),
        }
    }

    // statements of a BlockStatements node
    fn block_statements(block: &Node) -> &[Node] {
        match block {
            Node::BlockStatements(b) => &b.statements,
            other => panic!("expected block statements, got {other:?}"),
        }
    }

    #[test]
    fn parses_local_variable_declaration_with_type() {
        let tree = parse("int a = 1, b = 2;");
        match &statements(&tree)[0] {
            Node::LocalVariableDeclarationStatement(s) => {
                match s.local_variable_declaration.as_ref() {
                    Node::LocalVariableDeclaration(decl) => {
                        match decl.variable_declarator_list.as_ref() {
                            Node::VariableDeclaratorList(list) => {
                                assert_eq!(list.variables.len(), 2);
                                match &list.variables[0] {
                                    Node::VariableDeclarator(declarator) => {
                                        assert!(matches!(
                                            declarator.id.as_ref(),
                                            Node::VariableDeclaratorId(_)
                                        ));
                                        assert!(matches!(
                                            declarator.initializer.as_deref(),
                                            Some(Node::VariableInitializer(_))
                                        ));
                                    }
                                    other => {
                                        panic!("expected variable declarator, got {other:?}")
                                    }
                                }
                            }
                            other => panic!("expected declarator list, got {other:?}"),
                        }
                    }
                    other => panic!("expected local decl, got {other:?}"),
                }
            }
            other => panic!("expected local decl statement, got {other:?}"),
        }
    }

    #[test]
    fn var_declaration_without_type_is_assign_or_ref() {
        // `a = 1` is an assignment expression, not a declaration.
        let tree = parse("a = 1;");
        let expr = expr_statement(&statements(&tree)[0]);
        assert!(expr.is_assign());
        // `a.b = 1` assigns through a path.
        let tree = parse("a.b = 1;");
        let expr = expr_statement(&statements(&tree)[0]);
        match expr.left.as_deref() {
            Some(Node::LeftHandSide(l)) => assert_eq!(l.path_parts.len(), 1),
            other => panic!("expected left hand side, got {other:?}"),
        }
    }

    #[test]
    fn parses_if_else_chain() {
        let tree = parse("if (a > 1) { x = 1; } else if (a > 0) { x = 2; } else { x = 3; }");
        let expr = expr_statement(&statements(&tree)[0]);
        let primary = primary_of(base_expr(expr));
        match primary.non_pathable.as_deref() {
            Some(Node::QlIf(ql_if)) => {
                assert!(ql_if.else_body.is_some());
                match ql_if.else_body.as_deref() {
                    Some(Node::ElseBody(e)) => {
                        assert!(matches!(e.ql_if.as_deref(), Some(Node::QlIf(_))));
                    }
                    other => panic!("expected else body, got {other:?}"),
                }
            }
            other => panic!("expected ql if, got {other:?}"),
        }
    }

    #[test]
    fn parses_if_then_expression_body() {
        let tree = parse("if (a) then x = 1 else x = 2;");
        let expr = expr_statement(&statements(&tree)[0]);
        let primary = primary_of(base_expr(expr));
        match primary.non_pathable.as_deref() {
            Some(Node::QlIf(ql_if)) => {
                assert!(ql_if.then_keyword.is_some());
                assert!(matches!(
                    ql_if.then_body.as_ref(),
                    Node::ThenBody(t) if t.expression.is_some()
                ));
            }
            other => panic!("expected ql if, got {other:?}"),
        }
    }

    #[test]
    fn parses_switch_statement_and_expr_groups() {
        let tree = parse("switch (x) { case 1: a = 1; break; default: a = 0; }");
        let expr = expr_statement(&statements(&tree)[0]);
        let primary = primary_of(base_expr(expr));
        match primary.non_pathable.as_deref() {
            Some(Node::SwitchExpr(s)) => match s.groups.as_deref() {
                Some(Node::SwitchCaseGroups(groups)) => {
                    assert_eq!(groups.groups.len(), 2);
                    match &groups.groups[0] {
                        Node::SwitchStatementGroup(group) => {
                            match group.labels.as_ref() {
                                Node::SwitchLabels(labels) => {
                                    assert_eq!(labels.labels.len(), 1)
                                }
                                other => panic!("expected switch labels, got {other:?}"),
                            }
                            assert!(matches!(
                                group.block_statements.as_deref(),
                                Some(Node::BlockStatements(_))
                            ));
                        }
                        other => panic!("expected statement group, got {other:?}"),
                    }
                }
                other => panic!("expected groups, got {other:?}"),
            },
            other => panic!("expected switch, got {other:?}"),
        }

        let tree = parse("y = switch (x) { case 1 -> 10\n case 2, 3 -> 20\n default -> 0\n };");
        let expr = expr_statement(&statements(&tree)[0]);
        assert!(expr.is_assign());
        let rhs = match expr.expression.as_deref() {
            Some(Node::Expression(e)) => e,
            other => panic!("expected rhs, got {other:?}"),
        };
        let primary = primary_of(base_expr(rhs));
        match primary.non_pathable.as_deref() {
            Some(Node::SwitchExpr(s)) => match s.groups.as_deref() {
                Some(Node::SwitchCaseGroups(groups)) => {
                    assert_eq!(groups.groups.len(), 3);
                    match &groups.groups[0] {
                        Node::SwitchExprGroup(group) => {
                            assert!(matches!(
                                group.label.as_ref(),
                                Node::SwitchExpressionLabel(_)
                            ));
                            assert!(matches!(group.expression.as_ref(), Node::Expression(_)));
                        }
                        other => panic!("expected expression group, got {other:?}"),
                    }
                    assert!(matches!(groups.groups[2], Node::SwitchExprGroup(_)));
                }
                other => panic!("expected groups, got {other:?}"),
            },
            other => panic!("expected switch, got {other:?}"),
        }
    }

    #[test]
    fn parses_try_catch_finally() {
        let tree = parse("try { a(); } catch (IOException | RuntimeException e) { b(); } catch (e) { c(); } finally { d(); }");
        let expr = expr_statement(&statements(&tree)[0]);
        let primary = primary_of(base_expr(expr));
        match primary.non_pathable.as_deref() {
            Some(Node::TryCatchExpr(t)) => {
                match t.try_catches.as_deref() {
                    Some(Node::TryCatches(catches)) => {
                        assert_eq!(catches.catches.len(), 2);
                        match &catches.catches[0] {
                            Node::TryCatch(c) => match c.catch_params.as_ref() {
                                Node::CatchParams(p) => assert_eq!(p.decl_types.len(), 2),
                                other => panic!("expected catch params, got {other:?}"),
                            },
                            other => panic!("expected catch, got {other:?}"),
                        }
                        match &catches.catches[1] {
                            Node::TryCatch(c) => match c.catch_params.as_ref() {
                                Node::CatchParams(p) => assert!(p.decl_types.is_empty()),
                                other => panic!("expected catch params, got {other:?}"),
                            },
                            other => panic!("expected catch, got {other:?}"),
                        }
                    }
                    other => panic!("expected catches, got {other:?}"),
                }
                assert!(t.try_finally.is_some());
            }
            other => panic!("expected try, got {other:?}"),
        }
    }

    #[test]
    fn parses_imports() {
        let tree = parse("import java.util.HashMap;\nimport java.io.*;\nx = 1;");
        match &tree {
            Node::Program(program) => {
                assert_eq!(program.imports.len(), 2);
                assert!(matches!(program.imports[0], Node::ImportCls(_)));
                assert!(matches!(program.imports[1], Node::ImportPack(_)));
                if let Node::ImportCls(import) = &program.imports[0] {
                    let names: Vec<String> = import.var_ids.iter().map(|id| id.text()).collect();
                    assert_eq!(names, ["java", "util", "HashMap"]);
                }
            }
            other => panic!("expected program, got {other:?}"),
        }
    }

    #[test]
    fn import_not_at_beginning_is_error() {
        let err = parse_err("x = 1;\nimport java.util.HashMap;");
        assert_eq!(err.error_code(), error_codes::SYNTAX_ERROR);
    }

    // ------------------------------------------------------------------
    // Expressions: precedence and forms
    // ------------------------------------------------------------------

    #[test]
    fn multiplication_binds_tighter_than_plus() {
        let tree = parse("1 + 2 * 3;");
        let base = base_expr(expr_statement(&statements(&tree)[0]));
        // base: primary=1, leftAssos=[ + (2*3) ]
        assert_eq!(base.left_assos.len(), 1);
        assert_eq!(binaryop_text(&base.left_assos[0]), "+");
        match &base.left_assos[0] {
            Node::LeftAsso(l) => match l.right.as_ref() {
                Node::BaseExpr(right) => {
                    assert_eq!(right.left_assos.len(), 1);
                    assert_eq!(binaryop_text(&right.left_assos[0]), "*");
                }
                other => panic!("expected right base expr, got {other:?}"),
            },
            other => panic!("expected left asso, got {other:?}"),
        }
    }

    #[test]
    fn comparison_binds_looser_than_add() {
        // a + 1 < b * 2  ->  base(a) assos:[ + 1, < (b*2) ]
        let tree = parse("a + 1 < b * 2;");
        let base = base_expr(expr_statement(&statements(&tree)[0]));
        assert_eq!(base.left_assos.len(), 2);
        assert_eq!(binaryop_text(&base.left_assos[0]), "+");
        assert_eq!(binaryop_text(&base.left_assos[1]), "<");
        match &base.left_assos[1] {
            Node::LeftAsso(l) => match l.right.as_ref() {
                Node::BaseExpr(right) => assert_eq!(binaryop_text(&right.left_assos[0]), "*"),
                other => panic!("expected right base expr, got {other:?}"),
            },
            _other => panic!(),
        }
    }

    #[test]
    fn ternary_parses_right_associative_tail() {
        let tree = parse("a ? b : c ? d : e;");
        let expr = expr_statement(&statements(&tree)[0]);
        match expr.ternary.as_deref() {
            Some(Node::TernaryExpr(t)) => {
                assert!(t.question.is_some());
                assert!(t.then_expr.is_some());
                // else branch is a full expression containing another ternary
                match t.else_expr.as_deref() {
                    Some(Node::Expression(inner)) => {
                        assert!(matches!(
                            inner.ternary.as_deref(),
                            Some(Node::TernaryExpr(it)) if it.question.is_some()
                        ));
                    }
                    other => panic!("expected else expr, got {other:?}"),
                }
            }
            other => panic!("expected ternary, got {other:?}"),
        }
    }

    #[test]
    fn assignment_is_right_associative() {
        let tree = parse("a = b = 1;");
        let expr = expr_statement(&statements(&tree)[0]);
        assert!(expr.is_assign());
        match expr.expression.as_deref() {
            Some(Node::Expression(rhs)) => assert!(rhs.is_assign()),
            other => panic!("expected nested assign, got {other:?}"),
        }
    }

    #[test]
    fn unary_prefix_and_suffix() {
        let tree = parse("x = -a++ + !b;");
        let expr = expr_statement(&statements(&tree)[0]);
        let rhs = match expr.expression.as_deref() {
            Some(Node::Expression(e)) => e,
            other => panic!("expected rhs, got {other:?}"),
        };
        let base = base_expr(rhs);
        let primary = primary_of(base);
        assert!(matches!(
            primary.prefix.as_deref(),
            Some(Node::PrefixExpress(_))
        ));
        assert!(matches!(
            primary.suffix.as_deref(),
            Some(Node::SuffixExpress(_))
        ));
        assert_eq!(base.left_assos.len(), 1);
        assert_eq!(binaryop_text(&base.left_assos[0]), "+");
    }

    #[test]
    fn parses_cast_group_and_type_expr() {
        let tree = parse("x = (int) 3.5;");
        let expr = expr_statement(&statements(&tree)[0]);
        let rhs = match expr.expression.as_deref() {
            Some(Node::Expression(e)) => e,
            _other => panic!(),
        };
        let primary = primary_of(base_expr(rhs));
        assert!(matches!(
            primary.pathable.as_deref(),
            Some(Node::CastExpr(_))
        ));

        let tree = parse("x = (1 + 2) * 3;");
        let expr = expr_statement(&statements(&tree)[0]);
        let rhs = match expr.expression.as_deref() {
            Some(Node::Expression(e)) => e,
            _other => panic!(),
        };
        let base = base_expr(rhs);
        let primary = primary_of(base);
        assert!(matches!(
            primary.pathable.as_deref(),
            Some(Node::GroupExpr(_))
        ));
        assert_eq!(binaryop_text(&base.left_assos[0]), "*");
    }

    #[test]
    fn parses_new_expressions() {
        let tree = parse("m = new java.util.HashMap();");
        let expr = expr_statement(&statements(&tree)[0]);
        let rhs = match expr.expression.as_deref() {
            Some(Node::Expression(e)) => e,
            _other => panic!(),
        };
        let primary = primary_of(base_expr(rhs));
        match primary.pathable.as_deref() {
            Some(Node::NewObjExpr(new_obj)) => {
                let names: Vec<String> = new_obj.var_ids.iter().map(|id| id.text()).collect();
                assert_eq!(names, ["java", "util", "HashMap"]);
            }
            other => panic!("expected new obj, got {other:?}"),
        }

        let tree = parse("a = new int[10];");
        let expr = expr_statement(&statements(&tree)[0]);
        let rhs = match expr.expression.as_deref() {
            Some(Node::Expression(e)) => e,
            _other => panic!(),
        };
        assert!(matches!(
            primary_of(base_expr(rhs)).pathable.as_deref(),
            Some(Node::NewEmptyArrExpr(_))
        ));

        let tree = parse("a = new int[] {1, 2, 3};");
        let expr = expr_statement(&statements(&tree)[0]);
        let rhs = match expr.expression.as_deref() {
            Some(Node::Expression(e)) => e,
            _other => panic!(),
        };
        assert!(matches!(
            primary_of(base_expr(rhs)).pathable.as_deref(),
            Some(Node::NewInitArrExpr(_))
        ));
    }

    #[test]
    fn primitive_new_object_is_error() {
        let err = parse_err("x = new int();");
        assert_eq!(err.error_code(), error_codes::SYNTAX_ERROR);
    }

    #[test]
    fn parses_list_map_and_block_expr() {
        let tree = parse("x = [1, 2, 3];");
        let expr = expr_statement(&statements(&tree)[0]);
        let rhs = match expr.expression.as_deref() {
            Some(Node::Expression(e)) => e,
            _other => panic!(),
        };
        match primary_of(base_expr(rhs)).pathable.as_deref() {
            Some(Node::ListExpr(list)) => match list.list_items.as_deref() {
                Some(Node::ListItems(items)) => assert_eq!(items.expressions.len(), 3),
                other => panic!("expected list items, got {other:?}"),
            },
            other => panic!("expected list, got {other:?}"),
        }

        let tree = parse("x = {'a': 1, b: 2};");
        let expr = expr_statement(&statements(&tree)[0]);
        let rhs = match expr.expression.as_deref() {
            Some(Node::Expression(e)) => e,
            _other => panic!(),
        };
        match primary_of(base_expr(rhs)).pathable.as_deref() {
            Some(Node::MapExpr(map)) => match map.map_entries.as_ref() {
                Node::MapEntries(entries) => assert_eq!(entries.entries.len(), 2),
                other => panic!("expected entries, got {other:?}"),
            },
            other => panic!("expected map, got {other:?}"),
        }

        // Double-quoted StringKey and the special '@class' ClsValue accessor.
        let tree = parse(r#"x = {"name": 1, '@class': 'java.lang.String'};"#);
        let expr = expr_statement(&statements(&tree)[0]);
        let rhs = match expr.expression.as_deref() {
            Some(Node::Expression(e)) => e,
            _other => panic!(),
        };
        match primary_of(base_expr(rhs)).pathable.as_deref() {
            Some(Node::MapExpr(map)) => match map.map_entries.as_ref() {
                Node::MapEntries(entries) => {
                    assert_eq!(entries.entries.len(), 2);
                    match &entries.entries[0] {
                        Node::MapEntry(entry) => assert!(matches!(
                            entry.map_key.as_ref(),
                            Node::StringKey(StringKeyContext {
                                double_quote_string,
                            }) if matches!(double_quote_string.as_ref(), Node::DoubleQuoteStringLiteral(_))
                        )),
                        other => panic!("expected map entry, got {other:?}"),
                    }
                    match &entries.entries[1] {
                        Node::MapEntry(entry) => assert!(matches!(
                            entry.map_value.as_ref(),
                            Node::ClsValue(ClsValueContext { quote })
                                if quote.text() == "'java.lang.String'"
                        )),
                        other => panic!("expected class map entry, got {other:?}"),
                    }
                }
                other => panic!("expected entries, got {other:?}"),
            },
            other => panic!("expected map, got {other:?}"),
        }

        // empty map literal
        let tree = parse("x = {:};");
        let expr = expr_statement(&statements(&tree)[0]);
        let rhs = match expr.expression.as_deref() {
            Some(Node::Expression(e)) => e,
            _other => panic!(),
        };
        match primary_of(base_expr(rhs)).pathable.as_deref() {
            Some(Node::MapExpr(map)) => match map.map_entries.as_ref() {
                Node::MapEntries(entries) => assert!(entries.empty_colon.is_some()),
                other => panic!("expected entries, got {other:?}"),
            },
            other => panic!("expected map, got {other:?}"),
        }
    }

    #[test]
    fn parses_index_and_slice() {
        let tree = parse("x = a[1];");
        let expr = expr_statement(&statements(&tree)[0]);
        let rhs = match expr.expression.as_deref() {
            Some(Node::Expression(e)) => e,
            _other => panic!(),
        };
        let primary = primary_of(base_expr(rhs));
        match &primary.path_parts[0] {
            Node::IndexExpr(i) => {
                assert!(matches!(
                    i.index_value_expr.as_deref(),
                    Some(Node::SingleIndex(_))
                ));
            }
            other => panic!("expected index, got {other:?}"),
        }

        let tree = parse("x = a[1:3];");
        let expr = expr_statement(&statements(&tree)[0]);
        let rhs = match expr.expression.as_deref() {
            Some(Node::Expression(e)) => e,
            _other => panic!(),
        };
        let primary = primary_of(base_expr(rhs));
        match &primary.path_parts[0] {
            Node::IndexExpr(i) => match i.index_value_expr.as_deref() {
                Some(Node::SliceIndex(s)) => {
                    assert!(s.start.is_some() && s.end.is_some());
                }
                other => panic!("expected slice, got {other:?}"),
            },
            other => panic!("expected index, got {other:?}"),
        }
    }

    #[test]
    fn parses_method_field_and_chaining_paths() {
        let tree = parse("x = a.b().c?.d*.e;");
        let expr = expr_statement(&statements(&tree)[0]);
        let rhs = match expr.expression.as_deref() {
            Some(Node::Expression(e)) => e,
            _other => panic!(),
        };
        let primary = primary_of(base_expr(rhs));
        assert_eq!(primary.path_parts.len(), 4);
        match &primary.path_parts[0] {
            Node::MethodInvoke(m) => assert_eq!(m.chain, ChainKind::Plain),
            other => panic!("expected method invoke, got {other:?}"),
        }
        match &primary.path_parts[1] {
            Node::FieldAccess(f) => assert_eq!(f.chain, ChainKind::Plain),
            other => panic!("expected field access, got {other:?}"),
        }
        match &primary.path_parts[2] {
            Node::FieldAccess(f) => assert_eq!(f.chain, ChainKind::Optional),
            other => panic!("expected optional field, got {other:?}"),
        }
        match &primary.path_parts[3] {
            Node::FieldAccess(f) => assert_eq!(f.chain, ChainKind::Spread),
            other => panic!("expected spread field, got {other:?}"),
        }

        let tree = parse(r"x = a.'display\'name';");
        let expr = expr_statement(&statements(&tree)[0]);
        let rhs = match expr.expression.as_deref() {
            Some(Node::Expression(expression)) => expression,
            other => panic!("expected expression, got {other:?}"),
        };
        let primary = primary_of(base_expr(rhs));
        let field_id = match &primary.path_parts[0] {
            Node::FieldAccess(field) => match field.field_id.as_ref() {
                Node::FieldId(field_id) => field_id,
                other => panic!("expected field id, got {other:?}"),
            },
            other => panic!("expected quoted field access, got {other:?}"),
        };
        assert!(field_id.token.is_none());
        assert_eq!(
            field_id.quote_string_literal().map(TerminalNode::text),
            Some(r"'display\'name'")
        );
    }

    /// `SOURCE_PARITY`：Java `SyntaxTreeFactory#buildTree` 在 debug 模式下
    /// 依次输出 Token 流与 `RuleContext#toStringTree()`，而不是扁平文本。
    #[test]
    fn build_tree_prints_token_stream_and_java_tree_shape() {
        let mut printed = Vec::new();
        let tree = build_tree(
            "a + b;",
            Some(&DefaultOps),
            true,
            |line| printed.push(line),
            InterpolationMode::Script,
            "${",
            "}",
            true,
        )
        .expect("debug parse");

        assert_eq!(printed.len(), 2);
        assert_eq!(printed[0], "a | + | b | ; | <EOF>");
        assert_eq!(printed[1], tree.to_string_tree());
        assert!(printed[1].starts_with("(Program "));
        assert_ne!(printed[1], tree.text());
    }

    #[test]
    fn parses_custom_path() {
        let tree = parse("x = a .* 'path';");
        let expr = expr_statement(&statements(&tree)[0]);
        let rhs = match expr.expression.as_deref() {
            Some(Node::Expression(e)) => e,
            _other => panic!(),
        };
        let primary = primary_of(base_expr(rhs));
        match &primary.path_parts[0] {
            Node::CustomPath(c) => assert_eq!(c.path_text, "path"),
            other => panic!("expected custom path, got {other:?}"),
        }
    }

    #[test]
    fn parses_lambdas() {
        let tree = parse("f = x -> x * 2;");
        let expr = expr_statement(&statements(&tree)[0]);
        let rhs = match expr.expression.as_deref() {
            Some(Node::Expression(e)) => e,
            _other => panic!(),
        };
        let primary = primary_of(base_expr(rhs));
        match primary.non_pathable.as_deref() {
            Some(Node::LambdaExpr(l)) => {
                assert!(matches!(
                    l.lambda_parameters.as_ref(),
                    Node::LambdaParameters(p) if p.var_id.is_some()
                ));
                assert!(l.expression.is_some());
            }
            other => panic!("expected lambda, got {other:?}"),
        }

        let tree = parse("f = (a, b) -> { return a + b; };");
        let expr = expr_statement(&statements(&tree)[0]);
        let rhs = match expr.expression.as_deref() {
            Some(Node::Expression(e)) => e,
            _other => panic!(),
        };
        let primary = primary_of(base_expr(rhs));
        match primary.non_pathable.as_deref() {
            Some(Node::LambdaExpr(l)) => {
                match l.lambda_parameters.as_ref() {
                    Node::LambdaParameters(p) => match p.params.as_deref() {
                        Some(Node::FormalOrInferredParameterList(list)) => {
                            assert_eq!(list.params.len(), 2)
                        }
                        other => panic!("expected params, got {other:?}"),
                    },
                    other => panic!("expected lambda params, got {other:?}"),
                }
                assert!(l.block_statements.is_some());
            }
            other => panic!("expected lambda, got {other:?}"),
        }
    }

    #[test]
    fn parses_string_interpolation() {
        let tree = parse("x = \"a ${y + 1} b\";");
        let expr = expr_statement(&statements(&tree)[0]);
        let rhs = match expr.expression.as_deref() {
            Some(Node::Expression(e)) => e,
            _other => panic!(),
        };
        let primary = primary_of(base_expr(rhs));
        match primary.pathable.as_deref() {
            Some(Node::ConstExpr(c)) => match c.literal.as_ref() {
                Node::Literal(lit) => match lit.double_quote_string.as_deref() {
                    Some(Node::DoubleQuoteStringLiteral(s)) => {
                        // "a " + ${y + 1} + " b"
                        assert!(s.static_characters.is_none());
                        assert_eq!(s.parts.len(), 3);
                        match &s.parts[1] {
                            DyStrPart::Expr(expression) => match expression.as_ref() {
                                Node::StringExpression(string_expression) => {
                                    assert_eq!(string_expression.start.text(), "${");
                                    assert!(string_expression.expression.is_some());
                                    assert!(string_expression.selector_variable.is_none());
                                }
                                other => {
                                    panic!("expected string expression, got {other:?}")
                                }
                            },
                            other => panic!("expected dynamic part, got {other:?}"),
                        }
                    }
                    other => panic!("expected string literal, got {other:?}"),
                },
                other => panic!("expected literal, got {other:?}"),
            },
            other => panic!("expected const, got {other:?}"),
        }

        let tree = build_tree(
            "x = \"${user}\";",
            Some(&DefaultOps),
            false,
            |_| {},
            InterpolationMode::Variable,
            "${",
            "}",
            true,
        )
        .expect("variable interpolation");
        let expression = expr_statement(&statements(&tree)[0])
            .expression
            .as_deref()
            .and_then(|node| match node {
                Node::Expression(expression) => Some(expression),
                _ => None,
            })
            .expect("assignment rhs");
        let literal = literal_of(expression);
        match literal.double_quote_string.as_deref() {
            Some(Node::DoubleQuoteStringLiteral(string)) => match &string.parts[0] {
                DyStrPart::Expr(expression) => match expression.as_ref() {
                    Node::StringExpression(string_expression) => {
                        assert_eq!(
                            string_expression
                                .selector_variable
                                .as_ref()
                                .map(TerminalNode::text),
                            Some("user")
                        );
                        assert!(string_expression.expression.is_none());
                    }
                    other => panic!("expected string expression, got {other:?}"),
                },
                other => panic!("expected dynamic part, got {other:?}"),
            },
            other => panic!("expected double-quoted literal, got {other:?}"),
        }
    }

    /// `SOURCE_PARITY`：Java `LiteralContext` 的四种 token accessor、
    /// `boolenLiteral` 和 `doubleQuoteStringLiteral` 在 Rust 中分别适配为
    /// `token`、`boolen` 与 `double_quote_string` 字段。
    #[test]
    fn literal_context_accessors_preserve_java_variants() {
        for (script, token_type, text) in [
            ("0x1F;", INTEGER_LITERAL, "0x1F"),
            (".5;", FLOATING_POINT_LITERAL, ".5"),
            ("1;", INTEGER_OR_FLOATING_LITERAL, "1"),
            ("'text';", QUOTE_STRING_LITERAL, "'text'"),
        ] {
            let tree = parse(script);
            let literal = literal_of(expr_statement(&statements(&tree)[0]));
            let token = literal.token.as_ref().expect("token literal");
            assert_eq!(token.symbol().token_type(), token_type);
            assert_eq!(token.text(), text);
            assert!(literal.boolen.is_none());
            assert!(literal.double_quote_string.is_none());
        }

        let tree = parse("true;");
        let literal = literal_of(expr_statement(&statements(&tree)[0]));
        assert!(matches!(
            literal.boolen.as_deref(),
            Some(Node::BoolenLiteral(_))
        ));
        assert!(literal.token.is_none());

        // Java 仅在 DISABLE 模式发出 StaticStringCharacters；SCRIPT /
        // VARIABLE 模式使用 DyStrText，即使字符串中没有插值。
        let tree = build_tree(
            "\"plain\";",
            Some(&DefaultOps),
            false,
            |_| {},
            InterpolationMode::Disable,
            "${",
            "}",
            true,
        )
        .expect("disabled interpolation literal");
        let literal = literal_of(expr_statement(&statements(&tree)[0]));
        match literal.double_quote_string.as_deref() {
            Some(Node::DoubleQuoteStringLiteral(string)) => {
                assert_eq!(
                    string.static_characters.as_ref().map(TerminalNode::text),
                    Some("plain")
                );
            }
            other => panic!("expected double-quoted literal, got {other:?}"),
        }
    }

    #[test]
    fn parses_context_selector() {
        let tree = build_tree(
            "$[user]",
            Some(&DefaultOps),
            false,
            |_| {},
            InterpolationMode::Script,
            "$[",
            "]",
            true,
        )
        .unwrap();
        let expr = expr_statement(&statements(&tree)[0]);
        let primary = primary_of(base_expr(expr));
        match primary.pathable.as_deref() {
            Some(Node::ContextSelectExpr(s)) => assert_eq!(s.selector_variable.text(), "user"),
            other => panic!("expected selector, got {other:?}"),
        }
    }

    #[test]
    fn parses_method_reference() {
        let tree = parse("f = String::valueOf;");
        let expr = expr_statement(&statements(&tree)[0]);
        let rhs = match expr.expression.as_deref() {
            Some(Node::Expression(e)) => e,
            _other => panic!(),
        };
        let primary = primary_of(base_expr(rhs));
        assert!(matches!(primary.path_parts[0], Node::MethodAccess(_)));
    }

    #[test]
    fn array_initializer_in_declaration() {
        let tree = parse("int[] a = {1, 2};");
        match &statements(&tree)[0] {
            Node::LocalVariableDeclarationStatement(s) => {
                match s.local_variable_declaration.as_ref() {
                    Node::LocalVariableDeclaration(decl) => {
                        match decl.variable_declarator_list.as_ref() {
                            Node::VariableDeclaratorList(list) => match &list.variables[0] {
                                Node::VariableDeclarator(v) => match v.initializer.as_deref() {
                                    Some(Node::VariableInitializer(init)) => {
                                        match init.array_initializer.as_deref() {
                                            Some(Node::ArrayInitializer(array)) => {
                                                assert_eq!(array.lbrace.text(), "{");
                                                assert_eq!(array.rbrace.text(), "}");
                                                match array.initializers.as_deref() {
                                                    Some(Node::VariableInitializerList(list)) => {
                                                        assert_eq!(list.initializers.len(), 2);
                                                        assert_eq!(list.commas.len(), 1);
                                                    }
                                                    other => panic!(
                                                        "expected initializer list, got {other:?}"
                                                    ),
                                                }
                                            }
                                            other => {
                                                panic!("expected array initializer, got {other:?}")
                                            }
                                        }
                                    }
                                    other => panic!("expected initializer, got {other:?}"),
                                },
                                other => panic!("expected declarator, got {other:?}"),
                            },
                            _other => panic!(),
                        }
                    }
                    _other => panic!(),
                }
            }
            other => panic!("expected local decl, got {other:?}"),
        }
    }

    // ------------------------------------------------------------------
    // Errors: code + line/col
    // ------------------------------------------------------------------

    #[test]
    fn dangling_operator_reports_syntax_error_with_position() {
        let err = parse_err("1 + ;");
        assert_eq!(err.error_code(), error_codes::SYNTAX_ERROR);
        assert_eq!(err.line_no(), 1);
        // ';' is at column 4 (1-based)
        assert_eq!(err.col_no(), 5);
        assert!(err.reason().contains("expecting expression"));
    }

    #[test]
    fn unclosed_paren_reports_eof_position() {
        let err = parse_err("x = (1 + 2;");
        assert_eq!(err.error_code(), error_codes::SYNTAX_ERROR);
        assert!(err.reason().contains("expecting ')'"));
    }

    #[test]
    fn missing_semicolon_after_declaration_errors() {
        let err = parse_err("int a");
        assert_eq!(err.error_code(), error_codes::SYNTAX_ERROR);
        assert_eq!(err.line_no(), 1);
    }

    #[test]
    fn error_line_tracks_newlines() {
        let err = parse_err("a = 1;\nb = ;");
        assert_eq!(err.error_code(), error_codes::SYNTAX_ERROR);
        assert_eq!(err.line_no(), 2);
    }

    // ------------------------------------------------------------------
    // Compile-time visitors
    // ------------------------------------------------------------------

    #[test]
    fn out_var_names_collects_external_reads() {
        let tree = parse("a = b + c.d;");
        let supplier = crate::class_supplier::DefaultClassSupplier::instance();
        let import_manager = ImportManager::new(&supplier, vec![]);
        let mut visitor = OutVarNamesVisitor::new(import_manager);
        tree.accept(&mut visitor);
        assert!(visitor.out_vars().contains("b"));
        assert!(visitor.out_vars().contains("c"));
        assert!(!visitor.out_vars().contains("a"));
    }

    #[test]
    fn out_var_names_compound_assign_counts_as_read() {
        let tree = parse("a += 1;");
        let supplier = crate::class_supplier::DefaultClassSupplier::instance();
        let mut visitor = OutVarNamesVisitor::new(ImportManager::new(&supplier, vec![]));
        tree.accept(&mut visitor);
        assert!(visitor.out_vars().contains("a"));
    }

    #[test]
    fn out_var_names_respects_local_declaration() {
        let tree = parse("int b = 1;\na = b;");
        let supplier = crate::class_supplier::DefaultClassSupplier::instance();
        let mut visitor = OutVarNamesVisitor::new(ImportManager::new(&supplier, vec![]));
        tree.accept(&mut visitor);
        assert!(visitor.out_vars().is_empty());
    }

    #[test]
    fn out_var_names_skips_imported_class_paths() {
        let mut supplier = crate::class_supplier::DefaultClassSupplier::instance();
        supplier.register("java.lang.Math");
        let tree = parse("import java.lang.Math;\nx = Math.max(1, 2);");
        let mut visitor = OutVarNamesVisitor::new(ImportManager::new(&supplier, vec![]));
        tree.accept(&mut visitor);
        assert!(visitor.out_vars().is_empty());
    }

    #[test]
    fn out_var_attrs_collects_attr_paths() {
        let tree = parse("x = a.b.c + a.b[0];");
        let supplier = crate::class_supplier::DefaultClassSupplier::instance();
        let mut visitor = OutVarAttrsVisitor::new(ImportManager::new(&supplier, vec![]));
        tree.accept(&mut visitor);
        assert!(visitor.out_var_attrs().contains(&vec![
            "a".to_string(),
            "b".to_string(),
            "c".to_string()
        ]));
        assert!(visitor
            .out_var_attrs()
            .contains(&vec!["a".to_string(), "b".to_string()]));
    }

    #[test]
    fn out_function_collects_undefined_calls() {
        let tree = parse("function f(a) { g(a); }\nf(1);\nh();");
        let mut visitor = OutFunctionVisitor::new();
        tree.accept(&mut visitor);
        assert!(visitor.out_functions().contains("g"));
        assert!(visitor.out_functions().contains("h"));
        assert!(!visitor.out_functions().contains("f"));
    }

    #[test]
    fn check_visitor_blocks_disallowed_operator() {
        let tree = parse("a = 1 + 2;");
        let options = crate::check_options::CheckOptions::builder()
            .operator_check_strategy(OperatorCheckStrategy::blacklist(
                ["+".to_string()].into_iter().collect(),
            ))
            .build();
        let mut checker = CheckVisitor::new(&options, "a = 1 + 2;");
        let err = checker.check(&tree).unwrap_err();
        assert_eq!(err.error_code(), error_codes::OPERATOR_NOT_ALLOWED);
        assert_eq!(err.line_no(), 1);
    }

    #[test]
    fn check_visitor_can_disable_function_calls() {
        let options = crate::check_options::CheckOptions::builder()
            .disable_function_calls(true)
            .build();
        for script in ["a = f(1);", "a = value?.f();", "a = values*.f();"] {
            let tree = parse(script);
            let mut checker = CheckVisitor::new(&options, script);
            let err = checker.check(&tree).unwrap_err();
            assert_eq!(
                err.error_code(),
                "FUNCTION_CALL_NOT_ALLOWED",
                "script: {script}"
            );
        }
    }

    #[test]
    fn check_visitor_passes_clean_script() {
        let tree = parse("a = 1 + 2;");
        let options = crate::check_options::CheckOptions::builder().build();
        let mut checker = CheckVisitor::new(&options, "a = 1 + 2;");
        assert!(checker.check(&tree).is_ok());
    }
}
