struct AttachmentPathContext;

impl ExpressContext for AttachmentPathContext {
    fn get(
        &self,
        attachments: &Attachments,
        variable_name: &str,
    ) -> Result<Option<QValue>, QLException> {
        let mut segments = variable_name
            .split('/')
            .filter(|segment| !segment.is_empty());
        let Some(root) = segments.next() else {
            return Ok(None);
        };
        let Some(leaf) = segments.next() else {
            return Ok(None);
        };
        let value = attachments.get(root).and_then(|value| match value {
            DataValue::Map(map) => map.borrow().get(&DataValue::Str(leaf.into())).cloned(),
            _ => None,
        });
        Ok(value.map(QValue::Data))
    }
}

/// Java `Express4RunnerTest#customExpressKeyValue`。
#[test]
fn java_custom_express_key_value() {
    fn nested(key: &str, value: i32) -> DataValue {
        DataValue::Map(Rc::new(std::cell::RefCell::new(IndexMap::from_entries(
            vec![(DataValue::Str(key.into()), DataValue::Int(value))],
        ))))
    }
    let attachments = HashMap::from([
        ("a".to_string(), nested("aa", 123)),
        ("b".to_string(), nested("bb", 12)),
    ]);
    let result = Express4Runner::new()
        .execute_with_context(
            "${/a/aa} + ${/b/bb}",
            Rc::new(AttachmentPathContext),
            &QLOptions::builder().attachments(attachments).build(),
        )
        .expect("custom context selector");
    assert_integer(result.result(), 135);
}

/// Java `Express4RunnerTest#customComplexFunctionDocTest`。
#[test]
fn java_custom_complex_function_doc_test() {
    let runner = Express4Runner::new();
    assert!(runner.add_function(
        "hello",
        |context: &mut dyn QContext, _parameters: &Parameters| {
            let tenant = context
                .attachment()
                .get("tenant")
                .map(DataValue::string_value_of)
                .unwrap_or_default();
            Ok(DataValue::string(format!("hello,{tenant}")))
        }
    ));
    for tenant in ["jack", "lucy"] {
        let result = runner
            .execute(
                "hello()",
                HashMap::new(),
                &QLOptions::builder()
                    .attachments(HashMap::from([(
                        "tenant".to_string(),
                        DataValue::Str(tenant.into()),
                    )]))
                    .build(),
            )
            .expect("attachment-aware custom function");
        assert_eq!(
            result.result(),
            &DataValue::string(format!("hello,{tenant}"))
        );
    }
}

/// Java `Express4RunnerTest#customSelectorTest`。
#[test]
fn java_custom_selector_test() {
    let runner = Express4Runner::with_init_options(
        InitOptions::builder()
            .selector_start("#[")
            .selector_end("]")
            .build(),
    );
    let result = runner
        .execute(
            "'Hello ' + #[0]",
            HashMap::from([("0".to_string(), DataValue::Str("World".into()))]),
            &QLOptions::default(),
        )
        .expect("custom selector");
    assert_eq!(result.result(), &DataValue::Str("Hello World".into()));
}

/// Java `Express4RunnerTest#customSelectorWhenNoCloseTest`。
#[test]
fn java_custom_selector_when_no_close_test() {
    let runner = Express4Runner::with_init_options(
        InitOptions::builder()
            .selector_start("#[")
            .selector_end("]")
            .build(),
    );
    for script in ["'Hello ' + #[0grg", "'Hello ' + ${pl}"] {
        let error = runner
            .execute(script, HashMap::new(), &QLOptions::default())
            .expect_err("invalid selector must fail");
        assert_eq!(error.error_code(), "SYNTAX_ERROR");
    }
}

/// Java `Express4RunnerTest#listGetWhenPreciseTest`。
#[test]
fn java_list_get_when_precise_test() {
    let runner = Express4Runner::with_init_options(
        InitOptions::builder()
            .security_strategy(QLSecurityStrategy::open())
            .build(),
    );
    let result = runner
        .execute(
            "list.get(list.size()-1);",
            HashMap::from([(
                "list".to_string(),
                DataValue::list(vec![DataValue::Str("a".into()), DataValue::Str("b".into())]),
            )]),
            &QLOptions::builder().precise(true).cache(true).build(),
        )
        .expect("precise list index");
    assert_eq!(result.result(), &DataValue::Str("b".into()));
}

/// Java `Express4RunnerTest#dynamicVariableComplexTest`。
#[test]
fn java_dynamic_variable_complex_test() {
    let runner = Rc::new(Express4Runner::new());
    let static_context = Rc::new(std::cell::RefCell::new(IndexMap::from_entries(vec![
        (DataValue::Str("语文".into()), DataValue::Int(88)),
        (DataValue::Str("数学".into()), DataValue::Int(99)),
        (DataValue::Str("英语".into()), DataValue::Int(95)),
    ])));
    let dynamic: Rc<DynamicVariableContext> =
        Rc::new_cyclic(|weak: &std::rc::Weak<DynamicVariableContext>| {
            let weak_context = weak.clone();
            let nested_runner = Rc::clone(&runner);
            DynamicVariableContext::new(
                Rc::new(move |script, _context| {
                    let context: Rc<dyn ExpressContext> =
                        weak_context.upgrade().expect("dynamic context is alive");
                    nested_runner
                        .execute_with_context(script, context, &QLOptions::default())
                        .map(qlexpress::QLResult::into_result)
                }),
                Rc::clone(&static_context),
            )
        });
    dynamic.put("平均成绩", "(语文+数学+英语)/3.0");
    dynamic.put("是否优秀", "平均成绩>90");

    let excellent = runner
        .execute_with_context(
            "是否优秀",
            Rc::clone(&dynamic) as Rc<dyn ExpressContext>,
            &QLOptions::default(),
        )
        .expect("dynamic boolean");
    assert_eq!(excellent.result(), &DataValue::Bool(true));
    let average = runner
        .execute_with_context(
            "平均成绩",
            Rc::clone(&dynamic) as Rc<dyn ExpressContext>,
            &QLOptions::default(),
        )
        .expect("dynamic average");
    assert_eq!(
        average
            .result()
            .string_value_of()
            .split('.')
            .next()
            .unwrap_or_default(),
        "94"
    );
    let static_sum = runner
        .execute_with_context(
            "语文+数学",
            dynamic as Rc<dyn ExpressContext>,
            &QLOptions::default(),
        )
        .expect("static variables");
    assert_integer(static_sum.result(), 187);
}

/// Java `Express4RunnerTest#testDefaultAllowFunctionCall`。
#[test]
fn java_test_default_allow_function_call() {
    Express4Runner::new()
        .check("Math.max(1, 2)", &CheckOptions::default())
        .expect("default checker allows calls");
}

/// Java `Express4RunnerTest#testDisableFunctionCalls`。
#[test]
fn java_test_disable_function_calls() {
    let options = CheckOptions::builder().disable_function_calls(true).build();
    let error = Express4Runner::new()
        .check("Math.max(1, 2)", &options)
        .expect_err("function calls disabled");
    assert!(error.to_string().contains("Function calls are not allowed"));
}

/// Java `Express4RunnerTest#testDisableDifferentFunctionCallStyles`。
#[test]
fn java_test_disable_different_function_call_styles() {
    let runner = Express4Runner::new();
    let options = CheckOptions::builder().disable_function_calls(true).build();
    for script in ["func()", "obj.method()"] {
        assert!(
            runner.check(script, &options).is_err(),
            "{script} must be rejected"
        );
    }
}

/// Java `Express4RunnerTest#testDisableFunctionCallsAllowOtherSyntax`。
#[test]
fn java_test_disable_function_calls_allow_other_syntax() {
    let runner = Express4Runner::new();
    let options = CheckOptions::builder().disable_function_calls(true).build();
    for script in ["1 + 2", "x = 5", "x > 3 ? 'yes' : 'no'", "{a: 1, b: 2}"] {
        runner
            .check(script, &options)
            .unwrap_or_else(|error| panic!("{script} must remain valid: {error:?}"));
    }
}

/// Java `Express4RunnerTest#qlStaticFunctionTest`。
#[test]
fn java_ql_static_function_test() {
    let runner = Express4Runner::new();
    assert!(runner.add_function(
        "_str_formate",
        |_context: &mut dyn QContext, parameters: &Parameters| { Ok(parameters.get_value(0)) }
    ));
    let result = runner
        .execute(
            concat!(
                "return formate(params);\n",
                "function formate(params) {\n",
                "    return _str_formate(\"formate string\", params);\n",
                "}"
            ),
            HashMap::from([(
                "params".to_string(),
                DataValue::Map(Rc::new(std::cell::RefCell::new(IndexMap::new()))),
            )]),
            &QLOptions::default(),
        )
        .expect("static annotated function adaptation");
    assert_eq!(result.result(), &DataValue::Str("formate string".into()));
}

/// Java `Express4RunnerTest#listSpreadTest`。
#[test]
fn java_list_spread_test() {
    let runner = Express4Runner::with_init_options(
        InitOptions::builder()
            .security_strategy(QLSecurityStrategy::open())
            .build(),
    );
    let result = runner
        .execute(
            "[[1,2],[],[3],[]]*.isEmpty()",
            HashMap::new(),
            &QLOptions::default(),
        )
        .expect("spread isEmpty");
    assert_eq!(
        result.result(),
        &DataValue::list(vec![
            DataValue::Bool(false),
            DataValue::Bool(true),
            DataValue::Bool(false),
            DataValue::Bool(true),
        ])
    );
}

/// Java `Express4RunnerTest#importClsAliasTest`。
#[test]
fn java_import_cls_alias_test() {
    let runner = Express4Runner::with_init_options(
        InitOptions::builder()
            .security_strategy(QLSecurityStrategy::open())
            .add_default_import(vec![QLImport::import_cls_alias(
                "java.util.ArrayList",
                "MyList",
            )])
            .build(),
    );
    let size = runner
        .execute(
            "list = new MyList(); list.add(1); list.add(2); list.size()",
            HashMap::new(),
            &QLOptions::default(),
        )
        .expect("aliased ArrayList");
    assert_integer(size.result(), 2);
    assert_eq!(
        runner
            .execute(
                "MyList.class.getName()",
                HashMap::new(),
                &QLOptions::default()
            )
            .expect("alias class name")
            .result(),
        &DataValue::Str("java.util.ArrayList".into())
    );
}

/// Java `Express4RunnerTest#importClsAliasMultipleTest`。
#[test]
fn java_import_cls_alias_multiple_test() {
    let runner = Express4Runner::with_init_options(
        InitOptions::builder()
            .security_strategy(QLSecurityStrategy::open())
            .add_default_import(vec![
                QLImport::import_cls_alias("java.util.ArrayList", "MyList"),
                QLImport::import_cls_alias("java.util.HashMap", "MyMap"),
            ])
            .build(),
    );
    let result = runner
        .execute(
            concat!(
                "list = new MyList(); list.add('a'); ",
                "map = new MyMap(); map.put('key', list); ",
                "map.get('key').get(0)"
            ),
            HashMap::new(),
            &QLOptions::default(),
        )
        .expect("multiple class aliases");
    assert_eq!(result.result(), &DataValue::Str("a".into()));
}

/// Java `Express4RunnerTest#importClsAliasLowercaseAliasTest`。
#[test]
#[should_panic(expected = "Alias must start with an uppercase letter: myList")]
fn java_import_cls_alias_lowercase_alias_test() {
    let _ = QLImport::import_cls_alias("java.util.ArrayList", "myList");
}

#[derive(Default)]
struct RecordObject {
    type_name: String,
    fields: HashMap<String, DataValue>,
}

impl RecordObject {
    fn value(type_name: &str, fields: &[(&str, DataValue)]) -> DataValue {
        DataValue::Object(Rc::new(RefCell::new(Self {
            type_name: type_name.to_string(),
            fields: fields
                .iter()
                .map(|(name, value)| ((*name).to_string(), value.clone()))
                .collect(),
        })))
    }
}

impl NativeObject for RecordObject {
    fn get_field(&self, name: &str) -> Option<DataValue> {
        self.fields.get(name).cloned()
    }

    fn set_field(&mut self, name: &str, value: &DataValue) -> bool {
        let Some(slot) = self.fields.get_mut(name) else {
            return false;
        };
        *slot = value.clone();
        true
    }

    fn call_method(&mut self, name: &str, _args: &[DataValue]) -> Result<DataValue, QLException> {
        match name {
            "getBirth" => Ok(self.fields.get("birth").cloned().unwrap_or(DataValue::Null)),
            "getAge" => {
                let year = self
                    .fields
                    .get("birth")
                    .and_then(DataValue::as_str)
                    .and_then(|birth| birth.get(0..4))
                    .and_then(|year| year.parse::<i32>().ok())
                    .unwrap_or_default();
                Ok(DataValue::Int(2021 - year))
            }
            _ => Err(QLException::for_test(
                qlexpress::exception::ql_exception::QLExceptionKind::Runtime,
                format!("method not found: {name}"),
                error_codes::METHOD_NOT_FOUND,
            )),
        }
    }

    fn native_type_name(&self) -> &str {
        &self.type_name
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

fn record_native_type(
    type_name: &str,
    fields: &[(&str, &[&str], bool)],
    methods: &[(&str, &[&str])],
) -> NativeType {
    let mut native_type = NativeType::named(type_name);
    for (field_name, aliases, writable) in fields {
        let getter_name = (*field_name).to_string();
        native_type.fields.insert(
            getter_name.clone(),
            Rc::new(move |bean| {
                let DataValue::Object(object) = bean else {
                    return None;
                };
                object.borrow().get_field(&getter_name)
            }),
        );
        if *writable {
            let setter_name = (*field_name).to_string();
            native_type.field_setters.insert(
                setter_name.clone(),
                Rc::new(move |bean, value| {
                    let DataValue::Object(object) = bean else {
                        return false;
                    };
                    object.borrow_mut().set_field(&setter_name, value)
                }),
            );
        }
        if !aliases.is_empty() {
            native_type.field_aliases.insert(
                (*field_name).to_string(),
                aliases.iter().map(|alias| (*alias).to_string()).collect(),
            );
        }
    }
    for (method_name, aliases) in methods {
        let invoked_name = (*method_name).to_string();
        native_type.methods.insert(
            invoked_name.clone(),
            Rc::new(move |bean, args| {
                let DataValue::Object(object) = bean else {
                    return Err(QLException::for_test(
                        qlexpress::exception::ql_exception::QLExceptionKind::Runtime,
                        "native object expected",
                        error_codes::INVALID_ARGUMENT,
                    ));
                };
                object.borrow_mut().call_method(&invoked_name, args)
            }),
        );
        if !aliases.is_empty() {
            native_type.method_aliases.insert(
                (*method_name).to_string(),
                aliases.iter().map(|alias| (*alias).to_string()).collect(),
            );
        }
    }
    native_type
}
