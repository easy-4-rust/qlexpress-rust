/// Java `Express4RunnerTest#parseToCacheTest`。
#[test]
fn java_parse_to_cache_test() {
    let runner = Express4Runner::new();
    let first = runner
        .parse_to_definition_with_cache("a+b")
        .expect("first cached parse");
    let second = runner
        .parse_to_definition_with_cache("a+b")
        .expect("second cached parse");
    assert!(Rc::ptr_eq(&first, &second));
}

/// SOURCE_PARITY: Java `compileCache` 在显式 `clearCompileCache()` 前不会
/// 因容量自动淘汰；覆盖旧实现 1024 条目上限的回归。
#[test]
fn java_compatible_compile_cache_is_not_lru_bounded() {
    let runner = Express4Runner::new();
    let first = runner
        .parse_to_definition_with_cache("0")
        .expect("populate first cache entry");
    for value in 1..=1_024 {
        runner
            .parse_to_definition_with_cache(&value.to_string())
            .expect("populate Java-compatible cache");
    }
    let first_again = runner
        .parse_to_definition_with_cache("0")
        .expect("first entry must remain cached");

    assert!(Rc::ptr_eq(&first, &first_again));
    assert_eq!(runner.compile_cache_stats().evictions, 0);
}

/// SOURCE_PARITY: Java `Express4Runner#clearCompileCache()` 清空按脚本文本
/// 保存的编译产物；同一脚本随后必须重新编译，而不是继续返回旧对象。
#[test]
fn java_clear_compile_cache_forces_recompile() {
    let runner = Express4Runner::new();
    let before_clear = runner
        .parse_to_definition_with_cache("40 + 2")
        .expect("populate compatible compile cache");
    assert_eq!(runner.compile_cache_stats().entries, 1);

    runner.clear_compile_cache();
    assert_eq!(runner.compile_cache_stats().entries, 0);

    let after_clear = runner
        .parse_to_definition_with_cache("40 + 2")
        .expect("recompile after explicit cache clear");
    assert!(!Rc::ptr_eq(&before_clear, &after_clear));
    assert_eq!(runner.compile_cache_stats().entries, 1);
}

/// SOURCE_PARITY: Java `Express4Runner#parseToLambda(String, ExpressContext,
/// QLOptions)`，并覆盖 `QLOptions.cache` 的两条分支。
#[test]
fn java_parse_to_lambda_script_overload() {
    let runner = Express4Runner::new();
    let uncached = runner
        .parse_to_lambda(
            "1 + 2",
            Rc::new(EmptyContext),
            &QLOptions::builder().cache(false).build(),
        )
        .expect("compile uncached lambda");
    assert_integer(
        &uncached
            .q_lambda()
            .call(&[])
            .expect("invoke uncached lambda")
            .value(),
        3,
    );

    let cached = runner
        .parse_to_lambda(
            "1 + 2",
            Rc::new(EmptyContext),
            &QLOptions::builder().cache(true).build(),
        )
        .expect("compile cached lambda");
    assert_integer(
        &cached
            .q_lambda()
            .call(&[])
            .expect("invoke cached lambda")
            .value(),
        3,
    );
    assert_eq!(runner.compile_cache_stats().entries, 1);
}

/// SOURCE_PARITY: Java `parseToLambda` 的全局作用域保存 runner
/// `userDefineFunction` Map 的同一引用，创建 Lambda 后新增的函数仍然可见。
#[test]
fn existing_lambda_observes_later_runner_function_registration() {
    let runner = Express4Runner::new();
    let lambda = runner
        .parse_to_lambda("lateBound()", Rc::new(EmptyContext), &QLOptions::default())
        .expect("unknown runtime function names compile to dynamic lookup");

    assert!(runner.add_function(
        "lateBound",
        |_context: &mut dyn QContext, _parameters: &Parameters| Ok(DataValue::Int(42)),
    ));

    assert_eq!(
        lambda
            .q_lambda()
            .call(&[])
            .expect("existing lambda must see later function")
            .value(),
        DataValue::Int(42)
    );
}

/// SOURCE_PARITY: Java `QLOptions.Builder#attachments(Map)`、`QvmRuntime`
/// 与 `QvmGlobalScope` 保存同一个 Map 引用；Lambda 创建后替换附件条目时，
/// 自定义函数和外部变量上下文都必须读取到最新值。
#[test]
fn existing_lambda_observes_later_attachment_map_mutation() {
    let runner = Express4Runner::new();
    assert!(runner.add_function(
        "attachmentValue",
        |context: &mut dyn QContext, _parameters: &Parameters| {
            Ok(context
                .attachment()
                .get("direct")
                .cloned()
                .unwrap_or(DataValue::Null))
        },
    ));
    let shared_attachments: SharedAttachments = Rc::new(std::cell::RefCell::new(HashMap::new()));
    let options = QLOptions::builder()
        .shared_attachments(Rc::clone(&shared_attachments))
        .build();
    let lambda = runner
        .parse_to_lambda(
            "attachmentValue() + ${/box/value}",
            Rc::new(AttachmentPathContext),
            &options,
        )
        .expect("compile lambda before mutating attachments");

    shared_attachments
        .borrow_mut()
        .insert("direct".to_string(), DataValue::Int(40));
    shared_attachments.borrow_mut().insert(
        "box".to_string(),
        DataValue::Map(Rc::new(std::cell::RefCell::new(IndexMap::from_entries(
            vec![(DataValue::Str("value".into()), DataValue::Int(2))],
        )))),
    );

    assert_integer(
        &lambda
            .q_lambda()
            .call(&[])
            .expect("existing lambda must observe attachment mutation")
            .value(),
        42,
    );
}

/// SOURCE_PARITY: Java `QScope#getFunctionTable()` 返回当前作用域的实际可变
/// Map；宿主函数通过 `QContext` 动态登记函数后，同一次脚本执行的后续调用
/// 必须立即可见。
#[test]
fn custom_function_can_mutate_live_scope_function_table() {
    let runner = Express4Runner::new();
    assert!(runner.add_function(
        "install",
        |context: &mut dyn QContext, _parameters: &Parameters| {
            let late_function: Rc<dyn CustomFunction> =
                Rc::new(|_context: &mut dyn QContext, _parameters: &Parameters| {
                    Ok(DataValue::Int(42))
                });
            let function_table = context.function_table();
            function_table
                .borrow_mut()
                .insert("late".to_string(), late_function);
            Ok(DataValue::Null)
        },
    ));

    let result = runner
        .execute("install(); late()", HashMap::new(), &QLOptions::default())
        .expect("newly installed function must be visible in the same execution");
    assert_integer(result.result(), 42);
}

/// SOURCE_PARITY: Java `parseToLambda(LoadedParseCache, ...)` 与
/// `parseToLambda(SerializableParseCache, ...)`，包括 runner 身份绑定校验。
#[test]
fn java_parse_to_lambda_cache_overloads_and_runner_binding() {
    let owner = Express4Runner::new();
    let serialized = owner.export_parse_cache("40 + 2").expect("export cache");
    let loaded = owner.import_parse_cache(&serialized).expect("load cache");

    let loaded_lambda = owner
        .parse_loaded_cache_to_lambda(&loaded, Rc::new(EmptyContext), &QLOptions::default())
        .expect("materialize loaded cache");
    assert_integer(
        &loaded_lambda
            .q_lambda()
            .call(&[])
            .expect("invoke loaded lambda")
            .value(),
        42,
    );

    let serialized_lambda = owner
        .parse_serializable_cache_to_lambda(
            &serialized,
            Rc::new(EmptyContext),
            &QLOptions::default(),
        )
        .expect("materialize serializable cache");
    assert_integer(
        &serialized_lambda
            .q_lambda()
            .call(&[])
            .expect("invoke serialized lambda")
            .value(),
        42,
    );

    let other_runner = Express4Runner::new();
    let error = match other_runner.parse_loaded_cache_to_lambda(
        &loaded,
        Rc::new(EmptyContext),
        &QLOptions::default(),
    ) {
        Ok(_) => panic!("cache must stay bound to its importing runner"),
        Err(error) => error,
    };
    assert_eq!(
        error.error_code(),
        error_codes::SERIALIZABLE_PARSE_CACHE_INVALID_MODEL
    );
}

/// Java `Express4RunnerTest#addFunctionsDefinedInScriptTest`。
#[test]
fn java_add_functions_defined_in_script_test() {
    let runner = Express4Runner::with_init_options(
        InitOptions::builder()
            .security_strategy(QLSecurityStrategy::open())
            .build(),
    );
    let result = runner
        .add_functions_defined_in_script(
            concat!(
                "function myAdd(a,b) {\n    return a+b;}\n",
                "function getCurrentTime() {\n    return System.currentTimeMillis();\n}\n",
                "defineTime=System.currentTimeMillis();\n",
                "function defineTime() {\n    return defineTime;}\n"
            ),
            Rc::new(qlexpress::runtime::context::EmptyContext),
            &QLOptions::default(),
        )
        .expect("register script functions");
    assert_eq!(result.get_succ().len(), 3);
    assert!(result.get_fail().is_empty());

    let sum = runner
        .execute("myAdd(1,2)", HashMap::new(), &QLOptions::default())
        .expect("myAdd");
    assert_integer(sum.result(), 3);

    let current_time_1 = runner
        .execute("getCurrentTime()", HashMap::new(), &QLOptions::default())
        .expect("first current time")
        .into_result();
    thread::sleep(Duration::from_millis(3));
    let current_time_2 = runner
        .execute("getCurrentTime()", HashMap::new(), &QLOptions::default())
        .expect("second current time")
        .into_result();
    assert_ne!(current_time_1, current_time_2);

    let defined_time_1 = runner
        .execute("defineTime()", HashMap::new(), &QLOptions::default())
        .expect("first captured time")
        .into_result();
    thread::sleep(Duration::from_millis(3));
    let defined_time_2 = runner
        .execute("defineTime()", HashMap::new(), &QLOptions::default())
        .expect("second captured time")
        .into_result();
    assert_eq!(defined_time_1, defined_time_2);
}

/// Java `Express4RunnerTest#checkSyntaxTest`。
#[test]
fn java_check_syntax_test() {
    let runner = Express4Runner::new();
    let first = runner
        .check_default("a+b;\n(a+b")
        .expect_err("missing right parenthesis");
    assert_eq!(first.line_no(), 2);
    assert_eq!(first.col_no(), 5);
    assert_eq!(first.error_code(), "SYNTAX_ERROR");
    assert_eq!(
        first.to_string(),
        concat!(
            "[Error SYNTAX_ERROR: mismatched input '<EOF>' expecting ')']\n",
            "[Near: a+b; (a+b<EOF>]\n",
            "                ^^^^^\n",
            "[Line: 2, Column: 5]"
        )
    );

    let second = runner
        .check_default("sellerId in [1001] || (sellerId not in [1001])")
        .expect_err("invalid infix not");
    assert_eq!(
        second.to_string(),
        concat!(
            "[Error SYNTAX_ERROR: mismatched input 'not' expecting ')']\n",
            "[Near: ...[1001] || (sellerId not in [1001])]\n",
            "                              ^^^\n",
            "[Line: 1, Column: 33]"
        )
    );
}

/// Java `Express4RunnerTest#cacheDocTest`。
#[test]
fn java_cache_doc_test() {
    let result = Express4Runner::new()
        .execute(
            "1+2",
            HashMap::new(),
            &QLOptions::builder().cache(true).build(),
        )
        .expect("cached execution");
    assert_integer(result.result(), 3);
}

fn import_tester_runner(default_import: bool) -> Express4Runner {
    let class_name = "com.alibaba.qlexpress4.QLImportTester";
    let mut supplier = DefaultClassSupplier::instance();
    supplier.register(class_name);
    let mut builder = InitOptions::builder()
        .class_supplier(Rc::new(supplier))
        .security_strategy(QLSecurityStrategy::open());
    if default_import {
        builder = builder.add_default_import(vec![QLImport::import_cls(class_name)]);
    }
    let mut runner = Express4Runner::with_init_options(builder.build());
    let mut native_type = NativeType::named(class_name);
    native_type.static_methods.insert(
        "add".to_string(),
        Rc::new(|_bean, arguments| match arguments {
            [DataValue::Int(left), DataValue::Int(right)] => Ok(DataValue::Int(left + right)),
            _ => unreachable!("QLImportTester.add receives two int arguments"),
        }),
    );
    runner.register_native_type(native_type);
    runner
}

/// Java `Express4RunnerTest#docImportJavaTest`。
#[test]
fn java_doc_import_java_test() {
    let runner = import_tester_runner(false);
    let result = runner
        .execute(
            concat!(
                "import com.alibaba.qlexpress4.QLImportTester;",
                "QLImportTester.add(a,b)"
            ),
            HashMap::from([
                ("a".to_string(), DataValue::Int(1)),
                ("b".to_string(), DataValue::Int(2)),
            ]),
            &QLOptions::default(),
        )
        .expect("explicit class import");
    assert_integer(result.result(), 3);
}

/// Java `Express4RunnerTest#docDefaultImportJavaTest`。
#[test]
fn java_doc_default_import_java_test() {
    let runner = import_tester_runner(true);
    let result = runner
        .execute(
            "QLImportTester.add(1,2)",
            HashMap::new(),
            &QLOptions::default(),
        )
        .expect("default class import");
    assert_integer(result.result(), 3);
}

/// Java `Express4RunnerTest#mapSetGetTest`。
#[test]
fn java_map_set_get_test() {
    let runner = Express4Runner::with_init_options(
        InitOptions::builder()
            .security_strategy(QLSecurityStrategy::open())
            .build(),
    );
    let result = runner
        .execute(
            "a = new HashMap<>();a['aaa'] = 'bbb';a",
            HashMap::new(),
            &QLOptions::default(),
        )
        .expect("HashMap construct and mutate")
        .into_result();
    let DataValue::Map(map) = result else {
        panic!("new HashMap must produce a map");
    };
    assert_eq!(
        map.borrow().get(&DataValue::Str("aaa".into())),
        Some(&DataValue::Str("bbb".into()))
    );
}

/// Java `Express4RunnerTest#classFieldTest`。
#[test]
fn java_class_field_test() {
    let runner = Express4Runner::with_init_options(
        InitOptions::builder()
            .security_strategy(QLSecurityStrategy::open())
            .build(),
    );
    for script in ["List.class", "java.util.List.class"] {
        let result = runner
            .execute(script, HashMap::new(), &QLOptions::default())
            .unwrap_or_else(|error| panic!("{script} failed: {error}"))
            .into_result();
        assert_eq!(
            as_meta_class(&result).expect("class literal").java_name(),
            "java.util.List"
        );
    }
}

/// Java `Express4RunnerTest#numberAmbiguousValueTest`。
#[test]
fn java_number_ambiguous_value_test() {
    let runner = Express4Runner::with_init_options(
        InitOptions::builder()
            .security_strategy(QLSecurityStrategy::open())
            .build(),
    );
    assert_eq!(
        runner
            .execute("1.doubleValue()", HashMap::new(), &QLOptions::default())
            .expect("number method invocation")
            .result(),
        &DataValue::Double(1.0)
    );
}

/// Java `Express4RunnerTest#addFunctionOfServiceMethodBasicTest`。
#[test]
fn java_add_function_of_service_method_basic_test() {
    let runner = Express4Runner::new();
    let service = RecordObject::value("MyFunctionUtil", &[]);
    let method = NativeIMethod::from_native(
        "add",
        ClassRef::Named("MyFunctionUtil".to_string()),
        vec![ClassRef::from_name("int"), ClassRef::from_name("int")],
        Rc::new(|object, arguments| match (object, arguments) {
            (DataValue::Object(service), [DataValue::Int(left), DataValue::Int(right)]) => {
                assert_eq!(service.borrow().native_type_name(), "MyFunctionUtil");
                Ok(DataValue::Int(left + right))
            }
            _ => unreachable!("svcAdd receives its service instance and two ints"),
        }),
    );
    assert!(runner.add_function_of_class_method("svcAdd", Some(service), method));
    let result = runner
        .execute("svcAdd(1,2)", HashMap::new(), &QLOptions::default())
        .expect("service method function");
    assert_integer(result.result(), 3);
}

/// Java `Express4RunnerTest#addFunctionOfServiceMethodOverloadTest`。
#[test]
fn java_add_function_of_service_method_overload_test() {
    let runner = Express4Runner::new();
    let string_method = NativeIMethod::from_native(
        "format",
        ClassRef::Named("OverloadService".to_string()),
        vec![ClassRef::Named("java.lang.String".to_string())],
        Rc::new(|_object, arguments| match arguments {
            [DataValue::Str(value)] => Ok(DataValue::string(format!("S:{value}"))),
            _ => unreachable!("fmtStr receives one string"),
        }),
    );
    assert!(runner.add_function_of_class_method("fmtStr", None, string_method));
    assert_eq!(
        runner
            .execute("fmtStr('x')", HashMap::new(), &QLOptions::default())
            .expect("string overload")
            .result(),
        &DataValue::Str("S:x".into())
    );

    let int_method = NativeIMethod::from_native(
        "format",
        ClassRef::Named("OverloadService".to_string()),
        vec![
            ClassRef::Named("java.lang.Integer".to_string()),
            ClassRef::from_name("int"),
        ],
        Rc::new(|_object, arguments| match arguments {
            [DataValue::Null, DataValue::Int(right)] => {
                Ok(DataValue::string(format!("I:null,{right}")))
            }
            _ => unreachable!("fmtInt receives nullable Integer and int"),
        }),
    );
    assert!(runner.add_function_of_class_method("fmtInt", None, int_method));
    assert_eq!(
        runner
            .execute("fmtInt(null,2)", HashMap::new(), &QLOptions::default())
            .expect("integer overload")
            .result(),
        &DataValue::Str("I:null,2".into())
    );
}
