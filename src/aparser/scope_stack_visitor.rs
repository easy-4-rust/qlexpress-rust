//! 带作用域的遍历基类,对应 Java `com.alibaba.qlexpress4.aparser.ScopeStackVisitor`。
//! (`ExistStack`/`ExistVarStack` 已拆至 [`super::exist_stack`]。)
//!
//! Java `ScopeStackVisitor` is an abstract `QLParserBaseVisitor<Void>` that
//! pushes/pops an [`ExistStack`] around block-like constructs. Rust models
//! the "abstract class" as the [`ScopeStack`] state container plus the
//! [`ScopedVisitor`] extension trait: concrete visitors
//! ([`super::out_var_names_visitor`] 等) implement [`Visitor`] and delegate the
//! scope-sensitive methods to the default implementations here.

use super::exist_stack::{ExistStack, ExistVarStack};
use super::qlparser_base_visitor::Visitor;
use super::syntax_tree_factory::*;

/// Java `ScopeStackVisitor` 的 `existStack` 字段。
/// The `existStack` field of Java `ScopeStackVisitor`.
#[derive(Clone, Debug)]
pub struct ScopeStack<S: ExistStack> {
    stack: S,
}

impl<S: ExistStack> ScopeStack<S> {
    pub fn new(stack: S) -> Self {
        ScopeStack { stack }
    }

    /// Java `push()`.
    pub fn push(&mut self) {
        self.stack = self.stack.push();
    }

    /// Java `pop()`.
    pub fn pop(&mut self) {
        self.stack = self.stack.pop();
    }

    /// Java `getStack()`.
    pub fn stack(&self) -> &S {
        &self.stack
    }

    /// Java `getStack()` mutable (for `add`).
    pub fn stack_mut(&mut self) -> &mut S {
        &mut self.stack
    }
}

/// The scope-aware `visit*` overrides of Java `ScopeStackVisitor`, provided
/// as default methods. Concrete visitors implement [`ScopedVisitor`] and
/// forward from their [`Visitor`] implementation.
pub trait ScopedVisitor: Visitor<T = ()> {
    /// Access to the scope stack state.
    fn scope_stack(&mut self) -> &mut ScopeStack<ExistVarStack>;

    /// Java `ScopeStackVisitor.visitBlockExpr`.
    fn scoped_visit_block_expr(&mut self, ctx: &BlockExprContext) {
        self.scope_stack().push();
        self.visit_children_of(ctx);
        self.scope_stack().pop();
    }

    /// Java `ScopeStackVisitor.visitQlIf`.
    fn scoped_visit_ql_if(&mut self, ctx: &QlIfContext) {
        ctx.condition.accept(self);

        self.scope_stack().push();
        ctx.then_body.accept(self);
        self.scope_stack().pop();

        if let Some(else_body) = &ctx.else_body {
            self.scope_stack().push();
            else_body.accept(self);
            self.scope_stack().pop();
        }
    }

    /// Java `ScopeStackVisitor.visitSwitchExpr`.
    fn scoped_visit_switch_expr(&mut self, ctx: &SwitchExprContext) {
        ctx.expression.accept(self);

        if let Some(groups) = &ctx.groups {
            self.scope_stack().push();
            if let Node::SwitchCaseGroups(case_groups) = groups.as_ref() {
                for group in &case_groups.groups {
                    match group {
                        Node::SwitchStatementGroup(stmt_group) => {
                            if let Node::SwitchLabels(labels) = stmt_group.labels.as_ref() {
                                for label in &labels.labels {
                                    if let Node::SwitchLabel(switch_label) = label {
                                        if let Some(expression) = &switch_label.expression {
                                            expression.accept(self);
                                        }
                                    }
                                }
                            }
                            if let Some(block_statements) = &stmt_group.block_statements {
                                block_statements.accept(self);
                            }
                        }
                        Node::SwitchExprGroup(expr_group) => {
                            if let Node::SwitchExpressionLabel(label) = expr_group.label.as_ref() {
                                if let Some(expression_list) = &label.expression_list {
                                    if let Node::ExpressionList(list) = expression_list.as_ref() {
                                        for expr in &list.expressions {
                                            expr.accept(self);
                                        }
                                    }
                                }
                            }
                            expr_group.expression.accept(self);
                        }
                        _ => unreachable!("switch case group variant"),
                    }
                }
            }
            self.scope_stack().pop();
        }
    }

    /// Java `ScopeStackVisitor.visitTryCatchExpr`.
    fn scoped_visit_try_catch_expr(&mut self, ctx: &TryCatchExprContext) {
        if let Some(block_statements) = &ctx.block_statements {
            self.scope_stack().push();
            block_statements.accept(self);
            self.scope_stack().pop();
        }

        if let Some(try_catches) = &ctx.try_catches {
            try_catches.accept(self);
        }

        if let Some(try_finally) = &ctx.try_finally {
            self.scope_stack().push();
            try_finally.accept(self);
            self.scope_stack().pop();
        }
    }

    /// Java `ScopeStackVisitor.visitTryCatch`.
    fn scoped_visit_try_catch(&mut self, ctx: &TryCatchContext) {
        self.scope_stack().push();
        self.visit_children_of(ctx);
        self.scope_stack().pop();
    }

    /// Java `ScopeStackVisitor.visitFunctionStatement`.
    fn scoped_visit_function_statement(&mut self, ctx: &FunctionStatementContext) {
        ctx.var_id.accept(self);

        self.scope_stack().push();
        if let Some(params) = &ctx.params {
            params.accept(self);
        }
        if let Some(block_statements) = &ctx.block_statements {
            block_statements.accept(self);
        }
        self.scope_stack().pop();
    }
}
