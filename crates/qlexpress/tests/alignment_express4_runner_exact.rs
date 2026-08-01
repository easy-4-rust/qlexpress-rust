//! Java `Express4RunnerTest` 的逐方法精确迁移测试。
//!
//! 本文件只登记已经按 Java 方法输入与断言逐条复刻的用例；不能用一个
//! 宽泛 smoke test 代替多个 Java 方法。

#![allow(clippy::result_large_err)]

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;
use std::thread;
use std::time::{Duration, Instant};

use qlexpress::aparser::import_manager::QLImport;
use qlexpress::api::parsecache::ConcurrentParseCache;
use qlexpress::check_options::CheckOptions;
use qlexpress::default_class_supplier::DefaultClassSupplier;
use qlexpress::exception::{error_codes, QLException};
use qlexpress::init_options::InitOptions;
use qlexpress::ql_options::{Attachments, QLOptions, SharedAttachments};
use qlexpress::runtime::class_ref::ClassRef;
use qlexpress::runtime::context::{DynamicVariableContext, EmptyContext, ExpressContext};
use qlexpress::runtime::data::index_map::IndexMap;
use qlexpress::runtime::function::{CustomFunction, ExtensionFunction};
use qlexpress::runtime::jvm_i_method::NativeIMethod;
use qlexpress::runtime::meta_class::as_meta_class;
use qlexpress::runtime::native_object::NativeObject;
use qlexpress::runtime::native_type::NativeType;
use qlexpress::runtime::parameters::Parameters;
use qlexpress::runtime::qcontext::QContext;
use qlexpress::runtime::value::{DataValue, QValue};
use qlexpress::security::ql_security_strategy::QLSecurityStrategy;
use qlexpress::{Express4Runner, QLFunctionMethod, QLFunctionProvider};

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
