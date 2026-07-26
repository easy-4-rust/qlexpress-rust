//! 外部函数名收集 Visitor,对应 Java `com.alibaba.qlexpress4.aparser.OutFunctionVisitor`。
//! 职责:收集脚本中调用但未在脚本内定义的函数名。
//! 本文件由 `out_var_visitors.rs` 拆分而来(SPEC §5.5 一类一文件),仅移动代码与补充中文注释,行为完全一致。

use std::collections::HashSet;

use super::exist_stack::{ExistStack, ExistVarStack};
use super::qlparser_base_visitor::Visitor;
use super::scope_stack_visitor::{ScopeStack, ScopedVisitor};
use super::syntax_tree_factory::*;

/// Java `VarIdContext.getText` via node.
fn var_id_text(var_id: &Node) -> String {
    match var_id {
        Node::VarId(v) => v.token.text().to_string(),
        _ => String::new(),
    }
}

/// 外部函数名收集器:收集脚本中调用但未在脚本内定义的函数名。
/// 对应 Java: com.alibaba.qlexpress4.aparser.OutFunctionVisitor
/// Java `OutFunctionVisitor`: collects names of called functions that are
/// not defined inside the script.
pub struct OutFunctionVisitor {
    out_functions: HashSet<String>,
    stack: ScopeStack<ExistVarStack>,
}

impl OutFunctionVisitor {
    /// 构造收集器。对应 Java 构造器 `OutFunctionVisitor()`。
    /// Java `new OutFunctionVisitor()`.
    pub fn new() -> Self {
        OutFunctionVisitor {
            out_functions: HashSet::new(),
            stack: ScopeStack::new(ExistVarStack::root()),
        }
    }

    /// 获取收集到的外部函数名。对应 Java 方法 `getOutFunctions`。
    /// Java `getOutFunctions`.
    pub fn out_functions(&self) -> &HashSet<String> {
        &self.out_functions
    }
}

impl Default for OutFunctionVisitor {
    fn default() -> Self {
        Self::new()
    }
}

impl ScopedVisitor for OutFunctionVisitor {
    fn scope_stack(&mut self) -> &mut ScopeStack<ExistVarStack> {
        &mut self.stack
    }
}

impl Visitor for OutFunctionVisitor {
    type T = ();

    fn visit_block_expr(&mut self, ctx: &BlockExprContext) -> Self::T {
        self.scoped_visit_block_expr(ctx)
    }

    fn visit_ql_if(&mut self, ctx: &QlIfContext) -> Self::T {
        self.scoped_visit_ql_if(ctx)
    }

    fn visit_switch_expr(&mut self, ctx: &SwitchExprContext) -> Self::T {
        self.scoped_visit_switch_expr(ctx)
    }

    fn visit_try_catch_expr(&mut self, ctx: &TryCatchExprContext) -> Self::T {
        self.scoped_visit_try_catch_expr(ctx)
    }

    fn visit_try_catch(&mut self, ctx: &TryCatchContext) -> Self::T {
        self.scoped_visit_try_catch(ctx)
    }

    fn visit_block_statements(&mut self, ctx: &BlockStatementsContext) -> Self::T {
        let non_empty: Vec<&Node> = ctx
            .statements
            .iter()
            .filter(|bs| !matches!(bs, Node::EmptyStatement(_)))
            .collect();
        // Process all function definitions first to support forward refs.
        for child in &non_empty {
            if matches!(child, Node::FunctionStatement(_)) {
                child.accept(self);
            }
        }
        for child in &non_empty {
            if !matches!(child, Node::FunctionStatement(_)) {
                child.accept(self);
            }
        }
    }

    fn visit_var_id_expr(&mut self, ctx: &VarIdExprContext) -> Self::T {
        if ctx.lparen.is_some() {
            let function_name = var_id_text(&ctx.var_id);
            if !self.stack.stack().exist(&function_name) {
                self.out_functions.insert(function_name);
            }
        }
        self.visit_children_of(ctx);
    }

    fn visit_left_hand_side(&mut self, ctx: &LeftHandSideContext) -> Self::T {
        if ctx.lparen.is_some() {
            let function_name = var_id_text(&ctx.var_id);
            if !self.stack.stack().exist(&function_name) {
                self.out_functions.insert(function_name);
            }
        }
        self.visit_children_of(ctx);
    }

    fn visit_function_statement(&mut self, ctx: &FunctionStatementContext) -> Self::T {
        let function_name = var_id_text(&ctx.var_id);
        self.stack.stack_mut().add(function_name.clone());

        if let Some(params) = &ctx.params {
            params.accept(self);
        }

        if let Some(block_statements) = &ctx.block_statements {
            self.stack.push();
            // Recursion scene: the function can call itself.
            self.stack.stack_mut().add(function_name);
            block_statements.accept(self);
            self.stack.pop();
        }
    }
}
