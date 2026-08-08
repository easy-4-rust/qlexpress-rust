struct HelloExtension;

impl ExtensionFunction for HelloExtension {
    fn parameter_types(&self) -> Vec<ClassRef> {
        Vec::new()
    }

    fn name(&self) -> &str {
        "hello"
    }

    fn declaring_class(&self) -> ClassRef {
        ClassRef::Named("java.lang.String".to_string())
    }

    fn invoke(
        &self,
        object: &DataValue,
        _arguments: &[DataValue],
    ) -> Result<DataValue, QLException> {
        Ok(DataValue::string(format!(
            "Hello,{}",
            object.string_value_of()
        )))
    }
}

struct OverloadedExtension {
    parameter_types: Vec<ClassRef>,
    result: &'static str,
}

impl ExtensionFunction for OverloadedExtension {
    fn parameter_types(&self) -> Vec<ClassRef> {
        self.parameter_types.clone()
    }

    fn name(&self) -> &str {
        "pick"
    }

    fn declaring_class(&self) -> ClassRef {
        ClassRef::Named("java.lang.String".to_string())
    }

    fn invoke(
        &self,
        _object: &DataValue,
        _arguments: &[DataValue],
    ) -> Result<DataValue, QLException> {
        Ok(DataValue::Str(self.result.into()))
    }
}

/// Java `Express4RunnerTest#extensionFunctionTest`。
#[test]
fn java_extension_function_test() {
    let mut runner = Express4Runner::new();
    runner.add_extend_function(HelloExtension);
    assert_eq!(
        runner
            .execute("'jack'.hello()", HashMap::new(), &QLOptions::default())
            .expect("string extension")
            .result(),
        &DataValue::Str("Hello,jack".into())
    );

    runner.add_extend_function_varargs(
        "add",
        ClassRef::Named("java.lang.Number".to_string()),
        |params: &[DataValue]| {
            Ok(DataValue::Int(
                params
                    .iter()
                    .filter_map(|value| match value {
                        DataValue::Int(value) => Some(*value),
                        _ => None,
                    })
                    .sum(),
            ))
        },
    );
    let add = runner
        .execute("1.add(2)", HashMap::new(), &QLOptions::default())
        .expect("number add extension");
    assert_integer(add.result(), 3);

    runner.add_extend_function_varargs(
        "add2",
        ClassRef::Named("java.lang.Number".to_string()),
        |params: &[DataValue]| {
            Ok(DataValue::Int(
                params
                    .iter()
                    .filter_map(|value| match value {
                        DataValue::Int(value) => Some(*value),
                        _ => None,
                    })
                    .sum(),
            ))
        },
    );
    let add2 = runner
        .execute("1.add2(2,3)", HashMap::new(), &QLOptions::default())
        .expect("number add2 extension");
    assert_integer(add2.result(), 6);
}

/// RUST_OBLIGATION: Java `ReflectLoader.extensionFunctions` 使用列表保存候选；
/// 同一声明类、同一方法名的不同签名必须共存并按实参选择。
#[test]
fn extension_function_overloads_do_not_overwrite_each_other() {
    let mut runner = Express4Runner::new();
    runner.add_extend_function(OverloadedExtension {
        parameter_types: Vec::new(),
        result: "zero",
    });
    runner.add_extend_function(OverloadedExtension {
        parameter_types: vec![ClassRef::from_name("int")],
        result: "one",
    });

    let zero = runner
        .execute("'x'.pick()", HashMap::new(), &QLOptions::default())
        .expect("zero-argument extension overload");
    assert_eq!(zero.result(), &DataValue::Str("zero".into()));

    let one = runner
        .execute("'x'.pick(7)", HashMap::new(), &QLOptions::default())
        .expect("one-argument extension overload");
    assert_eq!(one.result(), &DataValue::Str("one".into()));
}

/// RUST_OBLIGATION: Java
/// `Express4Runner#addExtendFunction(String, Class, QLFunctionalVarargs)`
/// 将接收者放在参数 0，并在其后展开全部脚本实参。
#[test]
fn varargs_extension_function_receives_receiver_then_script_arguments() {
    let mut runner = Express4Runner::new();
    runner.add_extend_function_varargs(
        "describe",
        ClassRef::Named("java.lang.String".to_string()),
        |arguments: &[DataValue]| {
            Ok(DataValue::string(
                arguments
                    .iter()
                    .map(DataValue::string_value_of)
                    .collect::<Vec<_>>()
                    .join("|"),
            ))
        },
    );

    let result = runner
        .execute(
            "'root'.describe(1, 'leaf')",
            HashMap::new(),
            &QLOptions::default(),
        )
        .expect("varargs extension");
    assert_eq!(result.result(), &DataValue::Str("root|1|leaf".into()));
}

/// RUST_OBLIGATION: Java `ReflectLoader` 由 runner 与已创建 Lambda 共享；
/// Lambda 创建后新增的扩展函数仍须对该 Lambda 可见。
#[test]
fn extension_registration_remains_visible_to_existing_lambda() {
    let mut runner = Express4Runner::new();
    let lambda = runner
        .parse_to_lambda(
            "'jack'.hello()",
            Rc::new(EmptyContext),
            &QLOptions::default(),
        )
        .expect("compile lambda before registering extension");

    runner.add_extend_function(HelloExtension);

    assert_eq!(
        lambda
            .q_lambda()
            .call(&[])
            .expect("existing lambda must observe later registration")
            .value(),
        DataValue::Str("Hello,jack".into())
    );
}

fn annotated_constant(value: i32) -> Rc<dyn CustomFunction> {
    Rc::new(move |_context: &mut dyn QContext, _parameters: &Parameters| Ok(DataValue::Int(value)))
}

struct AnnotatedObjectFunctions;

impl QLFunctionProvider for AnnotatedObjectFunctions {
    fn ql_object_function_methods(&self) -> Vec<QLFunctionMethod> {
        vec![
            // Java 先检查 public，因此未标注的 private 方法也进入 fail。
            QLFunctionMethod::new("hidden", false, None, annotated_constant(-1)),
            // public 且未标注的方法被忽略。
            QLFunctionMethod::new("ignored", true, None, annotated_constant(-1)),
            QLFunctionMethod::new(
                "sum",
                true,
                Some(vec!["annotated".to_string(), "duplicate".to_string()]),
                annotated_constant(7),
            ),
            QLFunctionMethod::new(
                "second",
                true,
                Some(vec!["duplicate".to_string()]),
                annotated_constant(9),
            ),
        ]
    }
}

struct AnnotatedStaticFunctions;

impl QLFunctionProvider for AnnotatedStaticFunctions {
    fn ql_static_function_methods() -> Vec<QLFunctionMethod> {
        vec![QLFunctionMethod::new(
            "staticValue",
            true,
            Some(vec!["staticValue".to_string()]),
            annotated_constant(11),
        )]
    }
}

/// `SOURCE_PARITY`：Java `Express4Runner#addObjFunction` 的声明方法扫描、
/// 非公开失败、无注解忽略、多别名重复记录和 put-if-absent 冲突语义。
#[test]
fn java_add_obj_function_annotation_scan() {
    let runner = Express4Runner::new();
    let result = runner.add_obj_function(&AnnotatedObjectFunctions);

    assert_eq!(
        result.get_succ(),
        &vec!["sum".to_string(), "sum".to_string()]
    );
    assert_eq!(
        result.get_fail(),
        &vec!["hidden".to_string(), "second".to_string()]
    );
    assert_integer(
        runner
            .execute("annotated()", HashMap::new(), &QLOptions::default())
            .expect("annotated function")
            .result(),
        7,
    );
    // duplicate 保留第一个注册的方法。
    assert_integer(
        runner
            .execute("duplicate()", HashMap::new(), &QLOptions::default())
            .expect("first duplicate function")
            .result(),
        7,
    );
}

/// `SOURCE_PARITY`：Java `Express4Runner#addStaticFunction(Class<?>)`。
#[test]
fn java_add_static_function_annotation_scan() {
    let runner = Express4Runner::new();
    let result = runner.add_static_function::<AnnotatedStaticFunctions>();
    assert_eq!(result.get_succ(), &vec!["staticValue".to_string()]);
    assert!(result.get_fail().is_empty());
    assert_integer(
        runner
            .execute("staticValue()", HashMap::new(), &QLOptions::default())
            .expect("static annotated function")
            .result(),
        11,
    );
}
