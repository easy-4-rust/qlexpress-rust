//! 静态表达式追踪访问器，对应 Java
//! `com.alibaba.qlexpress4.aparser.TraceExpressionVisitor`。
//!
//! 该访问器只遍历语法树，不执行脚本；产出的 [`TracePointTree`] 会写入
//! 编译缓存，并在每次执行前转换为全新的运行时表达式追踪树。

use crate::aparser::qlparser_base_visitor::Visitor;
use crate::aparser::syntax_tree_factory::{
    BaseExprContext, BlockExprContext, BlockStatementsContext, CastExprContext, ConstExprContext,
    ContextSelectExprContext, ElseBodyContext, EmptyStatementContext, ExpressionContext,
    ExpressionStatementContext, ForEachStatementContext, FunctionStatementContext,
    GroupExprContext, LeftHandSideContext, ListExprContext,
    LocalVariableDeclarationStatementContext, MacroStatementContext, MapExprContext,
    NewEmptyArrExprContext, NewInitArrExprContext, NewObjExprContext, Node,
    NonExpressionStatementContext, PrimaryContext, QlIfContext, ReturnStatementContext,
    SwitchExprContext, TernaryExprContext, ThenBodyContext, ThrowStatementContext,
    TraditionalForStatementContext, TryCatchExprContext, TypeExprContext, VarIdExprContext,
    WhileStatementContext,
};
use crate::aparser::token::Token;
use crate::runtime::trace::{TracePointTree, TraceType};

/// 收集脚本静态表达式追踪点。
///
/// 对应 Java: `com.alibaba.qlexpress4.aparser.TraceExpressionVisitor`。
#[derive(Default)]
pub struct TraceExpressionVisitor {
    expression_trace_points: Vec<TracePointTree>,
}

impl TraceExpressionVisitor {
    /// 创建空访问器。对应 Java 默认构造器。
    pub fn new() -> Self {
        Self::default()
    }

    /// 遍历语法树并返回顶层追踪点。
    ///
    /// 对应 Java 先调用 `tree.accept(visitor)` 再调用
    /// `getExpressionTracePoints()`；返回值是快照，访问器仍保留已收集结果。
    pub fn visit(&mut self, tree: &Node) -> Vec<TracePointTree> {
        tree.accept(self);
        self.expression_trace_points.clone()
    }

    /// 获取已收集的顶层追踪点。对应 Java 方法 `getExpressionTracePoints`。
    pub fn expression_trace_points(&self) -> &[TracePointTree] {
        &self.expression_trace_points
    }

    fn new_point(
        trace_type: TraceType,
        children: Vec<TracePointTree>,
        token: &Token,
    ) -> TracePointTree {
        Self::new_point_with_text(trace_type, children, token.text(), token)
    }

    fn new_point_with_text(
        trace_type: TraceType,
        children: Vec<TracePointTree>,
        text: impl Into<String>,
        token: &Token,
    ) -> TracePointTree {
        TracePointTree::new(
            trace_type,
            text,
            children,
            token.line(),
            token.char_position_in_line(),
            token.start_index(),
        )
    }

    fn point_for_node(
        trace_type: TraceType,
        children: Vec<TracePointTree>,
        node: &Node,
    ) -> Option<TracePointTree> {
        node.start_token()
            .map(|token| Self::new_point(trace_type, children, token))
    }

    fn trace_argument_list(&mut self, argument_list: Option<&Node>) -> Vec<TracePointTree> {
        match argument_list {
            Some(Node::ArgumentList(ctx)) => ctx
                .expressions
                .iter()
                .filter_map(|expression| expression.accept(self))
                .collect(),
            _ => Vec::new(),
        }
    }

    fn path_parts(&mut self, mut path_root: TracePointTree, path_parts: &[Node]) -> TracePointTree {
        for current in path_parts {
            match current {
                Node::MethodInvoke(ctx) => {
                    let mut children = vec![path_root];
                    children.extend(self.trace_argument_list(ctx.argument_list.as_deref()));
                    let token = ctx
                        .var_id
                        .start_token()
                        .expect("method identifier must have a token");
                    path_root = Self::new_point(TraceType::Method, children, token);
                }
                Node::IndexExpr(ctx) => {
                    let mut children = vec![path_root];
                    if let Some(index_value) = &ctx.index_value_expr {
                        match index_value.as_ref() {
                            Node::SingleIndex(single) => {
                                if let Some(point) = single.expression.accept(self) {
                                    children.push(point);
                                }
                            }
                            Node::SliceIndex(slice) => {
                                if let Some(start) = &slice.start {
                                    if let Some(point) = start.accept(self) {
                                        children.push(point);
                                    }
                                }
                                if let Some(end) = &slice.end {
                                    if let Some(point) = end.accept(self) {
                                        children.push(point);
                                    }
                                }
                            }
                            other => {
                                if let Some(point) = other.accept(self) {
                                    children.push(point);
                                }
                            }
                        }
                    }
                    path_root = Self::new_point(TraceType::Operator, children, ctx.lbrack.symbol());
                }
                Node::FieldAccess(ctx) => {
                    let token = ctx
                        .field_id
                        .stop_token()
                        .expect("field identifier must have a token");
                    path_root = Self::new_point(TraceType::Field, vec![path_root], token);
                }
                _ => {
                    if let Some(token) = current.stop_token() {
                        path_root = Self::new_point(TraceType::Field, vec![path_root], token);
                    }
                }
            }
        }
        path_root
    }

    fn block_point(&mut self, block_statements: Option<&Node>, anchor: &Token) -> TracePointTree {
        let children = match block_statements {
            Some(block_statements) => {
                let mut visitor = Self::new();
                block_statements.accept(&mut visitor);
                visitor.expression_trace_points
            }
            None => Vec::new(),
        };
        Self::new_point(TraceType::Block, children, anchor)
    }

    fn visit_non_expression_statement_internal(&mut self, node: &Node) -> Option<TracePointTree> {
        match node {
            Node::NonExpressionStatement(ctx) => {
                self.visit_non_expression_statement_internal(&ctx.statement)
            }
            Node::ThrowStatement(ctx) => Some(Self::new_point(
                TraceType::Statement,
                vec![],
                ctx.throw_token.symbol(),
            )),
            Node::ReturnStatement(ctx) => {
                let children = ctx
                    .expression
                    .as_deref()
                    .and_then(|expression| expression.accept(self))
                    .into_iter()
                    .collect();
                Some(Self::new_point(
                    TraceType::Return,
                    children,
                    ctx.return_token.symbol(),
                ))
            }
            Node::WhileStatement(ctx) => Some(Self::new_point(
                TraceType::Statement,
                vec![],
                ctx.while_token.symbol(),
            )),
            Node::TraditionalForStatement(ctx) => Some(Self::new_point(
                TraceType::Statement,
                vec![],
                ctx.for_token.symbol(),
            )),
            Node::ForEachStatement(ctx) => Some(Self::new_point(
                TraceType::Statement,
                vec![],
                ctx.for_token.symbol(),
            )),
            Node::FunctionStatement(ctx) => ctx
                .var_id
                .start_token()
                .map(|token| Self::new_point(TraceType::DefineFunction, vec![], token)),
            Node::MacroStatement(ctx) => ctx
                .var_id
                .start_token()
                .map(|token| Self::new_point(TraceType::DefineMacro, vec![], token)),
            Node::BreakContinueStatement(ctx) => Some(Self::new_point(
                TraceType::Statement,
                vec![],
                ctx.token.symbol(),
            )),
            Node::EmptyStatement(ctx) => Some(Self::new_point(
                TraceType::Statement,
                vec![],
                ctx.token.symbol(),
            )),
            Node::LocalVariableDeclarationStatement(ctx) => Self::point_for_node(
                TraceType::Statement,
                vec![],
                &ctx.local_variable_declaration,
            ),
            _ => Self::point_for_node(TraceType::Statement, vec![], node),
        }
    }
}

impl Visitor for TraceExpressionVisitor {
    type T = Option<TracePointTree>;

    // ==================== Statement ====================

    /// 记录 throw 语句。对应 Java 方法 `visitThrowStatement`。
    fn visit_throw_statement(&mut self, ctx: &ThrowStatementContext) -> Self::T {
        self.expression_trace_points.push(Self::new_point(
            TraceType::Statement,
            vec![],
            ctx.throw_token.symbol(),
        ));
        None
    }

    /// 记录局部变量声明语句。对应 Java 方法
    /// `visitLocalVariableDeclarationStatement`。
    fn visit_local_variable_declaration_statement(
        &mut self,
        ctx: &LocalVariableDeclarationStatementContext,
    ) -> Self::T {
        if let Some(point) = Self::point_for_node(
            TraceType::Statement,
            vec![],
            &ctx.local_variable_declaration,
        ) {
            self.expression_trace_points.push(point);
        }
        None
    }

    /// 记录表达式语句的完整表达式树。对应 Java 方法 `visitExpressionStatement`。
    fn visit_expression_statement(&mut self, ctx: &ExpressionStatementContext) -> Self::T {
        if let Some(point) = ctx.expression.accept(self) {
            self.expression_trace_points.push(point);
        }
        None
    }

    /// 记录 while 语句。对应 Java 方法 `visitWhileStatement`。
    fn visit_while_statement(&mut self, ctx: &WhileStatementContext) -> Self::T {
        self.expression_trace_points.push(Self::new_point(
            TraceType::Statement,
            vec![],
            ctx.while_token.symbol(),
        ));
        None
    }

    /// 记录传统 for 语句。对应 Java 方法 `visitTraditionalForStatement`。
    fn visit_traditional_for_statement(&mut self, ctx: &TraditionalForStatementContext) -> Self::T {
        self.expression_trace_points.push(Self::new_point(
            TraceType::Statement,
            vec![],
            ctx.for_token.symbol(),
        ));
        None
    }

    /// 记录 foreach 语句。对应 Java 方法 `visitForEachStatement`。
    fn visit_for_each_statement(&mut self, ctx: &ForEachStatementContext) -> Self::T {
        self.expression_trace_points.push(Self::new_point(
            TraceType::Statement,
            vec![],
            ctx.for_token.symbol(),
        ));
        None
    }

    /// 记录函数定义。对应 Java 方法 `visitFunctionStatement`。
    fn visit_function_statement(&mut self, ctx: &FunctionStatementContext) -> Self::T {
        if let Some(token) = ctx.var_id.start_token() {
            self.expression_trace_points.push(Self::new_point(
                TraceType::DefineFunction,
                vec![],
                token,
            ));
        }
        None
    }

    /// 记录宏定义。对应 Java 方法 `visitMacroStatement`。
    fn visit_macro_statement(&mut self, ctx: &MacroStatementContext) -> Self::T {
        if let Some(token) = ctx.var_id.start_token() {
            self.expression_trace_points.push(Self::new_point(
                TraceType::DefineMacro,
                vec![],
                token,
            ));
        }
        None
    }

    /// 记录 break/continue。对应 Java 方法 `visitBreakContinueStatement`。
    fn visit_break_continue_statement(
        &mut self,
        ctx: &crate::aparser::syntax_tree_factory::BreakContinueStatementContext,
    ) -> Self::T {
        self.expression_trace_points.push(Self::new_point(
            TraceType::Statement,
            vec![],
            ctx.token.symbol(),
        ));
        None
    }

    /// 记录 return 及其返回表达式。对应 Java 方法 `visitReturnStatement`。
    fn visit_return_statement(&mut self, ctx: &ReturnStatementContext) -> Self::T {
        let children = ctx
            .expression
            .as_deref()
            .and_then(|expression| expression.accept(self))
            .into_iter()
            .collect();
        self.expression_trace_points.push(Self::new_point(
            TraceType::Return,
            children,
            ctx.return_token.symbol(),
        ));
        None
    }

    /// 记录空语句。对应 Java 方法 `visitEmptyStatement`。
    fn visit_empty_statement(&mut self, ctx: &EmptyStatementContext) -> Self::T {
        self.expression_trace_points.push(Self::new_point(
            TraceType::Statement,
            vec![],
            ctx.token.symbol(),
        ));
        None
    }

    /// 跳过混合块中的空语句；若块内全为空，仅保留第一个。
    /// 对应 Java 方法 `visitBlockStatements`。
    fn visit_block_statements(&mut self, ctx: &BlockStatementsContext) -> Self::T {
        let non_empty: Vec<&Node> = ctx
            .statements
            .iter()
            .filter(|node| !matches!(node, Node::EmptyStatement(_)))
            .collect();
        if non_empty.is_empty() {
            if let Some(first) = ctx.statements.first() {
                first.accept(self);
            }
        } else {
            for statement in non_empty {
                statement.accept(self);
            }
        }
        None
    }

    // ==================== Expression ====================

    /// 访问赋值或三元表达式。对应 Java 方法 `visitExpression`。
    fn visit_expression(&mut self, ctx: &ExpressionContext) -> Self::T {
        if let Some(ternary) = &ctx.ternary {
            return ternary.accept(self);
        }
        let left = ctx.left.as_deref()?.accept(self)?;
        let right = ctx.expression.as_deref()?.accept(self)?;
        let token = ctx.assign_operator.as_deref()?.start_token()?;
        Some(Self::new_point(
            TraceType::Operator,
            vec![left, right],
            token,
        ))
    }

    /// 访问赋值左值的变量、函数和路径。对应 Java 方法 `visitLeftHandSide`。
    fn visit_left_hand_side(&mut self, ctx: &LeftHandSideContext) -> Self::T {
        let token = ctx.var_id.start_token()?;
        let root = if ctx.lparen.is_some() {
            Self::new_point(
                TraceType::Function,
                self.trace_argument_list(ctx.argument_list.as_deref()),
                token,
            )
        } else {
            Self::new_point(TraceType::Variable, vec![], token)
        };
        Some(self.path_parts(root, &ctx.path_parts))
    }

    /// 访问三元表达式。对应 Java 方法 `visitTernaryExpr`。
    fn visit_ternary_expr(&mut self, ctx: &TernaryExprContext) -> Self::T {
        let condition = ctx.condition.accept(self)?;
        let Some(then_expr) = &ctx.then_expr else {
            return Some(condition);
        };
        let then_point = then_expr.accept(self)?;
        let else_point = ctx.else_expr.as_deref()?.accept(self)?;
        Some(Self::new_point(
            TraceType::Operator,
            vec![condition, then_point, else_point],
            ctx.question.as_ref()?.symbol(),
        ))
    }

    /// 访问左结合二元表达式链。对应 Java 方法 `visitBaseExpr`。
    fn visit_base_expr(&mut self, ctx: &BaseExprContext) -> Self::T {
        let mut left = ctx.primary.accept(self)?;
        for left_asso in &ctx.left_assos {
            let Node::LeftAsso(left_asso) = left_asso else {
                continue;
            };
            let right = left_asso.right.accept(self)?;
            let token = left_asso.binaryop.start_token()?;
            left = Self::new_point(TraceType::Operator, vec![left, right], token);
        }
        Some(left)
    }

    /// 访问主表达式并套用前后缀操作符。对应 Java 方法 `visitPrimary`。
    fn visit_primary(&mut self, ctx: &PrimaryContext) -> Self::T {
        let mut point = if let Some(non_pathable) = &ctx.non_pathable {
            non_pathable.accept(self)?
        } else {
            let root = ctx.pathable.as_deref()?.accept(self)?;
            self.path_parts(root, &ctx.path_parts)
        };
        if let Some(suffix) = &ctx.suffix {
            point = Self::new_point(TraceType::Operator, vec![point], suffix.start_token()?);
        }
        if let Some(prefix) = &ctx.prefix {
            point = Self::new_point(TraceType::Operator, vec![point], prefix.start_token()?);
        }
        Some(point)
    }

    /// 访问常量。对应 Java 方法 `visitConstExpr`。
    fn visit_const_expr(&mut self, ctx: &ConstExprContext) -> Self::T {
        let token = ctx.literal.start_token()?;
        Some(Self::new_point_with_text(
            TraceType::Value,
            vec![],
            ctx.literal.text(),
            token,
        ))
    }

    /// cast 追踪其被转换的主表达式。对应 Java 方法 `visitCastExpr`。
    fn visit_cast_expr(&mut self, ctx: &CastExprContext) -> Self::T {
        ctx.primary.accept(self)
    }

    /// 分组表达式透传内部表达式。对应 Java 方法 `visitGroupExpr`。
    fn visit_group_expr(&mut self, ctx: &GroupExprContext) -> Self::T {
        ctx.expression.accept(self)
    }

    /// 记录对象构造表达式。对应 Java 方法 `visitNewObjExpr`。
    fn visit_new_obj_expr(&mut self, ctx: &NewObjExprContext) -> Self::T {
        Some(Self::new_point_with_text(
            TraceType::Primary,
            vec![],
            Node::NewObjExpr(ctx.clone()).text(),
            ctx.new_token.symbol(),
        ))
    }

    /// 记录未初始化数组构造表达式。对应 Java 方法 `visitNewEmptyArrExpr`。
    fn visit_new_empty_arr_expr(&mut self, ctx: &NewEmptyArrExprContext) -> Self::T {
        Some(Self::new_point_with_text(
            TraceType::Primary,
            vec![],
            Node::NewEmptyArrExpr(ctx.clone()).text(),
            ctx.new_token.symbol(),
        ))
    }

    /// 记录带初始化器的数组构造表达式。对应 Java 方法 `visitNewInitArrExpr`。
    fn visit_new_init_arr_expr(&mut self, ctx: &NewInitArrExprContext) -> Self::T {
        Some(Self::new_point_with_text(
            TraceType::Primary,
            vec![],
            Node::NewInitArrExpr(ctx.clone()).text(),
            ctx.new_token.symbol(),
        ))
    }

    /// 记录 lambda 箭头。对应 Java 方法 `visitLambdaExpr`。
    fn visit_lambda_expr(
        &mut self,
        ctx: &crate::aparser::syntax_tree_factory::LambdaExprContext,
    ) -> Self::T {
        Some(Self::new_point(
            TraceType::Primary,
            vec![],
            ctx.arrow.symbol(),
        ))
    }

    /// 访问变量或函数调用头。对应 Java 方法 `visitVarIdExpr`。
    fn visit_var_id_expr(&mut self, ctx: &VarIdExprContext) -> Self::T {
        let token = ctx.var_id.start_token()?;
        Some(if ctx.lparen.is_some() {
            Self::new_point(
                TraceType::Function,
                self.trace_argument_list(ctx.argument_list.as_deref()),
                token,
            )
        } else {
            Self::new_point(TraceType::Variable, vec![], token)
        })
    }

    /// 访问作为值的类型。对应 Java 方法 `visitTypeExpr`。
    fn visit_type_expr(&mut self, ctx: &TypeExprContext) -> Self::T {
        Self::point_for_node(TraceType::Value, vec![], &ctx.primitive_type)
    }

    /// 访问列表字面量及其元素。对应 Java 方法 `visitListExpr`。
    fn visit_list_expr(&mut self, ctx: &ListExprContext) -> Self::T {
        let children = match ctx.list_items.as_deref() {
            Some(Node::ListItems(items)) => items
                .expressions
                .iter()
                .filter_map(|expression| expression.accept(self))
                .collect(),
            _ => Vec::new(),
        };
        Some(Self::new_point(
            TraceType::List,
            children,
            ctx.lbrack.symbol(),
        ))
    }

    /// 记录 map 字面量。对应 Java 方法 `visitMapExpr`。
    fn visit_map_expr(&mut self, ctx: &MapExprContext) -> Self::T {
        Some(Self::new_point(TraceType::Map, vec![], ctx.lbrace.symbol()))
    }

    /// 访问块表达式。对应 Java 方法 `visitBlockExpr`。
    fn visit_block_expr(&mut self, ctx: &BlockExprContext) -> Self::T {
        Some(self.block_point(ctx.block_statements.as_deref(), ctx.lbrace.symbol()))
    }

    /// 访问 if/then/else 表达式。对应 Java 方法 `visitQlIf`。
    fn visit_ql_if(&mut self, ctx: &QlIfContext) -> Self::T {
        let mut children = vec![ctx.condition.accept(self)?, ctx.then_body.accept(self)?];
        if let Some(else_body) = &ctx.else_body {
            if let Some(point) = else_body.accept(self) {
                children.push(point);
            }
        }
        Some(Self::new_point_with_text(
            TraceType::If,
            children,
            "if",
            ctx.if_token.symbol(),
        ))
    }

    /// 访问 then 分支。对应 Java 方法 `visitThenBody`。
    fn visit_then_body(&mut self, ctx: &ThenBodyContext) -> Self::T {
        if let Some(block_statements) = &ctx.block_statements {
            let anchor = ctx
                .lbrace
                .as_ref()
                .map(|node| node.symbol())
                .or_else(|| block_statements.start_token())?;
            return Some(self.block_point(Some(block_statements), anchor));
        }
        if let Some(statement) = &ctx.non_expression_statement {
            return self.visit_non_expression_statement_internal(statement);
        }
        if let Some(expression) = &ctx.expression {
            return expression.accept(self);
        }
        ctx.lbrace
            .as_ref()
            .map(|anchor| Self::new_point(TraceType::Block, vec![], anchor.symbol()))
    }

    /// 访问 else 分支。对应 Java 方法 `visitElseBody`。
    fn visit_else_body(&mut self, ctx: &ElseBodyContext) -> Self::T {
        if let Some(block_statements) = &ctx.block_statements {
            let anchor = ctx
                .lbrace
                .as_ref()
                .map(|node| node.symbol())
                .or_else(|| block_statements.start_token())?;
            return Some(self.block_point(Some(block_statements), anchor));
        }
        if let Some(ql_if) = &ctx.ql_if {
            return ql_if.accept(self);
        }
        if let Some(statement) = &ctx.non_expression_statement {
            return self.visit_non_expression_statement_internal(statement);
        }
        if let Some(expression) = &ctx.expression {
            return expression.accept(self);
        }
        ctx.lbrace
            .as_ref()
            .map(|anchor| Self::new_point(TraceType::Block, vec![], anchor.symbol()))
    }

    /// 访问 switch 表达式、case 标签和分支体。对应 Java 方法 `visitSwitchExpr`。
    fn visit_switch_expr(&mut self, ctx: &SwitchExprContext) -> Self::T {
        let mut children = Vec::new();
        if let Some(point) = ctx.expression.accept(self) {
            children.push(point);
        }
        if let Some(Node::SwitchCaseGroups(groups)) = ctx.groups.as_deref() {
            for group in &groups.groups {
                match group {
                    Node::SwitchStatementGroup(group) => {
                        if let Node::SwitchLabels(labels) = group.labels.as_ref() {
                            for label in &labels.labels {
                                if let Node::SwitchLabel(label) = label {
                                    if let Some(expression) = &label.expression {
                                        if let Some(point) = expression.accept(self) {
                                            children.push(point);
                                        }
                                    }
                                }
                            }
                        }
                        if let Some(block_statements) = &group.block_statements {
                            if let Some(anchor) = block_statements.start_token() {
                                children.push(self.block_point(Some(block_statements), anchor));
                            }
                        }
                    }
                    Node::SwitchExprGroup(group) => {
                        if let Node::SwitchExpressionLabel(label) = group.label.as_ref() {
                            if let Some(Node::ExpressionList(expressions)) =
                                label.expression_list.as_deref()
                            {
                                children.extend(
                                    expressions
                                        .expressions
                                        .iter()
                                        .filter_map(|expression| expression.accept(self)),
                                );
                            }
                        }
                        if let Some(point) = group.expression.accept(self) {
                            children.push(point);
                        }
                    }
                    _ => {}
                }
            }
        }
        Some(Self::new_point(
            TraceType::Switch,
            children,
            ctx.switch_token.symbol(),
        ))
    }

    /// try/catch 作为一个不可拆分的主表达式追踪。对应 Java 方法
    /// `visitTryCatchExpr`。
    fn visit_try_catch_expr(&mut self, ctx: &TryCatchExprContext) -> Self::T {
        Some(Self::new_point(
            TraceType::Primary,
            vec![],
            ctx.try_token.symbol(),
        ))
    }

    /// 上下文选择器作为一个不可拆分的主表达式追踪。对应 Java 方法
    /// `visitContextSelectExpr`。
    fn visit_context_select_expr(&mut self, ctx: &ContextSelectExprContext) -> Self::T {
        Some(Self::new_point(
            TraceType::Primary,
            vec![],
            ctx.selector_start.symbol(),
        ))
    }

    /// if/else 内部的非表达式语句不应污染顶层列表。
    fn visit_non_expression_statement(&mut self, ctx: &NonExpressionStatementContext) -> Self::T {
        self.visit_non_expression_statement_internal(&ctx.statement)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::aparser::syntax_tree_factory::{
        BaseExprContext, BoolenLiteralContext, ConstExprContext, LiteralContext, PrimaryContext,
        TernaryExprContext,
    };
    use crate::aparser::terminal_node::TerminalNode;
    use crate::aparser::token;

    fn terminal(token_type: u16, text: &str, position: i32) -> TerminalNode {
        TerminalNode::new(Token::new(
            token_type as i32,
            text,
            position,
            position,
            1,
            position,
        ))
    }

    #[test]
    fn visits_constant_expression() {
        let tree = Node::TernaryExpr(TernaryExprContext {
            condition: Box::new(Node::BaseExpr(BaseExprContext {
                primary: Box::new(Node::Primary(PrimaryContext {
                    prefix: None,
                    pathable: None,
                    path_parts: vec![],
                    suffix: None,
                    non_pathable: Some(Box::new(Node::ConstExpr(ConstExprContext {
                        literal: Box::new(Node::Literal(LiteralContext {
                            token: None,
                            boolen: Some(Box::new(Node::BoolenLiteral(BoolenLiteralContext {
                                token: terminal(token::TRUE, "true", 0),
                            }))),
                            double_quote_string: None,
                        })),
                    }))),
                })),
                left_assos: vec![],
            })),
            question: None,
            then_expr: None,
            else_expr: None,
        });

        let point = tree
            .accept(&mut TraceExpressionVisitor::new())
            .expect("constant trace point");
        assert_eq!(TraceType::Value, point.trace_type());
        assert_eq!("true", point.token());
        assert_eq!(0, point.position());
    }
}
