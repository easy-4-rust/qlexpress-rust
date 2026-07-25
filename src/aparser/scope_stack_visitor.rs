//! Scoped traversal base, mirroring Java `ScopeStackVisitor` + `ExistStack`.
//!
//! Java `ScopeStackVisitor` is an abstract `QLParserBaseVisitor<Void>` that
//! pushes/pops an [`ExistStack`] around block-like constructs. Rust models
//! the "abstract class" as the [`ScopeStack`] state container plus the
//! [`ScopedVisitor`] extension trait: concrete visitors
//! ([`super::out_var_visitors`]) implement [`Visitor`] and delegate the
//! scope-sensitive methods to the default implementations here.

use std::collections::HashSet;
use std::rc::Rc;

use super::syntax_tree::*;

/// Java `ExistStack` (with the `add` operation its implementations expose).
pub trait ExistStack: Sized {
    /// Java `push`: a child scope.
    fn push(&self) -> Self;
    /// Java `pop`: the parent scope; panics on the root (Java would NPE).
    fn pop(&self) -> Self;
    /// Java `exist`: is `var_name` visible in this scope chain?
    fn exist(&self, var_name: &str) -> bool;
    /// Declare `var_name` in the current (top) scope.
    fn add(&mut self, var_name: String);
}

/// Persistent scope stack shared by the out-var/out-function visitors,
/// mirroring the duplicated `ExistVarStack`/`ExistFunctionStack` private
/// classes in the Java visitors.
#[derive(Clone, Debug, Default)]
pub struct ExistVarStack {
    parent: Option<Rc<ExistVarStack>>,
    exist_vars: HashSet<String>,
}

impl ExistVarStack {
    /// Java `new ExistVarStack(null)` — a root scope.
    pub fn root() -> Self {
        ExistVarStack::default()
    }
}

impl ExistStack for ExistVarStack {
    fn push(&self) -> Self {
        ExistVarStack {
            parent: Some(Rc::new(self.clone())),
            exist_vars: HashSet::new(),
        }
    }

    fn pop(&self) -> Self {
        match &self.parent {
            Some(parent) => (**parent).clone(),
            None => panic!("ExistStack.pop on root scope"),
        }
    }

    fn exist(&self, var_name: &str) -> bool {
        if self.exist_vars.contains(var_name) {
            return true;
        }
        self.parent
            .as_ref()
            .map(|parent| parent.exist(var_name))
            .unwrap_or(false)
    }

    fn add(&mut self, var_name: String) {
        self.exist_vars.insert(var_name);
    }
}

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
