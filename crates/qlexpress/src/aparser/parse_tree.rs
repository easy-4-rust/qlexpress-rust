//! 语法树通用操作,对应 Java `com.alibaba.qlexpress4.aparser.ParseTree`。
//! 职责:Java ParseTree 接口方法(getText / 子节点访问 / accept)在 Rust 侧的落点,
//! 实现于 [`Node`] 的固有 impl(Java 的 RuleContext 边界计算 getStart/getStop 亦在此)。
//! 本文件由 `syntax_tree.rs` 拆分而来(SPEC §5.5 一类一文件),仅移动代码与补充中文注释,行为完全一致。

use super::qlparser_base_visitor::Visitor;
use super::rule_context::{ChildRef, HasChildren};
use super::syntax_tree_factory::Node;
use super::token::Token;

impl Node {
    /// 按源码顺序获取孩子。对应 Java 方法 `RuleContext.children`。
    /// Children in source order (Java `RuleContext.children`).
    pub fn children(&self) -> Vec<ChildRef<'_>> {
        <Self as HasChildren>::children(self)
    }

    /// 节点文本(孩子文本拼接)。对应 Java 方法 `ParseTree.getText`。
    /// Java `ParseTree.getText`: concatenation of every stored child text,
    /// including punctuation retained by `expectInto`/`consumeNode`.
    pub fn text(&self) -> String {
        let mut out = String::new();
        for child in self.children() {
            out.push_str(&child.text());
        }
        out
    }

    /// 判断当前规则节点是否没有孩子。对应 Java `RuleContext#isEmpty()`。
    pub fn is_empty(&self) -> bool {
        self.children().is_empty()
    }

    /// 返回直接孩子数量。对应 Java `RuleContext#getChildCount()`。
    pub fn child_count(&self) -> usize {
        self.children().len()
    }

    /// 返回指定位置的直接孩子。
    ///
    /// 与 Java `RuleContext#getChild(int)` 相同，索引越界时会 panic
    /// （对应 Java `IndexOutOfBoundsException`）。
    pub fn child(&self, index: usize) -> ChildRef<'_> {
        self.children()[index]
    }

    /// 返回满足谓词的第 `index` 个规则孩子。
    ///
    /// 这是 Java `RuleContext#getChild(Class<T>, int)` 的 Rust 类型安全
    /// 适配；未找到时返回 `None`，对应 Java 的 `null`。
    pub fn rule_child(
        &self,
        index: usize,
        mut predicate: impl FnMut(&Node) -> bool,
    ) -> Option<&Node> {
        self.children()
            .into_iter()
            .filter_map(|child| match child {
                ChildRef::Node(node) if predicate(node) => Some(node),
                _ => None,
            })
            .nth(index)
    }

    /// 返回全部满足谓词的规则孩子。
    ///
    /// 对应 Java `RuleContext#getRuleContexts(Class<T>)`。
    pub fn rule_contexts(
        &self,
        mut predicate: impl FnMut(&Node) -> bool,
    ) -> Vec<&Node> {
        self.children()
            .into_iter()
            .filter_map(|child| match child {
                ChildRef::Node(node) if predicate(node) => Some(node),
                _ => None,
            })
            .collect()
    }

    /// 返回第一个指定 token 类型的直接终结符。
    ///
    /// 对应 Java `RuleContext#tokenNode(int)`。
    pub fn token_node(&self, token_type: i32) -> Option<&super::terminal_node::TerminalNode> {
        self.children().into_iter().find_map(|child| match child {
            ChildRef::Term(term) if term.symbol().token_type() == token_type => Some(term),
            _ => None,
        })
    }

    /// 返回全部指定 token 类型的直接终结符。
    ///
    /// 对应 Java `RuleContext#tokenNodes(int)`。
    pub fn token_nodes(&self, token_type: i32) -> Vec<&super::terminal_node::TerminalNode> {
        self.children()
            .into_iter()
            .filter_map(|child| match child {
                ChildRef::Term(term) if term.symbol().token_type() == token_type => Some(term),
                _ => None,
            })
            .collect()
    }

    /// 节点覆盖的第一个 token。对应 Java 方法 `RuleContext.getStart`。
    /// First token covered by this node (Java `RuleContext.getStart`).
    pub fn start_token(&self) -> Option<&Token> {
        self.children().into_iter().find_map(|c| c.start_token())
    }

    /// 节点覆盖的最后一个 token。对应 Java 方法 `RuleContext.getStop`。
    /// Last token covered by this node (Java `RuleContext.getStop`).
    pub fn stop_token(&self) -> Option<&Token> {
        self.children()
            .into_iter()
            .rev()
            .find_map(|c| c.stop_token())
    }

    /// 第一个 token 的 1 起始行号(若有)。Java 无同名方法(Rust 便捷方法)。
    /// 1-based line of the first token, if any.
    /// 对应 Java: com.alibaba.qlexpress4.aparser.ParseTree#line。
    pub fn line(&self) -> Option<i32> {
        self.start_token().map(Token::line)
    }

    /// 以 Java `RuleContext#toStringTree()` 格式输出语法树。
    pub fn to_string_tree(&self) -> String {
        let children = self.children();
        if children.is_empty() {
            return self.text();
        }
        let mut output = format!("({}", self.rule_name());
        for child in children {
            output.push(' ');
            match child {
                ChildRef::Node(node) => output.push_str(&node.to_string_tree()),
                ChildRef::Term(term) => output.push_str(term.text()),
            }
        }
        output.push(')');
        output
    }

    /// 返回 Java `*Context` 去掉 `Context` 后的规则名称。
    fn rule_name(&self) -> &'static str {
        macro_rules! rule_name {
            ($($variant:ident),* $(,)?) => {
                match self {
                    $(Node::$variant(_) => stringify!($variant)),*
                }
            };
        }
        rule_name!(
            Program,
            BlockStatements,
            LocalVariableDeclarationStatement,
            ThrowStatement,
            WhileStatement,
            TraditionalForStatement,
            ForEachStatement,
            FunctionStatement,
            MacroStatement,
            BreakContinueStatement,
            ReturnStatement,
            EmptyStatement,
            ExpressionStatement,
            NonExpressionStatement,
            LocalVariableDeclaration,
            ForInit,
            VariableDeclaratorList,
            VariableDeclarator,
            VariableDeclaratorId,
            VariableInitializer,
            ArrayInitializer,
            VariableInitializerList,
            DeclType,
            DeclTypeNoArr,
            PrimitiveType,
            ClsType,
            Dims,
            DimExprs,
            Expression,
            LeftHandSide,
            AssignOperator,
            TernaryExpr,
            BaseExpr,
            LeftAsso,
            Binaryop,
            Primary,
            PrefixExpress,
            SuffixExpress,
            ConstExpr,
            CastExpr,
            GroupExpr,
            NewObjExpr,
            NewEmptyArrExpr,
            NewInitArrExpr,
            VarIdExpr,
            TypeExpr,
            ListExpr,
            ListItems,
            MapExpr,
            BlockExpr,
            ContextSelectExpr,
            QlIf,
            ThenBody,
            ElseBody,
            SwitchExpr,
            SwitchCaseGroups,
            SwitchStatementGroup,
            SwitchExprGroup,
            SwitchLabels,
            SwitchLabel,
            SwitchExpressionLabel,
            ExpressionList,
            TryCatchExpr,
            TryCatches,
            TryCatch,
            CatchParams,
            TryFinally,
            MapEntries,
            MapEntry,
            ClsValue,
            EValue,
            IdKey,
            StringKey,
            QuoteStringKey,
            MethodInvoke,
            FieldAccess,
            MethodAccess,
            IndexExpr,
            CustomPath,
            FieldId,
            SingleIndex,
            SliceIndex,
            ArgumentList,
            Literal,
            DoubleQuoteStringLiteral,
            StringExpression,
            BoolenLiteral,
            LambdaExpr,
            LambdaParameters,
            FormalOrInferredParameterList,
            FormalOrInferredParameter,
            ImportCls,
            ImportPack,
            OpId,
            VarId
        )
    }

    /// 双分派到类型化 Visitor 方法。对应 Java 方法 `*Context.accept`。
    /// Dispatch to the typed visitor method, mirroring Java
    /// `*Context.accept`.
    pub fn accept<V: Visitor + ?Sized>(&self, visitor: &mut V) -> V::T {
        match self {
            Node::Program(c) => visitor.visit_program(c),
            Node::BlockStatements(c) => visitor.visit_block_statements(c),
            Node::LocalVariableDeclarationStatement(c) => {
                visitor.visit_local_variable_declaration_statement(c)
            }
            Node::ThrowStatement(c) => visitor.visit_throw_statement(c),
            Node::WhileStatement(c) => visitor.visit_while_statement(c),
            Node::TraditionalForStatement(c) => visitor.visit_traditional_for_statement(c),
            Node::ForEachStatement(c) => visitor.visit_for_each_statement(c),
            Node::FunctionStatement(c) => visitor.visit_function_statement(c),
            Node::MacroStatement(c) => visitor.visit_macro_statement(c),
            Node::BreakContinueStatement(c) => visitor.visit_break_continue_statement(c),
            Node::ReturnStatement(c) => visitor.visit_return_statement(c),
            Node::EmptyStatement(c) => visitor.visit_empty_statement(c),
            Node::ExpressionStatement(c) => visitor.visit_expression_statement(c),
            Node::NonExpressionStatement(c) => visitor.visit_non_expression_statement(c),
            Node::LocalVariableDeclaration(c) => visitor.visit_local_variable_declaration(c),
            Node::ForInit(c) => visitor.visit_for_init(c),
            Node::VariableDeclaratorList(c) => visitor.visit_variable_declarator_list(c),
            Node::VariableDeclarator(c) => visitor.visit_variable_declarator(c),
            Node::VariableDeclaratorId(c) => visitor.visit_variable_declarator_id(c),
            Node::VariableInitializer(c) => visitor.visit_variable_initializer(c),
            Node::ArrayInitializer(c) => visitor.visit_array_initializer(c),
            Node::VariableInitializerList(c) => visitor.visit_variable_initializer_list(c),
            Node::DeclType(c) => visitor.visit_decl_type(c),
            Node::DeclTypeNoArr(c) => visitor.visit_decl_type_no_arr(c),
            Node::PrimitiveType(c) => visitor.visit_primitive_type(c),
            Node::ClsType(c) => visitor.visit_cls_type(c),
            Node::Dims(c) => visitor.visit_dims(c),
            Node::DimExprs(c) => visitor.visit_dim_exprs(c),
            Node::Expression(c) => visitor.visit_expression(c),
            Node::LeftHandSide(c) => visitor.visit_left_hand_side(c),
            Node::AssignOperator(c) => visitor.visit_assign_operator(c),
            Node::TernaryExpr(c) => visitor.visit_ternary_expr(c),
            Node::BaseExpr(c) => visitor.visit_base_expr(c),
            Node::LeftAsso(c) => visitor.visit_left_asso(c),
            Node::Binaryop(c) => visitor.visit_binaryop(c),
            Node::Primary(c) => visitor.visit_primary(c),
            Node::PrefixExpress(c) => visitor.visit_prefix_express(c),
            Node::SuffixExpress(c) => visitor.visit_suffix_express(c),
            Node::ConstExpr(c) => visitor.visit_const_expr(c),
            Node::CastExpr(c) => visitor.visit_cast_expr(c),
            Node::GroupExpr(c) => visitor.visit_group_expr(c),
            Node::NewObjExpr(c) => visitor.visit_new_obj_expr(c),
            Node::NewEmptyArrExpr(c) => visitor.visit_new_empty_arr_expr(c),
            Node::NewInitArrExpr(c) => visitor.visit_new_init_arr_expr(c),
            Node::VarIdExpr(c) => visitor.visit_var_id_expr(c),
            Node::TypeExpr(c) => visitor.visit_type_expr(c),
            Node::ListExpr(c) => visitor.visit_list_expr(c),
            Node::ListItems(c) => visitor.visit_list_items(c),
            Node::MapExpr(c) => visitor.visit_map_expr(c),
            Node::BlockExpr(c) => visitor.visit_block_expr(c),
            Node::ContextSelectExpr(c) => visitor.visit_context_select_expr(c),
            Node::QlIf(c) => visitor.visit_ql_if(c),
            Node::ThenBody(c) => visitor.visit_then_body(c),
            Node::ElseBody(c) => visitor.visit_else_body(c),
            Node::SwitchExpr(c) => visitor.visit_switch_expr(c),
            Node::SwitchCaseGroups(c) => visitor.visit_switch_case_groups(c),
            Node::SwitchStatementGroup(c) => visitor.visit_switch_statement_group(c),
            Node::SwitchExprGroup(c) => visitor.visit_switch_expr_group(c),
            Node::SwitchLabels(c) => visitor.visit_switch_labels(c),
            Node::SwitchLabel(c) => visitor.visit_switch_label(c),
            Node::SwitchExpressionLabel(c) => visitor.visit_switch_expression_label(c),
            Node::ExpressionList(c) => visitor.visit_expression_list(c),
            Node::TryCatchExpr(c) => visitor.visit_try_catch_expr(c),
            Node::TryCatches(c) => visitor.visit_try_catches(c),
            Node::TryCatch(c) => visitor.visit_try_catch(c),
            Node::CatchParams(c) => visitor.visit_catch_params(c),
            Node::TryFinally(c) => visitor.visit_try_finally(c),
            Node::MapEntries(c) => visitor.visit_map_entries(c),
            Node::MapEntry(c) => visitor.visit_map_entry(c),
            Node::ClsValue(c) => visitor.visit_cls_value(c),
            Node::EValue(c) => visitor.visit_e_value(c),
            Node::IdKey(c) => visitor.visit_id_key(c),
            Node::StringKey(c) => visitor.visit_string_key(c),
            Node::QuoteStringKey(c) => visitor.visit_quote_string_key(c),
            Node::MethodInvoke(c) => visitor.visit_method_invoke(c),
            Node::FieldAccess(c) => visitor.visit_field_access(c),
            Node::MethodAccess(c) => visitor.visit_method_access(c),
            Node::IndexExpr(c) => visitor.visit_index_expr(c),
            Node::CustomPath(c) => visitor.visit_custom_path(c),
            Node::FieldId(c) => visitor.visit_field_id(c),
            Node::SingleIndex(c) => visitor.visit_single_index(c),
            Node::SliceIndex(c) => visitor.visit_slice_index(c),
            Node::ArgumentList(c) => visitor.visit_argument_list(c),
            Node::Literal(c) => visitor.visit_literal(c),
            Node::DoubleQuoteStringLiteral(c) => visitor.visit_double_quote_string_literal(c),
            Node::StringExpression(c) => visitor.visit_string_expression(c),
            Node::BoolenLiteral(c) => visitor.visit_boolen_literal(c),
            Node::LambdaExpr(c) => visitor.visit_lambda_expr(c),
            Node::LambdaParameters(c) => visitor.visit_lambda_parameters(c),
            Node::FormalOrInferredParameterList(c) => {
                visitor.visit_formal_or_inferred_parameter_list(c)
            }
            Node::FormalOrInferredParameter(c) => visitor.visit_formal_or_inferred_parameter(c),
            Node::ImportCls(c) => visitor.visit_import_cls(c),
            Node::ImportPack(c) => visitor.visit_import_pack(c),
            Node::OpId(c) => visitor.visit_op_id(c),
            Node::VarId(c) => visitor.visit_var_id(c),
        }
    }
}
