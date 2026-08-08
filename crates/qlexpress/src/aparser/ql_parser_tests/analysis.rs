    // ------------------------------------------------------------------
    // Compile-time visitors
    // ------------------------------------------------------------------

    #[test]
    fn out_var_names_collects_external_reads() {
        let tree = parse("a = b + c.d;");
        let supplier = crate::class_supplier::DefaultClassSupplier::instance();
        let import_manager = ImportManager::new(&supplier, vec![]);
        let mut visitor = OutVarNamesVisitor::new(import_manager);
        tree.accept(&mut visitor);
        assert!(visitor.out_vars().contains("b"));
        assert!(visitor.out_vars().contains("c"));
        assert!(!visitor.out_vars().contains("a"));
    }

    #[test]
    fn out_var_names_compound_assign_counts_as_read() {
        let tree = parse("a += 1;");
        let supplier = crate::class_supplier::DefaultClassSupplier::instance();
        let mut visitor = OutVarNamesVisitor::new(ImportManager::new(&supplier, vec![]));
        tree.accept(&mut visitor);
        assert!(visitor.out_vars().contains("a"));
    }

    #[test]
    fn out_var_names_respects_local_declaration() {
        let tree = parse("int b = 1;\na = b;");
        let supplier = crate::class_supplier::DefaultClassSupplier::instance();
        let mut visitor = OutVarNamesVisitor::new(ImportManager::new(&supplier, vec![]));
        tree.accept(&mut visitor);
        assert!(visitor.out_vars().is_empty());
    }

    #[test]
    fn out_var_names_skips_imported_class_paths() {
        let mut supplier = crate::class_supplier::DefaultClassSupplier::instance();
        supplier.register("java.lang.Math");
        let tree = parse("import java.lang.Math;\nx = Math.max(1, 2);");
        let mut visitor = OutVarNamesVisitor::new(ImportManager::new(&supplier, vec![]));
        tree.accept(&mut visitor);
        assert!(visitor.out_vars().is_empty());
    }

    #[test]
    fn out_var_attrs_collects_attr_paths() {
        let tree = parse("x = a.b.c + a.b[0];");
        let supplier = crate::class_supplier::DefaultClassSupplier::instance();
        let mut visitor = OutVarAttrsVisitor::new(ImportManager::new(&supplier, vec![]));
        tree.accept(&mut visitor);
        assert!(visitor.out_var_attrs().contains(&vec![
            "a".to_string(),
            "b".to_string(),
            "c".to_string()
        ]));
        assert!(visitor
            .out_var_attrs()
            .contains(&vec!["a".to_string(), "b".to_string()]));
    }

    #[test]
    fn out_function_collects_undefined_calls() {
        let tree = parse("function f(a) { g(a); }\nf(1);\nh();");
        let mut visitor = OutFunctionVisitor::new();
        tree.accept(&mut visitor);
        assert!(visitor.out_functions().contains("g"));
        assert!(visitor.out_functions().contains("h"));
        assert!(!visitor.out_functions().contains("f"));
    }

    #[test]
    fn check_visitor_blocks_disallowed_operator() {
        let tree = parse("a = 1 + 2;");
        let options = crate::check_options::CheckOptions::builder()
            .operator_check_strategy(OperatorCheckStrategy::blacklist(
                ["+".to_string()].into_iter().collect(),
            ))
            .build();
        let mut checker = CheckVisitor::new(&options, "a = 1 + 2;");
        let err = checker.check(&tree).unwrap_err();
        assert_eq!(err.error_code(), error_codes::OPERATOR_NOT_ALLOWED);
        assert_eq!(err.line_no(), 1);
    }

    #[test]
    fn check_visitor_can_disable_function_calls() {
        let options = crate::check_options::CheckOptions::builder()
            .disable_function_calls(true)
            .build();
        for script in ["a = f(1);", "a = value?.f();", "a = values*.f();"] {
            let tree = parse(script);
            let mut checker = CheckVisitor::new(&options, script);
            let err = checker.check(&tree).unwrap_err();
            assert_eq!(
                err.error_code(),
                "FUNCTION_CALL_NOT_ALLOWED",
                "script: {script}"
            );
        }
    }

    #[test]
    fn check_visitor_passes_clean_script() {
        let tree = parse("a = 1 + 2;");
        let options = crate::check_options::CheckOptions::builder().build();
        let mut checker = CheckVisitor::new(&options, "a = 1 + 2;");
        assert!(checker.check(&tree).is_ok());
    }
