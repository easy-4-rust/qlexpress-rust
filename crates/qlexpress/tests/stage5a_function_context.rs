//! Stage 5a tests: runtime/function + runtime/context + member 体系。
//!
//! 覆盖:MapExpressContext 读写(含 pollute 写穿)/ QLAliasContext 别名 /
//! EmptyContext 空值语义 / ExtensionFunction(map/filter)经成员分派调用 /
//! CustomFunction 注册 + 经 QVM `CallFunctionInstruction` 端到端 /
//! LazyArg 不求值语义 / QMethodFunction(方法包装)/ 成员分派经
//! MethodInvokeUtils。

#[path = "stage3b_ops.rs"]
mod ops;
use ops::operator_manager;

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use qlexpress_rust::aparser::import_manager::ImportManager;
use qlexpress_rust::aparser::operator_factory::OperatorManager;
use qlexpress_rust::aparser::qlparser::build_tree;
use qlexpress_rust::aparser::qvm_instruction_visitor::{
    compile_script, CompileTimeFunctions, UserDefineFunctions,
};
use qlexpress_rust::class_supplier::DefaultClassSupplier;
use qlexpress_rust::exception::error_codes;
use qlexpress_rust::exception::pure_err_reporter::PureErrReporter;
use qlexpress_rust::exception::QLException;
use qlexpress_rust::init_options::InitOptions;
use qlexpress_rust::ql_options::QLOptions;
use qlexpress_rust::runtime::class_ref::ClassRef;
use qlexpress_rust::runtime::context::{
    DynamicVariableContext, EmptyContext, ExpressContext, MapExpressContext, QLAliasContext,
};
use qlexpress_rust::runtime::data::index_map::IndexMap;
use qlexpress_rust::runtime::function::{
    as_native_method, CustomFunction, FilterExtensionFunction, LazyArgCustomFunction,
    MapExtensionFunction, QMethodFunction,
};
use qlexpress_rust::runtime::instruction::Instruction;
use qlexpress_rust::runtime::jvm_i_method::NativeIMethod;
use qlexpress_rust::runtime::member::NativeRegistry;
use qlexpress_rust::runtime::native_type::NativeType;
use qlexpress_rust::runtime::parameters::Parameters;
use qlexpress_rust::runtime::qcontext::QContext;
use qlexpress_rust::runtime::qlambda_definition::QLambdaDefinition;
use qlexpress_rust::runtime::qlambda_definition_inner::QLambdaDefinitionInner;
use qlexpress_rust::runtime::qvm_global_scope::QvmGlobalScope;
use qlexpress_rust::runtime::qvm_runtime::QvmRuntime;
use qlexpress_rust::runtime::util::method_invoke_utils::find_method_and_invoke;
use qlexpress_rust::runtime::value::{DataValue, QValue};

// ---- harness(与 tests/stage3b_compile.rs 同构) -----------------------------

fn compile(
    script: &str,
    operator_manager: &OperatorManager,
    supplier: &DefaultClassSupplier,
    user_define_functions: &UserDefineFunctions,
) -> (Vec<Instruction>, usize) {
    let options = InitOptions::default();
    let tree = build_tree(
        script,
        Some(operator_manager),
        false,
        |_| {},
        options.interpolation_mode(),
        options.selector_start(),
        options.selector_end(),
        options.is_strict_new_lines(),
    )
    .unwrap_or_else(|err| panic!("parse failed for {script:?}: {err:?}"));
    let import_manager = RefCell::new(ImportManager::new(supplier, vec![]));
    compile_script(
        script,
        &tree,
        &import_manager,
        None,
        operator_manager,
        &CompileTimeFunctions::new(),
        user_define_functions,
        &options,
    )
    .unwrap_or_else(|err| panic!("compile failed for {script:?}: {err:?}"))
}

fn run_with_scope(
    script: &str,
    global_scope: QvmGlobalScope,
    registry: NativeRegistry,
    user_define_functions: UserDefineFunctions,
) -> DataValue {
    let operator_manager = operator_manager();
    let supplier = DefaultClassSupplier::instance();
    let (instructions, max_stack) =
        compile(script, &operator_manager, &supplier, &user_define_functions);
    let root: Rc<dyn QLambdaDefinition> = Rc::new(QLambdaDefinitionInner::new(
        "main",
        instructions,
        vec![],
        max_stack,
    ));
    let runtime = Rc::new(QvmRuntime::for_test(Rc::new(registry)));
    let options = QLOptions::builder().build();
    let result = runtime
        .execute(global_scope, root, &options)
        .unwrap_or_else(|err| panic!("execution failed for {script:?}: {err:?}"));
    result.value()
}

fn run(script: &str) -> DataValue {
    run_with_scope(
        script,
        QvmGlobalScope::empty(),
        NativeRegistry::with_builtins(),
        UserDefineFunctions::new(),
    )
}

fn map_context(entries: Vec<(&str, DataValue)>) -> Rc<RefCell<IndexMap>> {
    Rc::new(RefCell::new(IndexMap::from_entries(
        entries
            .into_iter()
            .map(|(k, v)| (DataValue::Str(k.to_string()), v))
            .collect(),
    )))
}

// ---- MapExpressContext:读写与 pollute 写穿 --------------------------------

#[test]
fn map_express_context_read_and_isolated_write() {
    // Java MapExpressContext:get 返回 MapItemValue;非 pollute 时
    // QvmGlobalScope 把外部值拷贝为脚本变量,脚本写不回宿主 Map。
    let external = map_context(vec![("x", DataValue::Int(40))]);
    let scope = QvmGlobalScope::new(Rc::clone(&external), HashMap::new(), false);
    assert_eq!(
        run_with_scope(
            "x = 1; x",
            scope,
            NativeRegistry::with_builtins(),
            UserDefineFunctions::new()
        ),
        DataValue::Int(1)
    );
    assert_eq!(
        external.borrow().get(&DataValue::Str("x".to_string())),
        Some(&DataValue::Int(40)),
        "non-pollute mode must not write through to the host map"
    );
}

#[test]
fn map_express_context_pollute_writes_through() {
    // Java:polluteUserContext 时 getSymbol 直接返回 MapItemValue,
    // 脚本赋值写穿宿主 Map。
    let external = map_context(vec![("x", DataValue::Int(40))]);
    let scope = QvmGlobalScope::new(Rc::clone(&external), HashMap::new(), true);
    assert_eq!(
        run_with_scope(
            "x = x + 2; x",
            scope,
            NativeRegistry::with_builtins(),
            UserDefineFunctions::new()
        ),
        DataValue::Int(42)
    );
    assert_eq!(
        external.borrow().get(&DataValue::Str("x".to_string())),
        Some(&DataValue::Int(42)),
        "pollute mode must write through to the host map"
    );
}

#[test]
fn map_express_context_get_returns_map_item_left_value() {
    let context = MapExpressContext::new(map_context(vec![("a", DataValue::Int(7))]));
    let attachments = HashMap::new();
    let value = context.get(&attachments, "a").unwrap().expect("hit");
    assert_eq!(value.get(), DataValue::Int(7));
    // 左值写入穿透(Java 靠 Map 引用别名实现)。
    if let QValue::Left(left) = value {
        left.borrow_mut().set_inner(DataValue::Int(8));
    } else {
        panic!("MapExpressContext must yield a LeftValue (MapItemValue)");
    }
    assert_eq!(
        context
            .source()
            .borrow()
            .get(&DataValue::Str("a".to_string())),
        Some(&DataValue::Int(8))
    );
}

// ---- QLAliasContext:别名单上下文 ------------------------------------------

#[test]
fn ql_alias_context_resolves_every_alias() {
    // Java:@QLAlias({"a", "b"}) 的对象以每个别名注册进同一上下文。
    let context: Rc<dyn ExpressContext> =
        Rc::new(QLAliasContext::new(&[(&["a", "b"], DataValue::Int(5))]));
    let scope = QvmGlobalScope::with_context(context, HashMap::new(), HashMap::new(), false);
    assert_eq!(
        run_with_scope(
            "a + b",
            scope,
            NativeRegistry::with_builtins(),
            UserDefineFunctions::new()
        ),
        DataValue::Int(10)
    );
}

// ---- EmptyContext:空值语义 -------------------------------------------------

#[test]
fn empty_context_yields_null_value_not_absent() {
    // Java EmptyContext.get 返回 Value.NULL_VALUE(非 null)。
    let context = EmptyContext::new();
    let attachments = HashMap::new();
    let value = context
        .get(&attachments, "anything")
        .unwrap()
        .expect("NULL_VALUE");
    assert_eq!(value.get(), DataValue::Null);
    // 经 QVM:未定义变量读为 null(而不是报错)。
    assert_eq!(run("foo"), DataValue::Null);
}

// ---- DynamicVariableContext ------------------------------------------------

#[test]
fn dynamic_variable_context_runs_script_lazily_per_get() {
    let calls = Rc::new(RefCell::new(0u32));
    let counter = Rc::clone(&calls);
    let runner: qlexpress_rust::runtime::context::DynamicScriptRunner =
        Rc::new(move |script, _ctx| {
            *counter.borrow_mut() += 1;
            assert_eq!(script, "1 + 2");
            Ok(DataValue::Int(99))
        });
    let context = DynamicVariableContext::new(runner, map_context(vec![("s", DataValue::Int(3))]));
    context.put("dyn", "1 + 2");
    let attachments = HashMap::new();
    // 动态变量:取值时执行脚本。
    assert_eq!(
        context
            .get(&attachments, "dyn")
            .unwrap()
            .expect("hit")
            .get(),
        DataValue::Int(99)
    );
    assert_eq!(*calls.borrow(), 1);
    // 静态变量:回退 MapItemValue。
    assert_eq!(
        context.get(&attachments, "s").unwrap().expect("hit").get(),
        DataValue::Int(3)
    );
    assert_eq!(*calls.borrow(), 1, "static lookup must not run any script");
}

// ---- ExtensionFunction:map / filter 经成员分派调用 --------------------------

/// 带 map/filter 扩展函数注册的注册表(对应 Java
/// `Express4Runner` 注册 `MapExtensionFunction.INSTANCE`/
/// `FilterExtensionFunction.INSTANCE` 的效果)。
fn registry_with_extensions() -> NativeRegistry {
    let mut registry = NativeRegistry::with_builtins();
    let mut list_type = NativeType::named("java.util.ArrayList");
    list_type.methods.insert(
        "map".to_string(),
        as_native_method(MapExtensionFunction::instance()),
    );
    list_type.methods.insert(
        "filter".to_string(),
        as_native_method(FilterExtensionFunction::instance()),
    );
    registry.register_type(list_type);
    registry
}

#[test]
fn extension_function_map_and_filter_end_to_end() {
    assert_eq!(
        run_with_scope(
            "[1, 2, 3].map(x -> x * 2)",
            QvmGlobalScope::empty(),
            registry_with_extensions(),
            UserDefineFunctions::new(),
        ),
        DataValue::list(vec![
            DataValue::Int(2),
            DataValue::Int(4),
            DataValue::Int(6)
        ])
    );
    assert_eq!(
        run_with_scope(
            "[1, 2, 3, 4].filter(x -> x % 2 == 0)",
            QvmGlobalScope::empty(),
            registry_with_extensions(),
            UserDefineFunctions::new(),
        ),
        DataValue::list(vec![DataValue::Int(2), DataValue::Int(4)])
    );
}

#[test]
fn extension_function_contract_matches_java() {
    use qlexpress_rust::runtime::function::ExtensionFunction;
    use qlexpress_rust::runtime::i_method::IMethod;

    let filter = FilterExtensionFunction::instance();
    // Java:getName/getDeclaringClass/getParameterTypes/isVarArgs/isAccess。
    assert_eq!(ExtensionFunction::name(&filter), "filter");
    assert_eq!(
        ExtensionFunction::declaring_class(&filter),
        ClassRef::Named("java.util.List".to_string())
    );
    assert_eq!(
        ExtensionFunction::parameter_types(&filter),
        vec![ClassRef::Named("java.util.function.Predicate".to_string())]
    );
    assert!(!ExtensionFunction::is_var_args(&filter));
    assert!(ExtensionFunction::is_access(&filter));
    // blanket impl:ExtensionFunction 即 IMethod。
    let as_method: &dyn IMethod = &filter;
    assert_eq!(as_method.name(), "filter");
    // Java:obj instanceof List 不成立时 invoke 返回 null。
    assert_eq!(
        as_method
            .invoke(&DataValue::Str("not a list".into()), &[])
            .unwrap(),
        DataValue::Null
    );
}

// ---- CustomFunction 注册 + QVM CallFunction 端到端 --------------------------

/// `greet(name)`:简单字符串函数,验证注册契约与 QVM 调用链。
struct Greet;

impl CustomFunction for Greet {
    fn call(
        &self,
        _q_context: &mut dyn QContext,
        parameters: &Parameters,
    ) -> Result<DataValue, QLException> {
        Ok(DataValue::Str(format!(
            "hi {}",
            parameters.get_value(0).string_value_of()
        )))
    }
}

#[test]
fn custom_function_registered_and_called_via_qvm() {
    let mut external_functions: HashMap<String, Rc<dyn CustomFunction>> = HashMap::new();
    external_functions.insert("greet".to_string(), Rc::new(Greet));
    let mut user_functions = UserDefineFunctions::new();
    user_functions.insert(
        "greet".to_string(),
        Rc::new(Greet) as Rc<dyn CustomFunction>,
    );
    let scope = QvmGlobalScope::new(
        Rc::new(RefCell::new(IndexMap::new())),
        external_functions,
        false,
    );
    assert_eq!(
        run_with_scope(
            "greet('ql') + '!'",
            scope,
            NativeRegistry::with_builtins(),
            user_functions
        ),
        DataValue::Str("hi ql!".into())
    );
}

// ---- LazyArg 不求值语义 -----------------------------------------------------

/// `pick(cond, lazyValue, fallback)`:第 2 个参数编译为 Lambda,仅在 cond 为真时求值。
struct Pick;

impl CustomFunction for Pick {
    fn call(
        &self,
        _q_context: &mut dyn QContext,
        parameters: &Parameters,
    ) -> Result<DataValue, QLException> {
        let values = parameters.values();
        match &values[..] {
            [DataValue::Bool(true), DataValue::Lambda(lambda), _] => Ok(lambda.call(&[])?.value()),
            [DataValue::Bool(false), _, fallback] => Ok(fallback.clone()),
            _ => Ok(DataValue::Null),
        }
    }

    fn as_lazy_arg(&self) -> Option<&dyn LazyArgCustomFunction> {
        Some(self)
    }
}

impl LazyArgCustomFunction for Pick {
    fn is_lazy_arg(&self, index: usize) -> bool {
        index == 1
    }
}

#[test]
fn lazy_arg_is_not_evaluated_unless_used() {
    let mut external_functions: HashMap<String, Rc<dyn CustomFunction>> = HashMap::new();
    external_functions.insert("pick".to_string(), Rc::new(Pick));
    let mut user_functions = UserDefineFunctions::new();
    user_functions.insert("pick".to_string(), Rc::new(Pick) as Rc<dyn CustomFunction>);
    let scope = QvmGlobalScope::new(
        Rc::new(RefCell::new(IndexMap::new())),
        external_functions,
        false,
    );
    // 惰性参数 `boom()` 的函数体不存在;若被求值会报 FUNCTION_NOT_FOUND。
    // cond=false 时不求值,直接取回退值。
    assert_eq!(
        run_with_scope(
            "pick(false, boom(), 'safe')",
            scope,
            NativeRegistry::with_builtins(),
            user_functions,
        ),
        DataValue::Str("safe".into())
    );
}

// ---- QMethodFunction:方法包装为函数 -----------------------------------------

#[test]
fn qmethod_function_invokes_wrapped_native_method() {
    // Java:QMethodFunction(object, method) 包装实例方法;
    // Rust:NativeIMethod 包装原生闭包(对应 JvmIMethod 包装反射 Method)。
    let method = NativeIMethod::from_native(
        "add",
        ClassRef::Named("com.example.Calc".to_string()),
        vec![ClassRef::from_name("int"), ClassRef::from_name("int")],
        Rc::new(|_bean, args| {
            let a = match &args[0] {
                DataValue::Int(v) => *v,
                _ => return Ok(DataValue::Null),
            };
            let b = match &args[1] {
                DataValue::Int(v) => *v,
                _ => return Ok(DataValue::Null),
            };
            Ok(DataValue::Int(a + b))
        }),
    );
    let function = QMethodFunction::new(Some(DataValue::Int(0)), method);
    let params = Parameters::new(vec![DataValue::Int(19).into(), DataValue::Int(23).into()]);
    let runtime = QvmRuntime::for_test(Rc::new(NativeRegistry::with_builtins()));
    let mut context = qlexpress_rust::runtime::delegate_qcontext::DelegateQContext::new(
        Rc::new(runtime),
        qlexpress_rust::runtime::scope::QScope::global(QvmGlobalScope::empty()),
    );
    assert_eq!(
        function.call(&mut context, &params).unwrap(),
        DataValue::Int(42)
    );
}

#[test]
fn qmethod_function_rejects_mismatched_argument_types() {
    let method = NativeIMethod::from_native(
        "shout",
        ClassRef::Named("com.example.Calc".to_string()),
        vec![ClassRef::from_name("int")],
        Rc::new(|_bean, _args| Ok(DataValue::Null)),
    );
    let function = QMethodFunction::new(None, method);
    // String 实参无法匹配 int 形参 → Java 抛 INVALID_ARGUMENT。
    let params = Parameters::new(vec![DataValue::Str("x".into()).into()]);
    let runtime = QvmRuntime::for_test(Rc::new(NativeRegistry::with_builtins()));
    let mut context = qlexpress_rust::runtime::delegate_qcontext::DelegateQContext::new(
        Rc::new(runtime),
        qlexpress_rust::runtime::scope::QScope::global(QvmGlobalScope::empty()),
    );
    let err = function.call(&mut context, &params).unwrap_err();
    assert_eq!(err.error_code(), error_codes::INVALID_ARGUMENT);
}

// ---- 成员分派经 MethodInvokeUtils -------------------------------------------

#[test]
fn method_dispatch_via_method_invoke_utils() {
    let registry = NativeRegistry::with_builtins();
    let reporter = PureErrReporter::INSTANCE;
    // String.length()
    assert_eq!(
        find_method_and_invoke(
            &DataValue::Str("abc".into()),
            "length",
            &[],
            &registry,
            &reporter
        )
        .unwrap()
        .get(),
        DataValue::Int(3)
    );
    // List.size()
    let list = DataValue::list(vec![DataValue::Int(1), DataValue::Int(2)]);
    assert_eq!(
        find_method_and_invoke(&list, "size", &[], &registry, &reporter)
            .unwrap()
            .get(),
        DataValue::Int(2)
    );
    // Map 中的 Lambda 可作为方法调用(Java findQLambdaInstance)。
    // 脚本端到端:成员不存在时按 METHOD_NOT_FOUND 报错码。
    let err = find_method_and_invoke(&list, "noSuchMethod", &[], &registry, &reporter).unwrap_err();
    assert_eq!(err.error_code(), error_codes::METHOD_NOT_FOUND);
}

#[test]
fn method_dispatch_through_script() {
    assert_eq!(run("'hello'.length()"), DataValue::Int(5));
    assert_eq!(run("[1, 2, 3].size()"), DataValue::Int(3));
    assert_eq!(run("'Abc'.toLowerCase()"), DataValue::Str("abc".into()));
}
