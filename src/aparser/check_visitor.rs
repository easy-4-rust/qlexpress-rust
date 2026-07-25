//! Compile-time script validation, mirroring Java `CheckVisitor`:
//! operator allow-listing and the "function calls disabled" check.
//!
//! Java signals violations by throwing `QLSyntaxException` mid-traversal.
//! The Rust [`Visitor`] trait returns values, so [`CheckVisitor`] records
//! the first violation; call [`CheckVisitor::check`] to get the
//! `Result<(), QLSyntaxException>`.

use super::syntax_tree::*;
use crate::check_options::CheckOptions;
use crate::exception::error_codes;
use crate::exception::ql_syntax_exception::QLSyntaxException;
use crate::exception::QLException;
use crate::operator::operator_check_strategy::OperatorCheckStrategy;
use crate::aparser::token::Token;

/// Java `CheckVisitor`.
pub struct CheckVisitor<'a> {
    operator_check_strategy: &'a OperatorCheckStrategy,
    disable_function_calls: bool,
    script: &'a str,
    /// First recorded violation (Java: the thrown exception).
    violation: Option<QLSyntaxException>,
}

impl<'a> CheckVisitor<'a> {
    /// Java `new CheckVisitor(checkOptions, script)`.
    pub fn new(check_options: &'a CheckOptions, script: &'a str) -> Self {
        CheckVisitor {
            operator_check_strategy: check_options.check_strategy(),
            disable_function_calls: check_options.is_disable_function_calls(),
            script,
            violation: None,
        }
    }

    /// Java `new CheckVisitor(checkOptions)` (empty script for reporting).
    pub fn without_script(check_options: &'a CheckOptions) -> Self {
        Self::new(check_options, "")
    }

    /// Visit `tree`, returning the first violation like Java's throw.
    pub fn check(&mut self, tree: &Node) -> Result<(), QLSyntaxException> {
        tree.accept(self);
        match self.violation.take() {
            Some(violation) => Err(violation),
            None => Ok(()),
        }
    }

    /// Stop traversing once a violation was recorded (Java unwinds via the
    /// exception; Rust short-circuits instead).
    fn failed(&self) -> bool {
        self.violation.is_some()
    }

    /// Java `checkOperator`.
    fn check_operator(&mut self, operator: &str, token: &Token) {
        if self.failed() {
            return;
        }
        if !self.operator_check_strategy.is_allowed(operator) {
            // Java String.format(msg, operatorString, operators): the
            // template has a single %s, so only the operator is substituted
            // (the second Java argument is ignored by String.format).
            let reason = error_codes::format_msg(
                error_codes::error_msg(error_codes::OPERATOR_NOT_ALLOWED),
                &[operator.to_string()],
            );
            self.violation = Some(QLException::report_scanner_err(
                self.script,
                token.start_index(),
                token.line(),
                token.char_position_in_line() + 1,
                operator,
                error_codes::OPERATOR_NOT_ALLOWED,
                &reason,
            ));
        }
    }

    /// Java `checkFunctionCall`.
    fn check_function_call(&mut self, token: &Token) {
        if self.failed() {
            return;
        }
        if self.disable_function_calls {
            let reason = "Function calls are not allowed in this context";
            self.violation = Some(QLException::report_scanner_err(
                self.script,
                token.start_index(),
                token.line(),
                token.char_position_in_line() + 1,
                token.text(),
                "FUNCTION_CALL_NOT_ALLOWED",
                reason,
            ));
        }
    }

    /// Visit children unless a violation was already recorded.
    fn visit_children_unless_failed(&mut self, ctx: &dyn HasChildren) {
        if !self.failed() {
            self.visit_children_of(ctx);
        }
    }
}

/// Java `ctx.getStart()`: first token of the node, falling back to a
/// synthetic EOF-position token when the node is token-less.
fn start_token(node: &Node) -> Token {
    node.start_token()
        .cloned()
        .unwrap_or_else(|| Token::new(crate::aparser::token::EOF, "<EOF>", 0, -1, 1, 0))
}

impl Visitor for CheckVisitor<'_> {
    type T = ();

    fn visit_left_asso(&mut self, ctx: &LeftAssoContext) -> Self::T {
        if self.failed() {
            return;
        }
        if let Node::Binaryop(binaryop) = ctx.binaryop.as_ref() {
            self.check_operator(binaryop.token.text(), binaryop.token.symbol());
        }
        self.visit_children_unless_failed(ctx);
    }

    fn visit_prefix_express(&mut self, ctx: &PrefixExpressContext) -> Self::T {
        if self.failed() {
            return;
        }
        if let Node::OpId(op_id) = ctx.op_id.as_ref() {
            self.check_operator(op_id.token.text(), op_id.token.symbol());
        }
        self.visit_children_unless_failed(ctx);
    }

    fn visit_suffix_express(&mut self, ctx: &SuffixExpressContext) -> Self::T {
        if self.failed() {
            return;
        }
        if let Node::OpId(op_id) = ctx.op_id.as_ref() {
            self.check_operator(op_id.token.text(), op_id.token.symbol());
        }
        self.visit_children_unless_failed(ctx);
    }

    fn visit_expression(&mut self, ctx: &ExpressionContext) -> Self::T {
        if self.failed() {
            return;
        }
        if let Some(Node::AssignOperator(op)) = ctx.assign_operator.as_deref() {
            self.check_operator(op.token.text(), op.token.symbol());
        }
        self.visit_children_unless_failed(ctx);
    }

    fn visit_left_hand_side(&mut self, ctx: &LeftHandSideContext) -> Self::T {
        if self.failed() {
            return;
        }
        if ctx.lparen.is_some() {
            self.check_function_call(&start_token_of_left_hand_side(ctx));
        }
        self.visit_children_unless_failed(ctx);
    }

    fn visit_var_id_expr(&mut self, ctx: &VarIdExprContext) -> Self::T {
        if self.failed() {
            return;
        }
        if ctx.lparen.is_some() {
            self.check_function_call(&start_token(&ctx.var_id));
        }
        self.visit_children_unless_failed(ctx);
    }

    fn visit_method_invoke(&mut self, ctx: &MethodInvokeContext) -> Self::T {
        if self.failed() {
            return;
        }
        self.check_function_call(ctx.dot.symbol());
        self.visit_children_unless_failed(ctx);
    }
}

/// Java `LeftHandSideContext.getStart()` (the head var id token).
fn start_token_of_left_hand_side(ctx: &LeftHandSideContext) -> Token {
    start_token(&ctx.var_id)
}
