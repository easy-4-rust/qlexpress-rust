//! 外部变量属性路径收集 Visitor,对应 Java `com.alibaba.qlexpress4.aparser.OutVarAttrsVisitor`。
//! 职责:收集脚本从外部上下文读取的属性路径(如 `a.b.c`)。
//! 本文件由 `out_var_visitors.rs` 拆分而来(SPEC §5.5 一类一文件),仅移动代码与补充中文注释,行为完全一致。
//! 文件内私有辅助函数为 Java 各 OutVar Visitor 私有方法的平移(在两个 Visitor 中各自重复,与 Java 一致)。

use std::collections::HashSet;

use super::exist_stack::{ExistStack, ExistVarStack};
use super::import_manager::ImportManager;
use super::qlparser_base_visitor::Visitor;
use super::scope_stack_visitor::{ScopeStack, ScopedVisitor};
use super::syntax_tree_factory::{
    BlockExprContext, ExpressionContext, FieldIdContext, FormalOrInferredParameterContext,
    FunctionStatementContext, ImportClsContext, ImportPackContext, LeftHandSideContext, Node,
    PrimaryContext, QlIfContext, SwitchExprContext, TryCatchContext, TryCatchExprContext,
    VarIdExprContext, VariableDeclaratorContext, VariableDeclaratorIdContext,
};
use super::token as tk;
use crate::utils::ql_string_utils::QLStringUtils;

/// Java `parseFieldId`.
fn parse_field_id(ctx: &FieldIdContext) -> String {
    if let Some(quote) = ctx.quote_string_literal() {
        return QLStringUtils::parse_string_escape(quote.text())
            .to_rust_string()
            .expect("quoted Rust source must remain valid UTF-8");
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

/// 外部变量属性路径收集器:收集脚本从外部上下文读取的属性路径。
/// 对应 Java: com.alibaba.qlexpress4.aparser.OutVarAttrsVisitor
/// Java `OutVarAttrsVisitor`: collects attribute paths (`a.b.c`) the script
/// reads from the outer context.
pub struct OutVarAttrsVisitor<'a> {
    out_var_attrs: HashSet<Vec<String>>,
    import_manager: ImportManager<'a>,
    stack: ScopeStack<ExistVarStack>,
}

impl<'a> OutVarAttrsVisitor<'a> {
    /// 构造收集器。对应 Java 构造器 `OutVarAttrsVisitor(importManager)`。
    /// Java `new OutVarAttrsVisitor(importManager)`.
    pub fn new(import_manager: ImportManager<'a>) -> Self {
        OutVarAttrsVisitor {
            out_var_attrs: HashSet::new(),
            import_manager,
            stack: ScopeStack::new(ExistVarStack::root()),
        }
    }

    /// 获取收集到的属性路径集合。对应 Java 方法 `getOutVarAttrs`。
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
        } else if self.stack.stack().exist(Some(&primary_id)) {
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

    fn visit_formal_or_inferred_parameter(
        &mut self,
        ctx: &FormalOrInferredParameterContext,
    ) -> Self::T {
        self.stack.stack_mut().add(Some(var_id_text(&ctx.var_id)));
    }

    fn visit_variable_declarator(&mut self, ctx: &VariableDeclaratorContext) -> Self::T {
        if let Some(initializer) = &ctx.initializer {
            initializer.accept(self);
        }
        ctx.id.accept(self);
    }

    fn visit_variable_declarator_id(&mut self, ctx: &VariableDeclaratorIdContext) -> Self::T {
        self.stack.stack_mut().add(Some(var_id_text(&ctx.var_id)));
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
            if !is_plain_assign && !self.stack.stack().exist(Some(&left_var_name)) {
                self.add_attrs(&left_var_name, &left_hand_side.path_parts);
            }
            if let Some(expression) = &ctx.expression {
                expression.accept(self);
            }
            self.stack.stack_mut().add(Some(left_var_name));
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
            self.stack.stack_mut().add(Some(left_var_name));
        } else if !self.stack.stack().exist(Some(&left_var_name)) {
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
