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
use super::syntax_tree_factory::{
    BlockExprContext, FunctionStatementContext, Node, QlIfContext, SwitchExprContext,
    TryCatchContext, TryCatchExprContext,
};

pub use super::scope_stack::ScopeStack;

impl<S: ExistStack> ScopeStack<S> {
    /// 使用根变量存在性栈创建作用域状态。
    /// 对应 Java: `ScopeStackVisitor` 初始化 `existStack` 字段。
    pub fn new(stack: S) -> Self {
        ScopeStack { stack: Some(stack) }
    }

    /// 将一个元素压入当前栈。
    /// 无显式参数；返回：无。
    /// 对应或承接 Java 源文件：`com/alibaba/qlexpress4/aparser/ScopeStackVisitor.java`，方法 `push`；Rust 侧按所有权与 `Result` 语义适配。
    /// Java `push()`.
    /// 对应 Java：`ScopeStackVisitor#push()`。
    pub fn push(&mut self) {
        self.stack = Some(
            self.stack
                .as_ref()
                .expect("ScopeStackVisitor.push on null stack")
                .push(),
        );
    }

    /// 弹出并返回当前栈顶元素。
    /// 无显式参数；返回：无。
    /// 对应或承接 Java 源文件：`com/alibaba/qlexpress4/aparser/ScopeStackVisitor.java`，方法 `pop`；Rust 侧按所有权与 `Result` 语义适配。
    /// Java `pop()`.
    /// 对应 Java: com.alibaba.qlexpress4.aparser.ScopeStackVisitor#pop。
    pub fn pop(&mut self) {
        self.stack = self.stack.take().and_then(|stack| stack.pop());
    }

    /// 返回当前内部栈的只读视图。
    /// 无显式参数；返回：`&S`。
    /// 对应或承接 Java 源文件：`com/alibaba/qlexpress4/aparser/ScopeStackVisitor.java`，方法 `stack`；Rust 侧按所有权与 `Result` 语义适配。
    /// Java `getStack()`.
    /// 对应 Java: com.alibaba.qlexpress4.aparser.ScopeStackVisitor#stack。
    pub fn stack(&self) -> &S {
        self.stack
            .as_ref()
            .expect("ScopeStackVisitor.getStack returned null")
    }

    /// 返回当前内部栈的可变视图。
    /// 无显式参数；返回：`&mut S`。
    /// 对应或承接 Java 源文件：`com/alibaba/qlexpress4/aparser/ScopeStackVisitor.java`，方法 `stackMut`；Rust 侧按所有权与 `Result` 语义适配。
    /// Java `getStack()` mutable (for `add`).
    /// 对应 Java: com.alibaba.qlexpress4.aparser.ScopeStackVisitor#stackMut。
    pub fn stack_mut(&mut self) -> &mut S {
        self.stack
            .as_mut()
            .expect("ScopeStackVisitor.getStack returned null")
    }

    /// 返回 Java `getStack()` 的可空结果；根栈执行 `pop()` 后为 `None`。
    /// 对应 Java: com.alibaba.qlexpress4.aparser.ScopeStackVisitor#getStack。
    pub fn get_stack(&self) -> Option<&S> {
        self.stack.as_ref()
    }
}

/// `ScopedVisitor` 接口的 Rust 实现，保留对应对象的领域职责与公开契约。
/// 对应或承接 Java 源文件：`com/alibaba/qlexpress4/aparser/ScopeStackVisitor.java`；具体对象路径见 `docs/对象级对照表.md`。
/// The scope-aware `visit*` overrides of Java `ScopeStackVisitor`, provided
/// as default methods. Concrete visitors implement [`ScopedVisitor`] and
/// forward from their [`Visitor`] implementation.
/// 对应 Java: com.alibaba.qlexpress4.aparser.ScopeStackVisitor。
pub trait ScopedVisitor: Visitor<T = ()> {
    /// 处理 scope stack 对应的接口职责。
    /// 无显式参数；返回：`&mut ScopeStack<ExistVarStack>`。
    /// 对应或承接 Java 源文件：`com/alibaba/qlexpress4/aparser/ScopeStackVisitor.java`，方法 `scopeStack`。
    /// Access to the scope stack state.
    fn scope_stack(&mut self) -> &mut ScopeStack<ExistVarStack>;

    /// 处理 scoped visit block expr 对应的接口职责。
    /// 参数：`ctx`；返回：无。
    /// 对应或承接 Java 源文件：`com/alibaba/qlexpress4/aparser/ScopeStackVisitor.java`，方法 `scopedVisitBlockExpr`。
    /// Java `ScopeStackVisitor.visitBlockExpr`.
    fn scoped_visit_block_expr(&mut self, ctx: &BlockExprContext) {
        self.scope_stack().push();
        self.visit_children_of(ctx);
        self.scope_stack().pop();
    }

    /// 处理 scoped visit ql if 对应的接口职责。
    /// 参数：`ctx`；返回：无。
    /// 对应或承接 Java 源文件：`com/alibaba/qlexpress4/aparser/ScopeStackVisitor.java`，方法 `scopedVisitQlIf`。
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

    /// 处理 scoped visit switch expr 对应的接口职责。
    /// 参数：`ctx`；返回：无。
    /// 对应或承接 Java 源文件：`com/alibaba/qlexpress4/aparser/ScopeStackVisitor.java`，方法 `scopedVisitSwitchExpr`。
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

    /// 处理 scoped visit try catch expr 对应的接口职责。
    /// 参数：`ctx`；返回：无。
    /// 对应或承接 Java 源文件：`com/alibaba/qlexpress4/aparser/ScopeStackVisitor.java`，方法 `scopedVisitTryCatchExpr`。
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

    /// 处理 scoped visit try catch 对应的接口职责。
    /// 参数：`ctx`；返回：无。
    /// 对应或承接 Java 源文件：`com/alibaba/qlexpress4/aparser/ScopeStackVisitor.java`，方法 `scopedVisitTryCatch`。
    /// Java `ScopeStackVisitor.visitTryCatch`.
    fn scoped_visit_try_catch(&mut self, ctx: &TryCatchContext) {
        self.scope_stack().push();
        self.visit_children_of(ctx);
        self.scope_stack().pop();
    }

    /// 处理 scoped visit function statement 对应的接口职责。
    /// 参数：`ctx`；返回：无。
    /// 对应或承接 Java 源文件：`com/alibaba/qlexpress4/aparser/ScopeStackVisitor.java`，方法 `scopedVisitFunctionStatement`。
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
