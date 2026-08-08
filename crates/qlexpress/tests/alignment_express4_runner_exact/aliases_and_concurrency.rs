/// Java `Express4RunnerTest#executeWithObjContextTest`。
#[test]
fn java_execute_with_obj_context_test() {
    let object = RecordObject::value(
        "test.MyObj",
        &[
            ("a", DataValue::Int(1)),
            ("b", DataValue::Str("test".into())),
        ],
    );
    let result = Express4Runner::new()
        .execute_with_object("a+b", object, &QLOptions::default())
        .expect("object fields must be exposed");
    assert_eq!(result.result(), &DataValue::Str("1test".into()));
}

/// Java `Express4RunnerTest#qlAliasTest`。Rust 以显式别名元数据替代
/// Java 运行时注解扫描，其余八组原脚本与断言保持一致。
#[test]
fn java_ql_alias_test() {
    let mut runner = Express4Runner::with_init_options(
        InitOptions::builder()
            .security_strategy(QLSecurityStrategy::open())
            .build(),
    );
    runner.register_native_type(record_native_type(
        "test.Patient",
        &[
            ("birth", &["出生年月", "生日"], false),
            ("name", &["姓名", "患者姓名"], false),
            ("sex", &["性别"], false),
            ("level", &["级别"], true),
        ],
        &[
            ("getBirth", &["出生年月", "生日"]),
            ("getAge", &["获取年龄", "获取患者年龄"]),
        ],
    ));
    let patient = RecordObject::value(
        "test.Patient",
        &[
            ("birth", DataValue::Str("1987-02-23".into())),
            ("name", DataValue::Str("老王".into())),
            ("sex", DataValue::Str("男".into())),
            ("level", DataValue::Str("高危".into())),
        ],
    );
    let cases = [
        ("患者.birth", "1987-02-23"),
        ("患者.生日()", "1987-02-23"),
        ("患者.患者姓名", "老王"),
        ("患者.姓名", "老王"),
        ("患者.getBirth()==患者.出生年月()", "true"),
        ("患者.生日()==患者.生日", "true"),
        (
            "患者.患者姓名 + ' 今年 '+ 患者.获取年龄() +' 岁'",
            "老王 今年 34 岁",
        ),
        ("患者.级别='低风险';return 患者.级别;", "低风险"),
    ];
    for (script, expected) in cases {
        let result = runner
            .execute_with_alias_objects(
                script,
                &QLOptions::default(),
                &[(&["患者"], patient.clone())],
            )
            .unwrap_or_else(|error| panic!("{script}: {error}"));
        assert_eq!(result.result().string_value_of(), expected, "{script}");
    }
}

/// Java `Express4RunnerTest#qlAliasDocTest`。对象别名与字段别名显式注册，
/// 复用 Java 文档中的原始表达式。
#[test]
fn java_ql_alias_doc_test() {
    let mut runner = Express4Runner::with_init_options(
        InitOptions::builder()
            .security_strategy(QLSecurityStrategy::open())
            .build(),
    );
    runner.register_native_type(record_native_type(
        "test.Order",
        &[("orderNum", &["订单号"], true), ("amount", &["金额"], true)],
        &[],
    ));
    runner.register_native_type(record_native_type(
        "test.User",
        &[("vip", &["是vip"], true), ("name", &["用户名"], true)],
        &[],
    ));
    let order = RecordObject::value(
        "test.Order",
        &[
            ("orderNum", DataValue::Str("OR123455".into())),
            ("amount", DataValue::Int(100)),
        ],
    );
    let user = RecordObject::value(
        "test.User",
        &[
            ("name", DataValue::Str("jack".into())),
            ("vip", DataValue::Bool(true)),
        ],
    );
    let result = runner
        .execute_with_alias_objects(
            "用户.是vip? 订单.金额 * 0.8 : 订单.金额",
            &QLOptions::default(),
            &[(&["订单"], order), (&["用户"], user)],
        )
        .expect("alias document expression");
    assert_eq!(result.result().string_value_of(), "80.0");
}

/// Java `Express4RunnerTest#importClsAliasObfuscationTest`。
#[test]
fn java_import_cls_alias_obfuscation_test() {
    let mut supplier = DefaultClassSupplier::instance();
    supplier.register("test.Aa");
    supplier.register("test.Bb");
    let mut runner = Express4Runner::with_init_options(
        InitOptions::builder()
            .class_supplier(Rc::new(supplier))
            .security_strategy(QLSecurityStrategy::open())
            .add_default_import(vec![
                QLImport::import_cls_alias("test.Aa", "User"),
                QLImport::import_cls_alias("test.Bb", "Order"),
            ])
            .build(),
    );
    let mut user_type = record_native_type("test.Aa", &[("name", &[], true)], &[]);
    user_type.constructor = Some(Rc::new(|_| {
        Ok(RecordObject::value("test.Aa", &[("name", DataValue::Null)]))
    }));
    runner.register_native_type(user_type);
    let mut order_type = record_native_type("test.Bb", &[("amount", &[], true)], &[]);
    order_type.constructor = Some(Rc::new(|_| {
        Ok(RecordObject::value(
            "test.Bb",
            &[("amount", DataValue::Int(0))],
        ))
    }));
    runner.register_native_type(order_type);
    let result = runner
        .execute(
            concat!(
                "user = new User(); user.name = 'jack'; ",
                "order = new Order(); order.amount = 100; ",
                "user.name + ':' + order.amount"
            ),
            HashMap::new(),
            &QLOptions::default(),
        )
        .expect("obfuscated class aliases");
    assert_eq!(result.result(), &DataValue::Str("jack:100".into()));
}

/// Java `Express4RunnerTest#addFunctionByAnnotationTest`。
///
/// `ADAPTED`：Rust 没有 Java 运行时注解扫描，使用同一个批量注册入口显式
/// 提供五个 `@QLFunction` 名称；成功数、原脚本和四组结果断言保持一致。
#[test]
fn java_add_function_by_annotation_test() {
    let runner = Express4Runner::new();
    let add: Rc<dyn CustomFunction> = Rc::new(
        |_context: &mut dyn QContext, parameters: &Parameters| -> Result<DataValue, QLException> {
            let left = match parameters.get_value(0) {
                DataValue::Int(value) => value,
                _ => 0,
            };
            let right = match parameters.get_value(1) {
                DataValue::Int(value) => value,
                _ => 0,
            };
            Ok(DataValue::Int(left + right))
        },
    );
    let arr3: Rc<dyn CustomFunction> = Rc::new(
        |_context: &mut dyn QContext, parameters: &Parameters| -> Result<DataValue, QLException> {
            Ok(DataValue::array(vec![
                parameters.get_value(0),
                parameters.get_value(1),
                parameters.get_value(2),
            ]))
        },
    );
    let concat: Rc<dyn CustomFunction> = Rc::new(
        |_context: &mut dyn QContext, parameters: &Parameters| -> Result<DataValue, QLException> {
            Ok(DataValue::string(format!(
                "{}{}",
                parameters.get_value(0).string_value_of(),
                parameters.get_value(1).string_value_of()
            )))
        },
    );
    let add_all: Rc<dyn CustomFunction> = Rc::new(
        |_context: &mut dyn QContext, parameters: &Parameters| -> Result<DataValue, QLException> {
            let list = parameters.get_value(0);
            let DataValue::List(items) = &list else {
                return Err(QLException::for_test(
                    qlexpress::exception::ql_exception::QLExceptionKind::Runtime,
                    "list expected",
                    error_codes::INVALID_ARGUMENT,
                ));
            };
            items
                .borrow_mut()
                .extend(parameters.values().into_iter().skip(1));
            Ok(list)
        },
    );
    let add_result = runner.batch_add_function(vec![
        ("myAdd".to_string(), Rc::clone(&add)),
        ("iAdd".to_string(), add),
        ("arr3".to_string(), arr3),
        ("concat".to_string(), concat),
        ("addAll".to_string(), add_all),
    ]);
    assert_eq!(add_result.get_succ().len(), 5);
    assert!(add_result.get_fail().is_empty());

    let sum = runner
        .execute(
            "myAdd(1,2) + iAdd(5,6)",
            HashMap::new(),
            &QLOptions::default(),
        )
        .expect("annotation-name functions");
    assert_integer(sum.result(), 14);
    let array = runner
        .execute("arr3(5,9,10)[2]", HashMap::new(), &QLOptions::default())
        .expect("array function");
    assert_integer(array.result(), 10);
    let concatenated = runner
        .execute("concat('aa', null)", HashMap::new(), &QLOptions::default())
        .expect("null concatenation");
    assert_eq!(concatenated.result(), &DataValue::Str("aanull".into()));
    let list = runner
        .execute(
            "l = [1,2];\naddAll(l, 'aa', 'bb', 'cc')",
            HashMap::new(),
            &QLOptions::default(),
        )
        .expect("varargs addAll");
    assert_eq!(
        list.result(),
        &DataValue::list(vec![
            DataValue::Int(1),
            DataValue::Int(2),
            DataValue::Str("aa".into()),
            DataValue::Str("bb".into()),
            DataValue::Str("cc".into()),
        ])
    );
}

/// Java `Express4RunnerTest#methodInvokeCauseTest`。Rust cause 使用稳定错误码
/// 表达 Java `IndexOutOfBoundsException` 的具体类别。
#[test]
fn java_method_invoke_cause_test() {
    let runner = Express4Runner::with_init_options(
        InitOptions::builder()
            .security_strategy(QLSecurityStrategy::open())
            .build(),
    );
    let error = runner
        .execute(
            "l = [];l.get(3)",
            HashMap::new(),
            &QLOptions::builder().cache(false).build(),
        )
        .expect_err("out-of-bounds method call must fail");
    assert_eq!(error.error_code(), error_codes::INVOKE_METHOD_INNER_ERROR);
    let cause = error
        .cause()
        .expect("native method cause must be preserved");
    assert_eq!(cause.error_code(), error_codes::INDEX_OUT_BOUND);
    assert_eq!(cause.reason(), "Index 3 out of bounds for length 0");
}

/// Java `Express4RunnerTest#innerFunctionExceptionTest`。
#[test]
fn java_inner_function_exception_test() {
    let runner = Express4Runner::new();
    assert!(runner.add_function(
        "testExp",
        |_context: &mut dyn QContext, _parameters: &Parameters| -> Result<DataValue, QLException> {
            Err(QLException::host_error(
                qlexpress::exception::ql_exception::QLExceptionKind::Runtime,
                "inner test",
                "java.lang.RuntimeException",
            ))
        }
    ));
    assert!(runner.get_function("testExp").is_some());
    let error = runner
        .execute("1+testExp()+10", HashMap::new(), &QLOptions::default())
        .expect_err("inner function exception");
    assert_eq!(
        error
            .cause()
            .expect("host function cause must be preserved")
            .to_string(),
        "inner test"
    );
    assert_eq!(
        error.to_string(),
        concat!(
            "[Error INVOKE_FUNCTION_INNER_ERROR: exception from inner when invoking function 'testExp', error message: inner test]\n",
            "[Near: 1+testExp()+10]\n",
            "         ^^^^^^^\n",
            "[Line: 1, Column: 3]"
        )
    );
    assert_eq!(error.pos(), 2);
}

/// Java `Express4RunnerTest#invokeDefaultMethodTest`。覆盖接口默认方法继承、
/// 祖父接口覆盖，以及原始 `Map.entrySet().parallelStream().map().collect()`
/// Lambda 链。
#[test]
fn java_invoke_default_method_test() {
    let mut supplier = DefaultClassSupplier::instance();
    supplier.register("test.defaults.InterWithDefaultImplChild");
    supplier.register("test.defaults.InterWithDefaultImplGrandPaChild");
    let mut runner = Express4Runner::with_init_options(
        InitOptions::builder()
            .class_supplier(Rc::new(supplier))
            .add_default_import(vec![QLImport::import_pack("test.defaults")])
            .security_strategy(QLSecurityStrategy::open())
            .build(),
    );

    let mut interface = NativeType::named("test.defaults.InterWithDefault");
    interface.methods.insert(
        "haha".to_string(),
        Rc::new(|_bean, args| {
            if args.is_empty() {
                Ok(DataValue::Str("haha".into()))
            } else {
                Err(QLException::for_test(
                    qlexpress::exception::ql_exception::QLExceptionKind::Runtime,
                    "haha takes no arguments",
                    error_codes::INVOKE_METHOD_WITH_WRONG_ARGUMENTS,
                ))
            }
        }),
    );
    runner.register_native_type(interface);
    let mut child = NativeType::named("test.defaults.InterWithDefaultImplChild");
    child.supertypes = vec!["test.defaults.InterWithDefault".to_string()];
    child.constructor = Some(Rc::new(|args| {
        if args.is_empty() {
            Ok(RecordObject::value(
                "test.defaults.InterWithDefaultImplChild",
                &[],
            ))
        } else {
            Err(QLException::for_test(
                qlexpress::exception::ql_exception::QLExceptionKind::Runtime,
                "constructor takes no arguments",
                error_codes::NO_SUITABLE_CONSTRUCTOR,
            ))
        }
    }));
    runner.register_native_type(child);

    let mut grand_parent = NativeType::named("test.defaults.InterWithDefaultGrandPa");
    grand_parent.methods.insert(
        "haha".to_string(),
        Rc::new(|_bean, args| {
            if args.is_empty() {
                Ok(DataValue::Str("grandPa".into()))
            } else {
                Err(QLException::for_test(
                    qlexpress::exception::ql_exception::QLExceptionKind::Runtime,
                    "haha takes no arguments",
                    error_codes::INVOKE_METHOD_WITH_WRONG_ARGUMENTS,
                ))
            }
        }),
    );
    runner.register_native_type(grand_parent);
    let mut grand_child = NativeType::named("test.defaults.InterWithDefaultImplGrandPaChild");
    grand_child.supertypes = vec!["test.defaults.InterWithDefaultGrandPa".to_string()];
    grand_child.constructor = Some(Rc::new(|args| {
        if args.is_empty() {
            Ok(RecordObject::value(
                "test.defaults.InterWithDefaultImplGrandPaChild",
                &[],
            ))
        } else {
            Err(QLException::for_test(
                qlexpress::exception::ql_exception::QLExceptionKind::Runtime,
                "constructor takes no arguments",
                error_codes::NO_SUITABLE_CONSTRUCTOR,
            ))
        }
    }));
    runner.register_native_type(grand_child);

    assert_eq!(
        runner
            .execute(
                "a = new InterWithDefaultImplChild();a.haha()",
                HashMap::new(),
                &QLOptions::default(),
            )
            .expect("default interface method")
            .result(),
        &DataValue::Str("haha".into())
    );
    assert_eq!(
        runner
            .execute(
                "a = new InterWithDefaultImplGrandPaChild();a.haha()",
                HashMap::new(),
                &QLOptions::default(),
            )
            .expect("grand-parent default method")
            .result(),
        &DataValue::Str("grandPa".into())
    );

    let map = DataValue::map(IndexMap::from_entries(vec![
        (DataValue::Str("a".into()), DataValue::Str("123".into())),
        (DataValue::Str("b".into()), DataValue::Str("456".into())),
        (DataValue::Str("c".into()), DataValue::Str("789".into())),
    ]));
    let stream_result = runner
        .execute(
            concat!(
                "map.entrySet()",
                ".parallelStream().map(en -> en.getKey() + \":\" + en.getValue())",
                ".collect(Collectors.toList())"
            ),
            HashMap::from([("map".to_string(), map)]),
            &QLOptions::default(),
        )
        .expect("Java stream pipeline");
    assert_eq!(
        stream_result.result(),
        &DataValue::list(vec![
            DataValue::Str("a:123".into()),
            DataValue::Str("b:456".into()),
            DataValue::Str("c:789".into()),
        ])
    );
}

/// Java `Express4RunnerTest#concurrentCacheTest`。
///
/// `ADAPTED`：Java 共享可并发 Runner；Rust 使用线程本地 Runner 和共享
/// `ConcurrentParseCache`。保留 5 个工作线程、同一表达式、缓存启用、
/// 结果正确、首次编译去重和 5 秒完成时限。
#[test]
fn java_concurrent_cache_test() {
    let thread_count = 5;
    let cache = std::sync::Arc::new(ConcurrentParseCache::new());
    let expression = "a+b*c";
    let start = Instant::now();
    let mut workers = Vec::with_capacity(thread_count);
    for _ in 0..thread_count {
        let cache = std::sync::Arc::clone(&cache);
        workers.push(thread::spawn(move || {
            let runner = Express4Runner::new();
            let compiled = cache
                .get_or_compile(expression, || runner.export_parse_cache(expression))
                .map_err(|error| error.to_string())?;
            runner
                .set_parse_cache(&compiled)
                .map_err(|error| error.to_string())?;
            let result = runner
                .execute(
                    expression,
                    HashMap::from([
                        ("a".to_string(), DataValue::Int(1)),
                        ("b".to_string(), DataValue::Int(2)),
                        ("c".to_string(), DataValue::Int(3)),
                    ]),
                    &QLOptions::builder().cache(true).build(),
                )
                .map_err(|error| error.to_string())?;
            match result.into_result() {
                DataValue::Int(value) => Ok::<i64, String>(i64::from(value)),
                DataValue::Long(value) => Ok(value),
                other => Err(format!("integer result expected, got {other:?}")),
            }
        }));
    }
    for worker in workers {
        assert_eq!(
            worker
                .join()
                .expect("worker must not panic")
                .expect("worker execution"),
            7
        );
    }
    assert_eq!(cache.len(), 1);
    assert_eq!(cache.compile_count(), 1);
    assert!(start.elapsed() < Duration::from_secs(5));
}

