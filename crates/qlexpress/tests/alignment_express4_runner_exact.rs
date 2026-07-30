//! Java `Express4RunnerTest` 的逐方法精确迁移测试。
//!
//! 本文件只登记已经按 Java 方法输入与断言逐条复刻的用例；不能用一个
//! 宽泛 smoke test 代替多个 Java 方法。

#![allow(clippy::result_large_err)]

use std::collections::HashMap;
use std::rc::Rc;
use std::thread;
use std::time::Duration;

use qlexpress::aparser::import_manager::QLImport;
use qlexpress::check_options::CheckOptions;
use qlexpress::default_class_supplier::DefaultClassSupplier;
use qlexpress::exception::QLException;
use qlexpress::init_options::InitOptions;
use qlexpress::ql_options::{Attachments, QLOptions};
use qlexpress::runtime::class_ref::ClassRef;
use qlexpress::runtime::context::{DynamicVariableContext, ExpressContext};
use qlexpress::runtime::data::index_map::IndexMap;
use qlexpress::runtime::function::ExtensionFunction;
use qlexpress::runtime::jvm_i_method::NativeIMethod;
use qlexpress::runtime::meta_class::as_meta_class;
use qlexpress::runtime::native_type::NativeType;
use qlexpress::runtime::parameters::Parameters;
use qlexpress::runtime::qcontext::QContext;
use qlexpress::runtime::value::{DataValue, QValue};
use qlexpress::security::ql_security_strategy::QLSecurityStrategy;
use qlexpress::Express4Runner;

fn assert_integer(value: &DataValue, expected: i64) {
    match value {
        DataValue::Int(actual) => assert_eq!(i64::from(*actual), expected),
        DataValue::Long(actual) => assert_eq!(*actual, expected),
        other => panic!("expected integer {expected}, got {other:?}"),
    }
}

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
        .execute(
            "getCurrentTime()",
            HashMap::new(),
            &QLOptions::default(),
        )
        .expect("first current time")
        .into_result();
    thread::sleep(Duration::from_millis(3));
    let current_time_2 = runner
        .execute(
            "getCurrentTime()",
            HashMap::new(),
            &QLOptions::default(),
        )
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
        map.borrow().get(&DataValue::Str("aaa".to_string())),
        Some(&DataValue::Str("bbb".to_string()))
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
            as_meta_class(&result)
                .expect("class literal")
                .java_name(),
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
            .execute(
                "1.doubleValue()",
                HashMap::new(),
                &QLOptions::default()
            )
            .expect("number method invocation")
            .result(),
        &DataValue::Double(1.0)
    );
}

/// Java `Express4RunnerTest#addFunctionOfServiceMethodBasicTest`。
#[test]
fn java_add_function_of_service_method_basic_test() {
    let runner = Express4Runner::new();
    let method = NativeIMethod::from_native(
        "add",
        ClassRef::Named("MyFunctionUtil".to_string()),
        vec![ClassRef::from_name("int"), ClassRef::from_name("int")],
        Rc::new(|_object, arguments| match arguments {
            [DataValue::Int(left), DataValue::Int(right)] => Ok(DataValue::Int(left + right)),
            _ => unreachable!("svcAdd receives two ints"),
        }),
    );
    assert!(runner.add_function_of_class_method("svcAdd", None, method));
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
            [DataValue::Str(value)] => Ok(DataValue::Str(format!("S:{value}"))),
            _ => unreachable!("fmtStr receives one string"),
        }),
    );
    assert!(runner.add_function_of_class_method("fmtStr", None, string_method));
    assert_eq!(
        runner
            .execute("fmtStr('x')", HashMap::new(), &QLOptions::default())
            .expect("string overload")
            .result(),
        &DataValue::Str("S:x".to_string())
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
                Ok(DataValue::Str(format!("I:null,{right}")))
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
        &DataValue::Str("I:null,2".to_string())
    );
}

struct AttachmentPathContext;

impl ExpressContext for AttachmentPathContext {
    fn get(
        &self,
        attachments: &Attachments,
        variable_name: &str,
    ) -> Result<Option<QValue>, QLException> {
        let mut segments = variable_name.split('/').filter(|segment| !segment.is_empty());
        let Some(root) = segments.next() else {
            return Ok(None);
        };
        let Some(leaf) = segments.next() else {
            return Ok(None);
        };
        let value = attachments.get(root).and_then(|value| match value {
            DataValue::Map(map) => map
                .borrow()
                .get(&DataValue::Str(leaf.to_string()))
                .cloned(),
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
            vec![(DataValue::Str(key.to_string()), DataValue::Int(value))],
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
            Ok(DataValue::Str(format!("hello,{tenant}")))
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
                        DataValue::Str(tenant.to_string()),
                    )]))
                    .build(),
            )
            .expect("attachment-aware custom function");
        assert_eq!(
            result.result(),
            &DataValue::Str(format!("hello,{tenant}"))
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
            HashMap::from([("0".to_string(), DataValue::Str("World".to_string()))]),
            &QLOptions::default(),
        )
        .expect("custom selector");
    assert_eq!(result.result(), &DataValue::Str("Hello World".to_string()));
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
                DataValue::list(vec![
                    DataValue::Str("a".to_string()),
                    DataValue::Str("b".to_string()),
                ]),
            )]),
            &QLOptions::builder().precise(true).cache(true).build(),
        )
        .expect("precise list index");
    assert_eq!(result.result(), &DataValue::Str("b".to_string()));
}

/// Java `Express4RunnerTest#dynamicVariableComplexTest`。
#[test]
fn java_dynamic_variable_complex_test() {
    let runner = Rc::new(Express4Runner::new());
    let static_context = Rc::new(std::cell::RefCell::new(IndexMap::from_entries(vec![
        (DataValue::Str("语文".to_string()), DataValue::Int(88)),
        (DataValue::Str("数学".to_string()), DataValue::Int(99)),
        (DataValue::Str("英语".to_string()), DataValue::Int(95)),
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
        |_context: &mut dyn QContext, parameters: &Parameters| {
            Ok(parameters.get_value(0))
        }
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
    assert_eq!(
        result.result(),
        &DataValue::Str("formate string".to_string())
    );
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
        &DataValue::Str("java.util.ArrayList".to_string())
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
    assert_eq!(result.result(), &DataValue::Str("a".to_string()));
}

/// Java `Express4RunnerTest#importClsAliasLowercaseAliasTest`。
#[test]
#[should_panic(expected = "Alias must start with an uppercase letter: myList")]
fn java_import_cls_alias_lowercase_alias_test() {
    let _ = QLImport::import_cls_alias("java.util.ArrayList", "myList");
}

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
        Ok(DataValue::Str(format!(
            "Hello,{}",
            object.string_value_of()
        )))
    }
}

struct AddExtension {
    name: &'static str,
}

impl ExtensionFunction for AddExtension {
    fn parameter_types(&self) -> Vec<ClassRef> {
        vec![ClassRef::Named("java.lang.Object".to_string())]
    }

    fn name(&self) -> &str {
        self.name
    }

    fn declaring_class(&self) -> ClassRef {
        ClassRef::Named("java.lang.Number".to_string())
    }

    fn invoke(
        &self,
        object: &DataValue,
        arguments: &[DataValue],
    ) -> Result<DataValue, QLException> {
        let mut total = match object {
            DataValue::Int(value) => *value,
            _ => 0,
        };
        for argument in arguments {
            if let DataValue::Int(value) = argument {
                total += value;
            }
        }
        Ok(DataValue::Int(total))
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
        &DataValue::Str("Hello,jack".to_string())
    );

    runner.add_extend_function(AddExtension { name: "add" });
    let add = runner
        .execute("1.add(2)", HashMap::new(), &QLOptions::default())
        .expect("number add extension");
    assert_integer(add.result(), 3);

    runner.add_extend_function(AddExtension { name: "add2" });
    let add2 = runner
        .execute("1.add2(2,3)", HashMap::new(), &QLOptions::default())
        .expect("number add2 extension");
    assert_integer(add2.result(), 6);
}
