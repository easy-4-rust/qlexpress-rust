//! External-variable / external-function collection visitors, mirroring
//! Java `OutVarNamesVisitor`, `OutVarAttrsVisitor` and `OutFunctionVisitor`.

use std::collections::HashSet;

use super::import_manager::ImportManager;
use super::scope_stack_visitor::{ExistStack, ExistVarStack, ScopeStack, ScopedVisitor};
use super::syntax_tree::*;
use super::token as tk;
use crate::utils::ql_string_utils::QLStringUtils;

// ---------------------------------------------------------------------------
// Shared helpers (Java private methods duplicated in both OutVar visitors)
// ---------------------------------------------------------------------------

/// Java `parseFieldId`.
fn parse_field_id(ctx: &FieldIdContext) -> String {
    if let Some(quote) = &ctx.quote {
        return QLStringUtils::parse_string_escape(quote.text());
    }
    ctx.token
        .as_ref()
        .map(|t| t.text().to_string())
        .unwrap_or_default()
}

/// Java `VarIdContext.getText` via node.
fn var_id_text(var_id: &Node) -> String {
    match var_id {
        Node::VarId(v) => v.token.text().to_string(),
        _ => String::new(),
    }
}

/// Collect the leading `head.field.field...` id chain, mirroring the Java
/// loop over `FieldAccessContext` path parts.
fn head_part_ids(primary_id: &str, path_parts: &[Node]) -> Vec<String> {
    let mut head = vec![primary_id.to_string()];
    for part in path_parts {
        let field = match part {
            Node::FieldAccess(field_access) => match field_access.field_id.as_ref() {
                Node::FieldId(field_id) => parse_field_id(field_id),
                _ => break,
            },
            _ => break,
        };
        head.push(field);
    }
    head
}

/// Java `isSimpleVariableLeftHandSide`.
fn is_simple_variable_left_hand_side(ctx: &LeftHandSideContext) -> bool {
    ctx.lparen.is_none() && ctx.path_parts.is_empty()
}

/// Import handling shared by both OutVar visitors (Java `visitImportCls` /
/// `visitImportPack`).
fn handle_import_cls(import_manager: &mut ImportManager<'_>, ctx: &ImportClsContext) {
    let path = ctx
        .var_ids
        .iter()
        .map(var_id_text)
        .collect::<Vec<_>>()
        .join(".");
    import_manager.add_import(super::import_manager::QLImport::import_cls(path));
}

fn handle_import_pack(import_manager: &mut ImportManager<'_>, ctx: &ImportPackContext) {
    let ids: Vec<String> = ctx.var_ids.iter().map(var_id_text).collect();
    let last = ids.last().cloned().unwrap_or_default();
    let is_inner_cls = !last.chars().next().map(char::is_lowercase).unwrap_or(false);
    let import_path = ids.join(".");
    let import = if is_inner_cls {
        super::import_manager::QLImport::import_inner_cls(import_path)
    } else {
        super::import_manager::QLImport::import_pack(import_path)
    };
    import_manager.add_import(import);
}

// ---------------------------------------------------------------------------
// OutVarNamesVisitor
// ---------------------------------------------------------------------------

/// Java `OutVarNamesVisitor`: collects the names of variables the script
/// reads from (or compound-assigns into) the outer context.
pub struct OutVarNamesVisitor<'a> {
    out_vars: HashSet<String>,
    import_manager: ImportManager<'a>,
    stack: ScopeStack<ExistVarStack>,
}

impl<'a> OutVarNamesVisitor<'a> {
    /// Java `new OutVarNamesVisitor(importManager)`.
    pub fn new(import_manager: ImportManager<'a>) -> Self {
        OutVarNamesVisitor {
            out_vars: HashSet::new(),
            import_manager,
            stack: ScopeStack::new(ExistVarStack::root()),
        }
    }

    /// Java `getOutVars`.
    pub fn out_vars(&self) -> &HashSet<String> {
        &self.out_vars
    }

    /// Java `parseVarIdInPath`.
    fn parse_var_id_in_path(&mut self, ctx: &VarIdExprContext, path_parts: &[Node]) -> usize {
        if ctx.lparen.is_some() {
            if let Some(argument_list) = &ctx.argument_list {
                argument_list.accept(self);
            }
            return 0;
        }

        let primary_id = var_id_text(&ctx.var_id);
        let head_ids = head_part_ids(&primary_id, path_parts);
        let result = self.import_manager.load_part_qualified(&head_ids);
        if result.cls().is_some() {
            result.rest_index().saturating_sub(1)
        } else {
            if !self.stack.stack().exist(&primary_id) {
                self.out_vars.insert(primary_id);
            }
            0
        }
    }
}

impl ScopedVisitor for OutVarNamesVisitor<'_> {
    fn scope_stack(&mut self) -> &mut ScopeStack<ExistVarStack> {
        &mut self.stack
    }
}

impl Visitor for OutVarNamesVisitor<'_> {
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

    fn visit_function_statement(&mut self, ctx: &FunctionStatementContext) -> Self::T {
        self.scoped_visit_function_statement(ctx)
    }

    fn visit_import_cls(&mut self, ctx: &ImportClsContext) -> Self::T {
        handle_import_cls(&mut self.import_manager, ctx);
    }

    fn visit_import_pack(&mut self, ctx: &ImportPackContext) -> Self::T {
        handle_import_pack(&mut self.import_manager, ctx);
    }

    fn visit_formal_or_inferred_parameter(&mut self, ctx: &FormalOrInferredParameterContext) -> Self::T {
        self.stack.stack_mut().add(var_id_text(&ctx.var_id));
    }

    fn visit_variable_declarator(&mut self, ctx: &VariableDeclaratorContext) -> Self::T {
        if let Some(initializer) = &ctx.initializer {
            initializer.accept(self);
        }
        ctx.id.accept(self);
    }

    fn visit_variable_declarator_id(&mut self, ctx: &VariableDeclaratorIdContext) -> Self::T {
        self.stack.stack_mut().add(var_id_text(&ctx.var_id));
    }

    fn visit_expression(&mut self, ctx: &ExpressionContext) -> Self::T {
        if let Some(ternary) = &ctx.ternary {
            ternary.accept(self);
            return;
        }

        let Some(left) = &ctx.left else {
            return;
        };
        let Node::LeftHandSide(left_hand_side) = left.as_ref() else {
            return;
        };
        if is_simple_variable_left_hand_side(left_hand_side) {
            let left_var_name = var_id_text(&left_hand_side.var_id);
            let is_plain_assign = matches!(
                ctx.assign_operator.as_deref(),
                Some(Node::AssignOperator(op)) if op.token.symbol().token_type() == tk::EQ as i32
            );
            if !is_plain_assign && !self.stack.stack().exist(&left_var_name) {
                self.out_vars.insert(left_var_name.clone());
            }
            if let Some(expression) = &ctx.expression {
                expression.accept(self);
            }
            self.stack.stack_mut().add(left_var_name);
            return;
        }

        left.accept(self);
        if let Some(expression) = &ctx.expression {
            expression.accept(self);
        }
    }

    fn visit_left_hand_side(&mut self, ctx: &LeftHandSideContext) -> Self::T {
        let left_var_name = var_id_text(&ctx.var_id);
        if ctx.path_parts.is_empty() {
            self.stack.stack_mut().add(left_var_name);
        } else if !self.stack.stack().exist(&left_var_name) {
            self.out_vars.insert(left_var_name);
        }
    }

    fn visit_context_select_expr(&mut self, ctx: &ContextSelectExprContext) -> Self::T {
        let variable_name = ctx.selector_variable.text().trim().to_string();
        if !self.stack.stack().exist(&variable_name) {
            self.out_vars.insert(variable_name);
        }
    }

    fn visit_primary(&mut self, ctx: &PrimaryContext) -> Self::T {
        if let Some(Node::VarIdExpr(var_id_expr)) = ctx.pathable.as_deref() {
            let rest_index = self.parse_var_id_in_path(var_id_expr, &ctx.path_parts);
            for part in ctx.path_parts.iter().skip(rest_index) {
                part.accept(self);
            }
            return;
        }
        self.visit_children_of(ctx);
    }

    fn visit_var_id_expr(&mut self, ctx: &VarIdExprContext) -> Self::T {
        let var_name = var_id_text(&ctx.var_id);
        if !self.stack.stack().exist(&var_name) {
            self.out_vars.insert(var_name);
        }
    }
}

// ---------------------------------------------------------------------------
// OutVarAttrsVisitor
// ---------------------------------------------------------------------------

/// Java `OutVarAttrsVisitor`: collects attribute paths (`a.b.c`) the script
/// reads from the outer context.
pub struct OutVarAttrsVisitor<'a> {
    out_var_attrs: HashSet<Vec<String>>,
    import_manager: ImportManager<'a>,
    stack: ScopeStack<ExistVarStack>,
}

impl<'a> OutVarAttrsVisitor<'a> {
    /// Java `new OutVarAttrsVisitor(importManager)`.
    pub fn new(import_manager: ImportManager<'a>) -> Self {
        OutVarAttrsVisitor {
            out_var_attrs: HashSet::new(),
            import_manager,
            stack: ScopeStack::new(ExistVarStack::root()),
        }
    }

    /// Java `getOutVarAttrs`.
    pub fn out_var_attrs(&self) -> &HashSet<Vec<String>> {
        &self.out_var_attrs
    }

    /// Java `parseOutVarAttrInPath`.
    fn parse_out_var_attr_in_path(&mut self, ctx: &VarIdExprContext, path_parts: &[Node]) -> usize {
        if ctx.lparen.is_some() {
            if let Some(argument_list) = &ctx.argument_list {
                argument_list.accept(self);
            }
            return 0;
        }

        let primary_id = var_id_text(&ctx.var_id);
        let head_ids = head_part_ids(&primary_id, path_parts);
        let result = self.import_manager.load_part_qualified(&head_ids);
        if result.cls().is_some() {
            result.rest_index().saturating_sub(1)
        } else if self.stack.stack().exist(&primary_id) {
            0
        } else {
            self.add_attrs(&primary_id, path_parts)
        }
    }

    /// Java `addAttrs`: record `[primaryId, field, field, ...]` for the
    /// leading field-access chain; returns the first un-consumed index.
    fn add_attrs(&mut self, primary_id: &str, path_parts: &[Node]) -> usize {
        let mut attrs = vec![primary_id.to_string()];
        let mut i = 0;
        for part in path_parts {
            let field_id = match part {
                Node::FieldAccess(field_access) => match field_access.field_id.as_ref() {
                    Node::FieldId(field_id) => Some(parse_field_id(field_id)),
                    _ => None,
                },
                _ => None,
            };
            let Some(field_id) = field_id else {
                break;
            };
            attrs.push(field_id);
            i += 1;
        }
        self.out_var_attrs.insert(attrs);
        i
    }
}

impl ScopedVisitor for OutVarAttrsVisitor<'_> {
    fn scope_stack(&mut self) -> &mut ScopeStack<ExistVarStack> {
        &mut self.stack
    }
}

impl Visitor for OutVarAttrsVisitor<'_> {
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

    fn visit_function_statement(&mut self, ctx: &FunctionStatementContext) -> Self::T {
        self.scoped_visit_function_statement(ctx)
    }

    fn visit_import_cls(&mut self, ctx: &ImportClsContext) -> Self::T {
        handle_import_cls(&mut self.import_manager, ctx);
    }

    fn visit_import_pack(&mut self, ctx: &ImportPackContext) -> Self::T {
        handle_import_pack(&mut self.import_manager, ctx);
    }

    fn visit_formal_or_inferred_parameter(&mut self, ctx: &FormalOrInferredParameterContext) -> Self::T {
        self.stack.stack_mut().add(var_id_text(&ctx.var_id));
    }

    fn visit_variable_declarator(&mut self, ctx: &VariableDeclaratorContext) -> Self::T {
        if let Some(initializer) = &ctx.initializer {
            initializer.accept(self);
        }
        ctx.id.accept(self);
    }

    fn visit_variable_declarator_id(&mut self, ctx: &VariableDeclaratorIdContext) -> Self::T {
        self.stack.stack_mut().add(var_id_text(&ctx.var_id));
    }

    fn visit_expression(&mut self, ctx: &ExpressionContext) -> Self::T {
        if let Some(ternary) = &ctx.ternary {
            ternary.accept(self);
            return;
        }

        let Some(left) = &ctx.left else {
            return;
        };
        let Node::LeftHandSide(left_hand_side) = left.as_ref() else {
            return;
        };
        if is_simple_variable_left_hand_side(left_hand_side) {
            let left_var_name = var_id_text(&left_hand_side.var_id);
            let is_plain_assign = matches!(
                ctx.assign_operator.as_deref(),
                Some(Node::AssignOperator(op)) if op.token.symbol().token_type() == tk::EQ as i32
            );
            if !is_plain_assign && !self.stack.stack().exist(&left_var_name) {
                self.add_attrs(&left_var_name, &left_hand_side.path_parts);
            }
            if let Some(expression) = &ctx.expression {
                expression.accept(self);
            }
            self.stack.stack_mut().add(left_var_name);
            return;
        }

        left.accept(self);
        if let Some(expression) = &ctx.expression {
            expression.accept(self);
        }
    }

    fn visit_left_hand_side(&mut self, ctx: &LeftHandSideContext) -> Self::T {
        let left_var_name = var_id_text(&ctx.var_id);
        if ctx.path_parts.is_empty() {
            self.stack.stack_mut().add(left_var_name);
        } else if !self.stack.stack().exist(&left_var_name) {
            self.add_attrs(&left_var_name, &ctx.path_parts);
        }
    }

    fn visit_primary(&mut self, ctx: &PrimaryContext) -> Self::T {
        if let Some(Node::VarIdExpr(var_id_expr)) = ctx.pathable.as_deref() {
            let rest_index = self.parse_out_var_attr_in_path(var_id_expr, &ctx.path_parts);
            for part in ctx.path_parts.iter().skip(rest_index) {
                part.accept(self);
            }
            return;
        }
        self.visit_children_of(ctx);
    }
}

// ---------------------------------------------------------------------------
// OutFunctionVisitor
// ---------------------------------------------------------------------------

/// Java `OutFunctionVisitor`: collects names of called functions that are
/// not defined inside the script.
pub struct OutFunctionVisitor {
    out_functions: HashSet<String>,
    stack: ScopeStack<ExistVarStack>,
}

impl OutFunctionVisitor {
    /// Java `new OutFunctionVisitor()`.
    pub fn new() -> Self {
        OutFunctionVisitor {
            out_functions: HashSet::new(),
            stack: ScopeStack::new(ExistVarStack::root()),
        }
    }

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
