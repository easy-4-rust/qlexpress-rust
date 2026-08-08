/// `SOURCE_PARITY`：Java `RuleContext#getText()` 会拼接
/// `expectInto/consumeNode` 写入的全部标点，同时不包含
/// `consumeNextStatement` 和 import 路径中以普通 `consume` 跳过的 token。
    #[test]
    fn rule_context_text_preserves_exact_java_children() {
        let cases = [
            ("while (a) { b = [1, 2,]; }", "while(a){b=[1,2,]}"),
            (
                "function f(int a, b) { return a ? b : 0; }",
                "functionf(inta,b){returna?b:0}",
            ),
            ("x = {'a': 1, b: 2,};", "x={'a':1,b:2,}"),
            ("x = new Foo(1, 2).bar();", "x=newFoo(1,2).bar()"),
            (
                "try { x = 1; } catch (A | B e) { x = 2; } finally { x = 3; }",
                "try{x=1}catch(A|Be){x=2}finally{x=3}",
            ),
            ("f = (a, b) -> { return a; };", "f=(a,b)->{returna}"),
            ("x = \"${a}\";", "x=\"${a}\""),
            ("import a.b.C; x = 1;", "importabC;x=1"),
        ];

        for (script, expected) in cases {
            assert_eq!(parse(script).text(), expected, "script: {script}");
        }
    }

    /// `SOURCE_PARITY`：覆盖 Java `RuleContext#isEmpty/getChildCount/getChild/
    /// getRuleContexts/tokenNode/tokenNodes/getStart/getStop/toStringTree`。
    /// Rust 解析器在构造强类型 AST 时完成 Java `addChild/addToken/setStart/
    /// setStop` 的职责，所有查询必须观察到完整的括号和规则孩子。
    #[test]
    fn rule_context_queries_include_punctuation_and_bounds() {
        let tree = parse("(a);");
        let expression = expr_statement(&statements(&tree)[0]);
        let group = match primary_of(base_expr(expression)).pathable.as_deref() {
            Some(Node::GroupExpr(group)) => group,
            other => panic!("expected group expression, got {other:?}"),
        };
        let group_node = Node::GroupExpr(group.clone());

        assert!(!group_node.is_empty());
        assert_eq!(group_node.child_count(), 3);
        assert_eq!(group_node.child(0).text(), "(");
        assert_eq!(group_node.child(2).text(), ")");
        assert_eq!(
            group_node
                .rule_child(0, |_| true)
                .map(Node::text)
                .as_deref(),
            Some("a")
        );
        assert_eq!(group_node.rule_contexts(|_| true).len(), 1);
        assert_eq!(
            group_node.token_node(LPAREN).map(TerminalNode::text),
            Some("(")
        );
        assert_eq!(group_node.token_nodes(LPAREN).len(), 1);
        assert_eq!(group_node.token_nodes(RPAREN).len(), 1);
        assert_eq!(group_node.start_token().map(Token::text), Some("("));
        assert_eq!(group_node.stop_token().map(Token::text), Some(")"));
        assert!(group_node.to_string_tree().starts_with("(GroupExpr ("));
    }

    // ------------------------------------------------------------------
    // Statements
    // ------------------------------------------------------------------

    #[test]
    fn parses_while_statement() {
        let tree = parse("while (i < 10) { i = i + 1; }");
        match &statements(&tree)[0] {
            Node::WhileStatement(w) => {
                assert_eq!(w.while_token.text(), "while");
                assert!(matches!(
                    w.block_statements.as_deref(),
                    Some(Node::BlockStatements(_))
                ));
            }
            other => panic!("expected while, got {other:?}"),
        }
    }

    #[test]
    fn parses_traditional_for() {
        let tree = parse("for (int i = 0; i < 10; i = i + 1) { sum = sum + i; }");
        match &statements(&tree)[0] {
            Node::TraditionalForStatement(f) => {
                assert!(matches!(
                    f.for_init.as_ref(),
                    Node::ForInit(init) if init.local_variable_declaration.is_some()
                ));
                assert!(f.for_condition.is_some());
                assert!(f.for_update.is_some());
            }
            other => panic!("expected for, got {other:?}"),
        }
    }

    #[test]
    fn parses_for_each_with_and_without_type() {
        let tree = parse("for (int x : xs) { s = s + x; }");
        match &statements(&tree)[0] {
            Node::ForEachStatement(f) => {
                assert!(f.decl_type.is_some());
                assert_eq!(f.var_id.text(), "x");
            }
            other => panic!("expected foreach, got {other:?}"),
        }
        let tree = parse("for (x : xs) { s = s + x; }");
        match &statements(&tree)[0] {
            Node::ForEachStatement(f) => assert!(f.decl_type.is_none()),
            other => panic!("expected foreach, got {other:?}"),
        }
    }

    #[test]
    fn parses_function_and_macro() {
        let tree = parse("function add(int a, int b) { return a + b; }");
        match &statements(&tree)[0] {
            Node::FunctionStatement(f) => {
                assert_eq!(f.var_id.text(), "add");
                match f.params.as_deref() {
                    Some(Node::FormalOrInferredParameterList(list)) => {
                        assert_eq!(list.params.len(), 2)
                    }
                    other => panic!("expected params, got {other:?}"),
                }
            }
            other => panic!("expected function, got {other:?}"),
        }
        let tree = parse("macro inc { a = a + 1; }");
        match &statements(&tree)[0] {
            Node::MacroStatement(m) => assert_eq!(m.var_id.text(), "inc"),
            other => panic!("expected macro, got {other:?}"),
        }
    }

    #[test]
    fn parses_throw_break_continue_return() {
        let tree = parse("throw 'err';");
        assert!(matches!(statements(&tree)[0], Node::ThrowStatement(_)));
        let tree = parse("while (true) { break; continue; }");
        match &statements(&tree)[0] {
            Node::WhileStatement(w) => {
                let body = block_statements(w.block_statements.as_deref().unwrap());
                assert!(matches!(body[0], Node::BreakContinueStatement(ref b) if b.is_break()));
                assert!(matches!(body[1], Node::BreakContinueStatement(ref b) if !b.is_break()));
            }
            other => panic!("expected while, got {other:?}"),
        }
        let tree = parse("return 1;");
        match &statements(&tree)[0] {
            Node::ReturnStatement(r) => assert!(r.expression.is_some()),
            other => panic!("expected return, got {other:?}"),
        }
        let tree = parse("return;");
        match &statements(&tree)[0] {
            Node::ReturnStatement(r) => assert!(r.expression.is_none()),
            other => panic!("expected return, got {other:?}"),
        }
    }

    // statements of a BlockStatements node
    fn block_statements(block: &Node) -> &[Node] {
        match block {
            Node::BlockStatements(b) => &b.statements,
            other => panic!("expected block statements, got {other:?}"),
        }
    }

    #[test]
    fn parses_local_variable_declaration_with_type() {
        let tree = parse("int a = 1, b = 2;");
        match &statements(&tree)[0] {
            Node::LocalVariableDeclarationStatement(s) => {
                match s.local_variable_declaration.as_ref() {
                    Node::LocalVariableDeclaration(decl) => {
                        match decl.variable_declarator_list.as_ref() {
                            Node::VariableDeclaratorList(list) => {
                                assert_eq!(list.variables.len(), 2);
                                match &list.variables[0] {
                                    Node::VariableDeclarator(declarator) => {
                                        assert!(matches!(
                                            declarator.id.as_ref(),
                                            Node::VariableDeclaratorId(_)
                                        ));
                                        assert!(matches!(
                                            declarator.initializer.as_deref(),
                                            Some(Node::VariableInitializer(_))
                                        ));
                                    }
                                    other => {
                                        panic!("expected variable declarator, got {other:?}")
                                    }
                                }
                            }
                            other => panic!("expected declarator list, got {other:?}"),
                        }
                    }
                    other => panic!("expected local decl, got {other:?}"),
                }
            }
            other => panic!("expected local decl statement, got {other:?}"),
        }
    }

    #[test]
    fn var_declaration_without_type_is_assign_or_ref() {
        // `a = 1` is an assignment expression, not a declaration.
        let tree = parse("a = 1;");
        let expr = expr_statement(&statements(&tree)[0]);
        assert!(expr.is_assign());
        // `a.b = 1` assigns through a path.
        let tree = parse("a.b = 1;");
        let expr = expr_statement(&statements(&tree)[0]);
        match expr.left.as_deref() {
            Some(Node::LeftHandSide(l)) => assert_eq!(l.path_parts.len(), 1),
            other => panic!("expected left hand side, got {other:?}"),
        }
    }

    #[test]
    fn parses_if_else_chain() {
        let tree = parse("if (a > 1) { x = 1; } else if (a > 0) { x = 2; } else { x = 3; }");
        let expr = expr_statement(&statements(&tree)[0]);
        let primary = primary_of(base_expr(expr));
        match primary.non_pathable.as_deref() {
            Some(Node::QlIf(ql_if)) => {
                assert!(ql_if.else_body.is_some());
                match ql_if.else_body.as_deref() {
                    Some(Node::ElseBody(e)) => {
                        assert!(matches!(e.ql_if.as_deref(), Some(Node::QlIf(_))));
                    }
                    other => panic!("expected else body, got {other:?}"),
                }
            }
            other => panic!("expected ql if, got {other:?}"),
        }
    }

    #[test]
    fn parses_if_then_expression_body() {
        let tree = parse("if (a) then x = 1 else x = 2;");
        let expr = expr_statement(&statements(&tree)[0]);
        let primary = primary_of(base_expr(expr));
        match primary.non_pathable.as_deref() {
            Some(Node::QlIf(ql_if)) => {
                assert!(ql_if.then_keyword.is_some());
                assert!(matches!(
                    ql_if.then_body.as_ref(),
                    Node::ThenBody(t) if t.expression.is_some()
                ));
            }
            other => panic!("expected ql if, got {other:?}"),
        }
    }

    #[test]
    fn parses_switch_statement_and_expr_groups() {
        let tree = parse("switch (x) { case 1: a = 1; break; default: a = 0; }");
        let expr = expr_statement(&statements(&tree)[0]);
        let primary = primary_of(base_expr(expr));
        match primary.non_pathable.as_deref() {
            Some(Node::SwitchExpr(s)) => match s.groups.as_deref() {
                Some(Node::SwitchCaseGroups(groups)) => {
                    assert_eq!(groups.groups.len(), 2);
                    match &groups.groups[0] {
                        Node::SwitchStatementGroup(group) => {
                            match group.labels.as_ref() {
                                Node::SwitchLabels(labels) => {
                                    assert_eq!(labels.labels.len(), 1)
                                }
                                other => panic!("expected switch labels, got {other:?}"),
                            }
                            assert!(matches!(
                                group.block_statements.as_deref(),
                                Some(Node::BlockStatements(_))
                            ));
                        }
                        other => panic!("expected statement group, got {other:?}"),
                    }
                }
                other => panic!("expected groups, got {other:?}"),
            },
            other => panic!("expected switch, got {other:?}"),
        }

        let tree = parse("y = switch (x) { case 1 -> 10\n case 2, 3 -> 20\n default -> 0\n };");
        let expr = expr_statement(&statements(&tree)[0]);
        assert!(expr.is_assign());
        let rhs = match expr.expression.as_deref() {
            Some(Node::Expression(e)) => e,
            other => panic!("expected rhs, got {other:?}"),
        };
        let primary = primary_of(base_expr(rhs));
        match primary.non_pathable.as_deref() {
            Some(Node::SwitchExpr(s)) => match s.groups.as_deref() {
                Some(Node::SwitchCaseGroups(groups)) => {
                    assert_eq!(groups.groups.len(), 3);
                    match &groups.groups[0] {
                        Node::SwitchExprGroup(group) => {
                            assert!(matches!(
                                group.label.as_ref(),
                                Node::SwitchExpressionLabel(_)
                            ));
                            assert!(matches!(group.expression.as_ref(), Node::Expression(_)));
                        }
                        other => panic!("expected expression group, got {other:?}"),
                    }
                    assert!(matches!(groups.groups[2], Node::SwitchExprGroup(_)));
                }
                other => panic!("expected groups, got {other:?}"),
            },
            other => panic!("expected switch, got {other:?}"),
        }
    }

    #[test]
    fn parses_try_catch_finally() {
        let tree = parse("try { a(); } catch (IOException | RuntimeException e) { b(); } catch (e) { c(); } finally { d(); }");
        let expr = expr_statement(&statements(&tree)[0]);
        let primary = primary_of(base_expr(expr));
        match primary.non_pathable.as_deref() {
            Some(Node::TryCatchExpr(t)) => {
                match t.try_catches.as_deref() {
                    Some(Node::TryCatches(catches)) => {
                        assert_eq!(catches.catches.len(), 2);
                        match &catches.catches[0] {
                            Node::TryCatch(c) => match c.catch_params.as_ref() {
                                Node::CatchParams(p) => assert_eq!(p.decl_types.len(), 2),
                                other => panic!("expected catch params, got {other:?}"),
                            },
                            other => panic!("expected catch, got {other:?}"),
                        }
                        match &catches.catches[1] {
                            Node::TryCatch(c) => match c.catch_params.as_ref() {
                                Node::CatchParams(p) => assert!(p.decl_types.is_empty()),
                                other => panic!("expected catch params, got {other:?}"),
                            },
                            other => panic!("expected catch, got {other:?}"),
                        }
                    }
                    other => panic!("expected catches, got {other:?}"),
                }
                assert!(t.try_finally.is_some());
            }
            other => panic!("expected try, got {other:?}"),
        }
    }

    #[test]
    fn parses_imports() {
        let tree = parse("import java.util.HashMap;\nimport java.io.*;\nx = 1;");
        match &tree {
            Node::Program(program) => {
                assert_eq!(program.imports.len(), 2);
                assert!(matches!(program.imports[0], Node::ImportCls(_)));
                assert!(matches!(program.imports[1], Node::ImportPack(_)));
                if let Node::ImportCls(import) = &program.imports[0] {
                    let names: Vec<String> = import.var_ids.iter().map(|id| id.text()).collect();
                    assert_eq!(names, ["java", "util", "HashMap"]);
                }
            }
            other => panic!("expected program, got {other:?}"),
        }
    }

    #[test]
    fn import_not_at_beginning_is_error() {
        let err = parse_err("x = 1;\nimport java.util.HashMap;");
        assert_eq!(err.error_code(), error_codes::SYNTAX_ERROR);
    }
