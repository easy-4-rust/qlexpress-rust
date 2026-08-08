    #[test]
    fn parses_custom_path() {
        let tree = parse("x = a .* 'path';");
        let expr = expr_statement(&statements(&tree)[0]);
        let rhs = match expr.expression.as_deref() {
            Some(Node::Expression(e)) => e,
            _other => panic!(),
        };
        let primary = primary_of(base_expr(rhs));
        match &primary.path_parts[0] {
            Node::CustomPath(c) => assert_eq!(c.path_text, "path"),
            other => panic!("expected custom path, got {other:?}"),
        }
    }

    #[test]
    fn parses_lambdas() {
        let tree = parse("f = x -> x * 2;");
        let expr = expr_statement(&statements(&tree)[0]);
        let rhs = match expr.expression.as_deref() {
            Some(Node::Expression(e)) => e,
            _other => panic!(),
        };
        let primary = primary_of(base_expr(rhs));
        match primary.non_pathable.as_deref() {
            Some(Node::LambdaExpr(l)) => {
                assert!(matches!(
                    l.lambda_parameters.as_ref(),
                    Node::LambdaParameters(p) if p.var_id.is_some()
                ));
                assert!(l.expression.is_some());
            }
            other => panic!("expected lambda, got {other:?}"),
        }

        let tree = parse("f = (a, b) -> { return a + b; };");
        let expr = expr_statement(&statements(&tree)[0]);
        let rhs = match expr.expression.as_deref() {
            Some(Node::Expression(e)) => e,
            _other => panic!(),
        };
        let primary = primary_of(base_expr(rhs));
        match primary.non_pathable.as_deref() {
            Some(Node::LambdaExpr(l)) => {
                match l.lambda_parameters.as_ref() {
                    Node::LambdaParameters(p) => match p.params.as_deref() {
                        Some(Node::FormalOrInferredParameterList(list)) => {
                            assert_eq!(list.params.len(), 2)
                        }
                        other => panic!("expected params, got {other:?}"),
                    },
                    other => panic!("expected lambda params, got {other:?}"),
                }
                assert!(l.block_statements.is_some());
            }
            other => panic!("expected lambda, got {other:?}"),
        }
    }

    #[test]
    fn parses_string_interpolation() {
        let tree = parse("x = \"a ${y + 1} b\";");
        let expr = expr_statement(&statements(&tree)[0]);
        let rhs = match expr.expression.as_deref() {
            Some(Node::Expression(e)) => e,
            _other => panic!(),
        };
        let primary = primary_of(base_expr(rhs));
        match primary.pathable.as_deref() {
            Some(Node::ConstExpr(c)) => match c.literal.as_ref() {
                Node::Literal(lit) => match lit.double_quote_string.as_deref() {
                    Some(Node::DoubleQuoteStringLiteral(s)) => {
                        // "a " + ${y + 1} + " b"
                        assert!(s.static_characters.is_none());
                        assert_eq!(s.parts.len(), 3);
                        match &s.parts[1] {
                            DyStrPart::Expr(expression) => match expression.as_ref() {
                                Node::StringExpression(string_expression) => {
                                    assert_eq!(string_expression.start.text(), "${");
                                    assert!(string_expression.expression.is_some());
                                    assert!(string_expression.selector_variable.is_none());
                                }
                                other => {
                                    panic!("expected string expression, got {other:?}")
                                }
                            },
                            other => panic!("expected dynamic part, got {other:?}"),
                        }
                    }
                    other => panic!("expected string literal, got {other:?}"),
                },
                other => panic!("expected literal, got {other:?}"),
            },
            other => panic!("expected const, got {other:?}"),
        }

        let tree = build_tree(
            "x = \"${user}\";",
            Some(&DefaultOps),
            false,
            |_| {},
            InterpolationMode::Variable,
            "${",
            "}",
            true,
        )
        .expect("variable interpolation");
        let expression = expr_statement(&statements(&tree)[0])
            .expression
            .as_deref()
            .and_then(|node| match node {
                Node::Expression(expression) => Some(expression),
                _ => None,
            })
            .expect("assignment rhs");
        let literal = literal_of(expression);
        match literal.double_quote_string.as_deref() {
            Some(Node::DoubleQuoteStringLiteral(string)) => match &string.parts[0] {
                DyStrPart::Expr(expression) => match expression.as_ref() {
                    Node::StringExpression(string_expression) => {
                        assert_eq!(
                            string_expression
                                .selector_variable
                                .as_ref()
                                .map(TerminalNode::text),
                            Some("user")
                        );
                        assert!(string_expression.expression.is_none());
                    }
                    other => panic!("expected string expression, got {other:?}"),
                },
                other => panic!("expected dynamic part, got {other:?}"),
            },
            other => panic!("expected double-quoted literal, got {other:?}"),
        }
    }

    /// `SOURCE_PARITY`：Java `LiteralContext` 的四种 token accessor、
    /// `boolenLiteral` 和 `doubleQuoteStringLiteral` 在 Rust 中分别适配为
    /// `token`、`boolen` 与 `double_quote_string` 字段。
    #[test]
    fn literal_context_accessors_preserve_java_variants() {
        for (script, token_type, text) in [
            ("0x1F;", INTEGER_LITERAL, "0x1F"),
            (".5;", FLOATING_POINT_LITERAL, ".5"),
            ("1;", INTEGER_OR_FLOATING_LITERAL, "1"),
            ("'text';", QUOTE_STRING_LITERAL, "'text'"),
        ] {
            let tree = parse(script);
            let literal = literal_of(expr_statement(&statements(&tree)[0]));
            let token = literal.token.as_ref().expect("token literal");
            assert_eq!(token.symbol().token_type(), token_type);
            assert_eq!(token.text(), text);
            assert!(literal.boolen.is_none());
            assert!(literal.double_quote_string.is_none());
        }

        let tree = parse("true;");
        let literal = literal_of(expr_statement(&statements(&tree)[0]));
        assert!(matches!(
            literal.boolen.as_deref(),
            Some(Node::BoolenLiteral(_))
        ));
        assert!(literal.token.is_none());

        // Java 仅在 DISABLE 模式发出 StaticStringCharacters；SCRIPT /
        // VARIABLE 模式使用 DyStrText，即使字符串中没有插值。
        let tree = build_tree(
            "\"plain\";",
            Some(&DefaultOps),
            false,
            |_| {},
            InterpolationMode::Disable,
            "${",
            "}",
            true,
        )
        .expect("disabled interpolation literal");
        let literal = literal_of(expr_statement(&statements(&tree)[0]));
        match literal.double_quote_string.as_deref() {
            Some(Node::DoubleQuoteStringLiteral(string)) => {
                assert_eq!(
                    string.static_characters.as_ref().map(TerminalNode::text),
                    Some("plain")
                );
            }
            other => panic!("expected double-quoted literal, got {other:?}"),
        }
    }

    #[test]
    fn parses_context_selector() {
        let tree = build_tree(
            "$[user]",
            Some(&DefaultOps),
            false,
            |_| {},
            InterpolationMode::Script,
            "$[",
            "]",
            true,
        )
        .unwrap();
        let expr = expr_statement(&statements(&tree)[0]);
        let primary = primary_of(base_expr(expr));
        match primary.pathable.as_deref() {
            Some(Node::ContextSelectExpr(s)) => assert_eq!(s.selector_variable.text(), "user"),
            other => panic!("expected selector, got {other:?}"),
        }
    }

    #[test]
    fn parses_method_reference() {
        let tree = parse("f = String::valueOf;");
        let expr = expr_statement(&statements(&tree)[0]);
        let rhs = match expr.expression.as_deref() {
            Some(Node::Expression(e)) => e,
            _other => panic!(),
        };
        let primary = primary_of(base_expr(rhs));
        assert!(matches!(primary.path_parts[0], Node::MethodAccess(_)));
    }

    #[test]
    fn array_initializer_in_declaration() {
        let tree = parse("int[] a = {1, 2};");
        match &statements(&tree)[0] {
            Node::LocalVariableDeclarationStatement(s) => {
                match s.local_variable_declaration.as_ref() {
                    Node::LocalVariableDeclaration(decl) => {
                        match decl.variable_declarator_list.as_ref() {
                            Node::VariableDeclaratorList(list) => match &list.variables[0] {
                                Node::VariableDeclarator(v) => match v.initializer.as_deref() {
                                    Some(Node::VariableInitializer(init)) => {
                                        match init.array_initializer.as_deref() {
                                            Some(Node::ArrayInitializer(array)) => {
                                                assert_eq!(array.lbrace.text(), "{");
                                                assert_eq!(array.rbrace.text(), "}");
                                                match array.initializers.as_deref() {
                                                    Some(Node::VariableInitializerList(list)) => {
                                                        assert_eq!(list.initializers.len(), 2);
                                                        assert_eq!(list.commas.len(), 1);
                                                    }
                                                    other => panic!(
                                                        "expected initializer list, got {other:?}"
                                                    ),
                                                }
                                            }
                                            other => {
                                                panic!("expected array initializer, got {other:?}")
                                            }
                                        }
                                    }
                                    other => panic!("expected initializer, got {other:?}"),
                                },
                                other => panic!("expected declarator, got {other:?}"),
                            },
                            _other => panic!(),
                        }
                    }
                    _other => panic!(),
                }
            }
            other => panic!("expected local decl, got {other:?}"),
        }
    }

    // ------------------------------------------------------------------
    // Errors: code + line/col
    // ------------------------------------------------------------------

    #[test]
    fn dangling_operator_reports_syntax_error_with_position() {
        let err = parse_err("1 + ;");
        assert_eq!(err.error_code(), error_codes::SYNTAX_ERROR);
        assert_eq!(err.line_no(), 1);
        // ';' is at column 4 (1-based)
        assert_eq!(err.col_no(), 5);
        assert!(err.reason().contains("expecting expression"));
    }

    #[test]
    fn unclosed_paren_reports_eof_position() {
        let err = parse_err("x = (1 + 2;");
        assert_eq!(err.error_code(), error_codes::SYNTAX_ERROR);
        assert!(err.reason().contains("expecting ')'"));
    }

    #[test]
    fn missing_semicolon_after_declaration_errors() {
        let err = parse_err("int a");
        assert_eq!(err.error_code(), error_codes::SYNTAX_ERROR);
        assert_eq!(err.line_no(), 1);
    }

    #[test]
    fn error_line_tracks_newlines() {
        let err = parse_err("a = 1;\nb = ;");
        assert_eq!(err.error_code(), error_codes::SYNTAX_ERROR);
        assert_eq!(err.line_no(), 2);
    }

