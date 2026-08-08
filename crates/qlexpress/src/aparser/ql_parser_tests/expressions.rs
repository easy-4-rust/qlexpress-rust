    // ------------------------------------------------------------------
    // Expressions: precedence and forms
    // ------------------------------------------------------------------

    #[test]
    fn multiplication_binds_tighter_than_plus() {
        let tree = parse("1 + 2 * 3;");
        let base = base_expr(expr_statement(&statements(&tree)[0]));
        // base: primary=1, leftAssos=[ + (2*3) ]
        assert_eq!(base.left_assos.len(), 1);
        assert_eq!(binaryop_text(&base.left_assos[0]), "+");
        match &base.left_assos[0] {
            Node::LeftAsso(l) => match l.right.as_ref() {
                Node::BaseExpr(right) => {
                    assert_eq!(right.left_assos.len(), 1);
                    assert_eq!(binaryop_text(&right.left_assos[0]), "*");
                }
                other => panic!("expected right base expr, got {other:?}"),
            },
            other => panic!("expected left asso, got {other:?}"),
        }
    }

    #[test]
    fn comparison_binds_looser_than_add() {
        // a + 1 < b * 2  ->  base(a) assos:[ + 1, < (b*2) ]
        let tree = parse("a + 1 < b * 2;");
        let base = base_expr(expr_statement(&statements(&tree)[0]));
        assert_eq!(base.left_assos.len(), 2);
        assert_eq!(binaryop_text(&base.left_assos[0]), "+");
        assert_eq!(binaryop_text(&base.left_assos[1]), "<");
        match &base.left_assos[1] {
            Node::LeftAsso(l) => match l.right.as_ref() {
                Node::BaseExpr(right) => assert_eq!(binaryop_text(&right.left_assos[0]), "*"),
                other => panic!("expected right base expr, got {other:?}"),
            },
            _other => panic!(),
        }
    }

    #[test]
    fn ternary_parses_right_associative_tail() {
        let tree = parse("a ? b : c ? d : e;");
        let expr = expr_statement(&statements(&tree)[0]);
        match expr.ternary.as_deref() {
            Some(Node::TernaryExpr(t)) => {
                assert!(t.question.is_some());
                assert!(t.then_expr.is_some());
                // else branch is a full expression containing another ternary
                match t.else_expr.as_deref() {
                    Some(Node::Expression(inner)) => {
                        assert!(matches!(
                            inner.ternary.as_deref(),
                            Some(Node::TernaryExpr(it)) if it.question.is_some()
                        ));
                    }
                    other => panic!("expected else expr, got {other:?}"),
                }
            }
            other => panic!("expected ternary, got {other:?}"),
        }
    }

    #[test]
    fn assignment_is_right_associative() {
        let tree = parse("a = b = 1;");
        let expr = expr_statement(&statements(&tree)[0]);
        assert!(expr.is_assign());
        match expr.expression.as_deref() {
            Some(Node::Expression(rhs)) => assert!(rhs.is_assign()),
            other => panic!("expected nested assign, got {other:?}"),
        }
    }

    #[test]
    fn unary_prefix_and_suffix() {
        let tree = parse("x = -a++ + !b;");
        let expr = expr_statement(&statements(&tree)[0]);
        let rhs = match expr.expression.as_deref() {
            Some(Node::Expression(e)) => e,
            other => panic!("expected rhs, got {other:?}"),
        };
        let base = base_expr(rhs);
        let primary = primary_of(base);
        assert!(matches!(
            primary.prefix.as_deref(),
            Some(Node::PrefixExpress(_))
        ));
        assert!(matches!(
            primary.suffix.as_deref(),
            Some(Node::SuffixExpress(_))
        ));
        assert_eq!(base.left_assos.len(), 1);
        assert_eq!(binaryop_text(&base.left_assos[0]), "+");
    }

    #[test]
    fn parses_cast_group_and_type_expr() {
        let tree = parse("x = (int) 3.5;");
        let expr = expr_statement(&statements(&tree)[0]);
        let rhs = match expr.expression.as_deref() {
            Some(Node::Expression(e)) => e,
            _other => panic!(),
        };
        let primary = primary_of(base_expr(rhs));
        assert!(matches!(
            primary.pathable.as_deref(),
            Some(Node::CastExpr(_))
        ));

        let tree = parse("x = (1 + 2) * 3;");
        let expr = expr_statement(&statements(&tree)[0]);
        let rhs = match expr.expression.as_deref() {
            Some(Node::Expression(e)) => e,
            _other => panic!(),
        };
        let base = base_expr(rhs);
        let primary = primary_of(base);
        assert!(matches!(
            primary.pathable.as_deref(),
            Some(Node::GroupExpr(_))
        ));
        assert_eq!(binaryop_text(&base.left_assos[0]), "*");
    }

    #[test]
    fn parses_new_expressions() {
        let tree = parse("m = new java.util.HashMap();");
        let expr = expr_statement(&statements(&tree)[0]);
        let rhs = match expr.expression.as_deref() {
            Some(Node::Expression(e)) => e,
            _other => panic!(),
        };
        let primary = primary_of(base_expr(rhs));
        match primary.pathable.as_deref() {
            Some(Node::NewObjExpr(new_obj)) => {
                let names: Vec<String> = new_obj.var_ids.iter().map(|id| id.text()).collect();
                assert_eq!(names, ["java", "util", "HashMap"]);
            }
            other => panic!("expected new obj, got {other:?}"),
        }

        let tree = parse("a = new int[10];");
        let expr = expr_statement(&statements(&tree)[0]);
        let rhs = match expr.expression.as_deref() {
            Some(Node::Expression(e)) => e,
            _other => panic!(),
        };
        assert!(matches!(
            primary_of(base_expr(rhs)).pathable.as_deref(),
            Some(Node::NewEmptyArrExpr(_))
        ));

        let tree = parse("a = new int[] {1, 2, 3};");
        let expr = expr_statement(&statements(&tree)[0]);
        let rhs = match expr.expression.as_deref() {
            Some(Node::Expression(e)) => e,
            _other => panic!(),
        };
        assert!(matches!(
            primary_of(base_expr(rhs)).pathable.as_deref(),
            Some(Node::NewInitArrExpr(_))
        ));
    }

    #[test]
    fn primitive_new_object_is_error() {
        let err = parse_err("x = new int();");
        assert_eq!(err.error_code(), error_codes::SYNTAX_ERROR);
    }

    #[test]
    fn parses_list_map_and_block_expr() {
        let tree = parse("x = [1, 2, 3];");
        let expr = expr_statement(&statements(&tree)[0]);
        let rhs = match expr.expression.as_deref() {
            Some(Node::Expression(e)) => e,
            _other => panic!(),
        };
        match primary_of(base_expr(rhs)).pathable.as_deref() {
            Some(Node::ListExpr(list)) => match list.list_items.as_deref() {
                Some(Node::ListItems(items)) => assert_eq!(items.expressions.len(), 3),
                other => panic!("expected list items, got {other:?}"),
            },
            other => panic!("expected list, got {other:?}"),
        }

        let tree = parse("x = {'a': 1, b: 2};");
        let expr = expr_statement(&statements(&tree)[0]);
        let rhs = match expr.expression.as_deref() {
            Some(Node::Expression(e)) => e,
            _other => panic!(),
        };
        match primary_of(base_expr(rhs)).pathable.as_deref() {
            Some(Node::MapExpr(map)) => match map.map_entries.as_ref() {
                Node::MapEntries(entries) => assert_eq!(entries.entries.len(), 2),
                other => panic!("expected entries, got {other:?}"),
            },
            other => panic!("expected map, got {other:?}"),
        }

        // Double-quoted StringKey and the special '@class' ClsValue accessor.
        let tree = parse(r#"x = {"name": 1, '@class': 'java.lang.String'};"#);
        let expr = expr_statement(&statements(&tree)[0]);
        let rhs = match expr.expression.as_deref() {
            Some(Node::Expression(e)) => e,
            _other => panic!(),
        };
        match primary_of(base_expr(rhs)).pathable.as_deref() {
            Some(Node::MapExpr(map)) => match map.map_entries.as_ref() {
                Node::MapEntries(entries) => {
                    assert_eq!(entries.entries.len(), 2);
                    match &entries.entries[0] {
                        Node::MapEntry(entry) => assert!(matches!(
                            entry.map_key.as_ref(),
                            Node::StringKey(StringKeyContext {
                                double_quote_string,
                            }) if matches!(double_quote_string.as_ref(), Node::DoubleQuoteStringLiteral(_))
                        )),
                        other => panic!("expected map entry, got {other:?}"),
                    }
                    match &entries.entries[1] {
                        Node::MapEntry(entry) => assert!(matches!(
                            entry.map_value.as_ref(),
                            Node::ClsValue(ClsValueContext { quote })
                                if quote.text() == "'java.lang.String'"
                        )),
                        other => panic!("expected class map entry, got {other:?}"),
                    }
                }
                other => panic!("expected entries, got {other:?}"),
            },
            other => panic!("expected map, got {other:?}"),
        }

        // empty map literal
        let tree = parse("x = {:};");
        let expr = expr_statement(&statements(&tree)[0]);
        let rhs = match expr.expression.as_deref() {
            Some(Node::Expression(e)) => e,
            _other => panic!(),
        };
        match primary_of(base_expr(rhs)).pathable.as_deref() {
            Some(Node::MapExpr(map)) => match map.map_entries.as_ref() {
                Node::MapEntries(entries) => assert!(entries.empty_colon.is_some()),
                other => panic!("expected entries, got {other:?}"),
            },
            other => panic!("expected map, got {other:?}"),
        }
    }

    #[test]
    fn parses_index_and_slice() {
        let tree = parse("x = a[1];");
        let expr = expr_statement(&statements(&tree)[0]);
        let rhs = match expr.expression.as_deref() {
            Some(Node::Expression(e)) => e,
            _other => panic!(),
        };
        let primary = primary_of(base_expr(rhs));
        match &primary.path_parts[0] {
            Node::IndexExpr(i) => {
                assert!(matches!(
                    i.index_value_expr.as_deref(),
                    Some(Node::SingleIndex(_))
                ));
            }
            other => panic!("expected index, got {other:?}"),
        }

        let tree = parse("x = a[1:3];");
        let expr = expr_statement(&statements(&tree)[0]);
        let rhs = match expr.expression.as_deref() {
            Some(Node::Expression(e)) => e,
            _other => panic!(),
        };
        let primary = primary_of(base_expr(rhs));
        match &primary.path_parts[0] {
            Node::IndexExpr(i) => match i.index_value_expr.as_deref() {
                Some(Node::SliceIndex(s)) => {
                    assert!(s.start.is_some() && s.end.is_some());
                }
                other => panic!("expected slice, got {other:?}"),
            },
            other => panic!("expected index, got {other:?}"),
        }
    }

    #[test]
    fn parses_method_field_and_chaining_paths() {
        let tree = parse("x = a.b().c?.d*.e;");
        let expr = expr_statement(&statements(&tree)[0]);
        let rhs = match expr.expression.as_deref() {
            Some(Node::Expression(e)) => e,
            _other => panic!(),
        };
        let primary = primary_of(base_expr(rhs));
        assert_eq!(primary.path_parts.len(), 4);
        match &primary.path_parts[0] {
            Node::MethodInvoke(m) => assert_eq!(m.chain, ChainKind::Plain),
            other => panic!("expected method invoke, got {other:?}"),
        }
        match &primary.path_parts[1] {
            Node::FieldAccess(f) => assert_eq!(f.chain, ChainKind::Plain),
            other => panic!("expected field access, got {other:?}"),
        }
        match &primary.path_parts[2] {
            Node::FieldAccess(f) => assert_eq!(f.chain, ChainKind::Optional),
            other => panic!("expected optional field, got {other:?}"),
        }
        match &primary.path_parts[3] {
            Node::FieldAccess(f) => assert_eq!(f.chain, ChainKind::Spread),
            other => panic!("expected spread field, got {other:?}"),
        }

        let tree = parse(r"x = a.'display\'name';");
        let expr = expr_statement(&statements(&tree)[0]);
        let rhs = match expr.expression.as_deref() {
            Some(Node::Expression(expression)) => expression,
            other => panic!("expected expression, got {other:?}"),
        };
        let primary = primary_of(base_expr(rhs));
        let field_id = match &primary.path_parts[0] {
            Node::FieldAccess(field) => match field.field_id.as_ref() {
                Node::FieldId(field_id) => field_id,
                other => panic!("expected field id, got {other:?}"),
            },
            other => panic!("expected quoted field access, got {other:?}"),
        };
        assert!(field_id.token.is_none());
        assert_eq!(
            field_id.quote_string_literal().map(TerminalNode::text),
            Some(r"'display\'name'")
        );
    }

    /// `SOURCE_PARITY`：Java `SyntaxTreeFactory#buildTree` 在 debug 模式下
    /// 依次输出 Token 流与 `RuleContext#toStringTree()`，而不是扁平文本。
    #[test]
    fn build_tree_prints_token_stream_and_java_tree_shape() {
        let mut printed = Vec::new();
        let tree = build_tree(
            "a + b;",
            Some(&DefaultOps),
            true,
            |line| printed.push(line),
            InterpolationMode::Script,
            "${",
            "}",
            true,
        )
        .expect("debug parse");

        assert_eq!(printed.len(), 2);
        assert_eq!(printed[0], "a | + | b | ; | <EOF>");
        assert_eq!(printed[1], tree.to_string_tree());
        assert!(printed[1].starts_with("(Program "));
        assert_ne!(printed[1], tree.text());
    }

