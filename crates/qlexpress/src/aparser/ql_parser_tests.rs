//! 从主对象文件机械搬移的聚焦单元测试；测试语义与来源标记保持不变。

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

include!("ql_parser_tests/statements.rs");
include!("ql_parser_tests/expressions.rs");
include!("ql_parser_tests/paths_and_literals.rs");
include!("ql_parser_tests/analysis.rs");
