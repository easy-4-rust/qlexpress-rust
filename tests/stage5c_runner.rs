//! Stage 5c 端到端测试:全部经 `Express4Runner` 公共 API。
//!
//! 覆盖:基础 execute(算术/上下文/if/for/lambda/function)、自定义函数
//! (add_function/varargs/batch 部分失败/脚本定义函数)、类方法注册
//! (add_function_of_class_method/add_static_method)、自定义操作符
//! (add_operator/alias/黑名单策略拒绝)、安全策略(白名单放行/拒绝、
//! 黑名单拦截 method 调用)、out var/out function 收集、parse cache
//! 导出→导入→执行一致、宏、编译期函数、超时(QLTimeoutException)、
//! 语法错误行列号与运行时错误码。

// 与 lib 一致的架构性豁免:QLException 对齐 Java 单一异常类(见 lib.rs)。
#![allow(clippy::result_large_err)]

use std::collections::{HashMap, HashSet};
use std::rc::Rc;

use qlexpress_rust::api::parsecache::LoadedParseCache;
use qlexpress_rust::aparser::compile_time_function::{CodeGenerator, CompileTimeFunction};
use qlexpress_rust::aparser::import_manager::QLImport;
use qlexpress_rust::aparser::operator_factory::OperatorFactory;
use qlexpress_rust::aparser::syntax_tree_factory::Node;
use qlexpress_rust::check_options::CheckOptions;
use qlexpress_rust::default_class_supplier::DefaultClassSupplier;
use qlexpress_rust::exception::error_codes;
use qlexpress_rust::exception::ql_exception::QLExceptionKind;
use qlexpress_rust::init_options::InitOptions;
use qlexpress_rust::ql_options::QLOptions;
use qlexpress_rust::runtime::class_ref::ClassRef;
use qlexpress_rust::runtime::function::QMethodFunction;
use qlexpress_rust::runtime::instruction::ConstInstruction;
use qlexpress_rust::runtime::jvm_i_method::NativeIMethod;
use qlexpress_rust::runtime::native_type::NativeType;
use qlexpress_rust::runtime::parameters::Parameters;
use qlexpress_rust::runtime::qcontext::QContext;
use qlexpress_rust::runtime::value::DataValue;
use qlexpress_rust::security::ql_security_strategy::{NativeMember, QLSecurityStrategy};
use qlexpress_rust::{CustomFunction, Express4Runner, MapExpressContext};

fn opts() -> QLOptions {
    QLOptions::builder().build()
}

fn ctx(pairs: &[(&str, DataValue)]) -> HashMap<String, DataValue> {
    pairs
        .iter()
        .map(|(k, v)| (k.to_string(), v.clone()))
        .collect()
}

fn run(script: &str) -> DataValue {
    let runner = Express4Runner::new();
    runner
        .execute(script, HashMap::new(), &opts())
        .unwrap_or_else(|err| panic!("execute failed for {script:?}: {err:?}"))
        .into_result()
}

// ---- 基础 execute:算术 / 变量上下文 / if / for / lambda / function ---------

#[test]
fn execute_basic_arithmetic_and_context() {
    let runner = Express4Runner::new();
    assert_eq!(
        runner
            .execute("1 + 2 * 3", HashMap::new(), &opts())
            .unwrap()
            .into_result(),
        DataValue::Int(7)
    );
    let result = runner
        .execute(
            "a + b",
            ctx(&[("a", DataValue::Int(19)), ("b", DataValue::Int(23))]),
            &opts(),
        )
        .unwrap();
    assert_eq!(result.into_result(), DataValue::Int(42));
}

#[test]
fn execute_if_for_lambda_function() {
    assert_eq!(
        run("a = 0; if (a > 1) { 'big' } else { 'small' }"),
        DataValue::Str("small".to_string())
    );
    // for 循环求和
    assert_eq!(
        run("s = 0; for (i = 1; i <= 4; i = i + 1) { s = s + i; } s"),
        DataValue::Int(10)
    );
    // lambda
    assert_eq!(run("f = x -> x * 2; f(21)"), DataValue::Int(42));
    assert_eq!(
        run("f = (a, b) -> { return a * b; }; f(6, 7)"),
        DataValue::Int(42)
    );
    // 脚本内 function 定义
    assert_eq!(
        run("function add(a, b) { return a + b; } add(2, 3)"),
        DataValue::Int(5)
    );
}

#[test]
fn execute_template_wraps_dynamic_string() {
    let runner = Express4Runner::new();
    let result = runner
        .execute_template("hello", HashMap::new(), &opts())
        .unwrap();
    assert_eq!(result.into_result(), DataValue::Str("hello".to_string()));
}

// ---- 自定义函数:add_function / varargs / batch 部分失败 -------------------

#[test]
fn add_function_closure_and_varargs() {
    let runner = Express4Runner::new();
    // Java: addFunction(name, (qContext, parameters) -> ...)
    assert!(runner.add_function("dbl", |_ctx: &mut dyn QContext, params: &Parameters| {
        match params.get_value(0) {
            DataValue::Int(v) => Ok(DataValue::Int(v * 2)),
            _ => Ok(DataValue::Null),
        }
    }));
    // 同名再注册失败(Java putIfAbsent 语义)。
    assert!(!runner.add_function("dbl", |_ctx: &mut dyn QContext, _params: &Parameters| {
        Ok(DataValue::Null)
    }));
    assert_eq!(run_with(&runner, "dbl(21)"), DataValue::Int(42));

    // Java: addVarArgsFunction(name, params -> ...)
    assert!(runner.add_varargs_function("sumAll", |params: &[DataValue]| {
        let sum = params.iter().fold(0i32, |acc, v| match v {
            DataValue::Int(i) => acc + i,
            _ => acc,
        });
        Ok(DataValue::Int(sum))
    }));
    assert_eq!(run_with(&runner, "sumAll(1, 2, 3, 4)"), DataValue::Int(10));
}

#[test]
fn batch_add_function_partial_failure() {
    let runner = Express4Runner::new();
    assert!(runner.add_function("existing", |_ctx: &mut dyn QContext, _p: &Parameters| {
        Ok(DataValue::Int(1))
    }));
    let dup: Rc<dyn CustomFunction> = Rc::new(|_ctx: &mut dyn QContext, _p: &Parameters| {
        Ok(DataValue::Int(2))
    });
    let fresh: Rc<dyn CustomFunction> = Rc::new(|_ctx: &mut dyn QContext, _p: &Parameters| {
        Ok(DataValue::Int(3))
    });
    let result = runner.batch_add_function(vec![
        ("existing".to_string(), dup),
        ("fresh".to_string(), fresh),
    ]);
    // Java BatchAddFunctionResult 部分失败语义:同名冲突进 fail,其余进 succ。
    assert_eq!(result.get_fail(), &vec!["existing".to_string()]);
    assert_eq!(result.get_succ(), &vec!["fresh".to_string()]);
    assert!(!result.is_all_succ());
    assert_eq!(run_with(&runner, "fresh(0)"), DataValue::Int(3));
    // 冲突的旧函数保持原值。
    assert_eq!(run_with(&runner, "existing(0)"), DataValue::Int(1));
}

#[test]
fn add_functions_defined_in_script_registers_them() {
    let runner = Express4Runner::new();
    let context: Rc<dyn qlexpress_rust::ExpressContext> =
        Rc::new(MapExpressContext::new(Rc::new(std::cell::RefCell::new(
            qlexpress_rust::runtime::data::index_map::IndexMap::new(),
        ))));
    let result = runner
        .add_functions_defined_in_script(
            "function triple(x) { return x * 3; }",
            context,
            &opts(),
        )
        .unwrap();
    assert_eq!(result.get_succ(), &vec!["triple".to_string()]);
    assert_eq!(run_with(&runner, "triple(14)"), DataValue::Int(42));
}

// ---- 类方法注册:add_function_of_class_method / add_static_method ----------

#[test]
fn add_function_of_class_method_invokes_native_method() {
    let runner = Express4Runner::new();
    // Java: addFunctionOfServiceMethod(name, serviceObject, "mul", ...) —
    // Rust 显式给出 IMethod(SPEC §4)。
    let method = NativeIMethod::from_native(
        "mul",
        ClassRef::Named("com.example.Calc".to_string()),
        vec![ClassRef::from_name("int"), ClassRef::from_name("int")],
        Rc::new(|_bean, args| match args {
            [DataValue::Int(a), DataValue::Int(b)] => Ok(DataValue::Int(a * b)),
            _ => Ok(DataValue::Null),
        }),
    );
    assert!(runner.add_function_of_class_method("mul", None, method));
    assert_eq!(run_with(&runner, "mul(6, 7)"), DataValue::Int(42));
}

#[test]
fn add_static_method_resolves_from_registry() {
    let mut runner = Express4Runner::new();
    let mut calc = NativeType::named("com.example.Calc");
    calc.static_methods.insert(
        "mul".to_string(),
        Rc::new(|_bean, args| match args {
            [DataValue::Int(a), DataValue::Int(b)] => Ok(DataValue::Int(a * b)),
            _ => Ok(DataValue::Null),
        }),
    );
    runner.register_native_type(calc);
    assert!(runner.add_static_method("mul", "com.example.Calc", "mul"));
    // 未注册的方法名 → false(Java 抛 IllegalArgumentException)。
    assert!(!runner.add_static_method("nope", "com.example.Calc", "missing"));
    assert_eq!(run_with(&runner, "mul(6, 7)"), DataValue::Int(42));
}

// ---- 自定义操作符:add_operator / alias / 黑名单策略拒绝 -------------------

#[test]
fn add_operator_and_alias() {
    let mut runner = Express4Runner::new();
    // Java: addOperatorBiFunction("**", (a, b) -> Math.pow(a, b))
    assert!(runner.add_operator_bi("**", |left, right| match (left, right) {
        (DataValue::Int(a), DataValue::Int(b)) => DataValue::Int(a.pow(b as u32)),
        _ => DataValue::Null,
    }));
    assert_eq!(run_with(&runner, "2 ** 3"), DataValue::Int(8));
    // 内建操作符名冲突 → false。
    assert!(!runner.add_operator_bi("+", |l, _r| l));

    // Java: addAlias("plus", "+") → 操作符别名分支。
    assert!(runner.add_operator_alias("plus", "+"));
    assert_eq!(run_with(&runner, "1 plus 2"), DataValue::Int(3));
}

#[test]
fn replace_default_operator() {
    let mut runner = Express4Runner::new();
    // Java: replaceDefaultOperator("+", (l, r) -> 拼接)
    assert!(runner.replace_operator(
        "+",
        Rc::new(|left: &qlexpress_rust::QValue, right: &qlexpress_rust::QValue| {
            match (left.get(), right.get()) {
                (DataValue::Int(a), DataValue::Int(b)) => {
                    Ok(DataValue::Str(format!("{a}{b}")))
                }
                _ => Ok(DataValue::Null),
            }
        })
    ));
    assert_eq!(
        run_with(&runner, "1 + 2"),
        DataValue::Str("12".to_string())
    );
}

#[test]
fn check_rejects_blacklisted_operator() {
    let runner = Express4Runner::new();
    let forbidden: HashSet<String> = ["+".to_string()].into_iter().collect();
    let check_options = CheckOptions::builder()
        .operator_check_strategy(qlexpress_rust::operator::operator_check_strategy::OperatorCheckStrategy::blacklist(forbidden))
        .build();
    // 黑名单拒绝 `+`。
    let err = runner.check("1 + 2", &check_options).unwrap_err();
    assert!(err.is_syntax());
    // 未命中黑名单的脚本放行。
    assert!(runner.check("1 * 2", &check_options).is_ok());
    // 默认配置放行一切。
    assert!(runner.check_default("1 + 2").is_ok());
}

// ---- 安全策略:白名单放行/拒绝、黑名单拦截 method 调用 ----------------------

/// 带 `com.example.Calc` 静态方法 `mul` 的 runner(导入 `Calc` 类)。
fn runner_with_calc(security_strategy: QLSecurityStrategy) -> Express4Runner {
    let mut supplier = DefaultClassSupplier::instance();
    supplier.register("com.example.Calc");
    let init_options = InitOptions::builder()
        .class_supplier(Rc::new(supplier))
        .add_default_import(vec![QLImport::import_cls("com.example.Calc")])
        .security_strategy(security_strategy)
        .build();
    let mut runner = Express4Runner::with_init_options(init_options);
    let mut calc = NativeType::named("com.example.Calc");
    calc.static_methods.insert(
        "mul".to_string(),
        Rc::new(|_bean, args| match args {
            [DataValue::Int(a), DataValue::Int(b)] => Ok(DataValue::Int(a * b)),
            _ => Ok(DataValue::Null),
        }),
    );
    runner.register_native_type(calc);
    runner
}

#[test]
fn security_open_allows_static_method_call() {
    let runner = runner_with_calc(QLSecurityStrategy::open());
    assert_eq!(run_with(&runner, "Calc.mul(6, 7)"), DataValue::Int(42));
}

#[test]
fn security_isolation_blocks_method_call() {
    // Java InitOptions 默认 isolation:反射成员一律按「不存在」处理。
    let runner = runner_with_calc(QLSecurityStrategy::isolation());
    let err = runner
        .execute("Calc.mul(6, 7)", HashMap::new(), &opts())
        .unwrap_err();
    assert_eq!(err.error_code(), error_codes::METHOD_NOT_FOUND);
}

#[test]
fn security_white_list_allows_and_rejects() {
    let white: HashSet<NativeMember> =
        [NativeMember::new("com.example.Calc", "mul")].into_iter().collect();
    let runner = runner_with_calc(QLSecurityStrategy::white_list(white));
    // 白名单命中 → 放行。
    assert_eq!(run_with(&runner, "Calc.mul(6, 7)"), DataValue::Int(42));

    // 白名单不含目标成员 → 拒绝(METHOD_NOT_FOUND)。
    let runner = runner_with_calc(QLSecurityStrategy::white_list(HashSet::new()));
    let err = runner
        .execute("Calc.mul(6, 7)", HashMap::new(), &opts())
        .unwrap_err();
    assert_eq!(err.error_code(), error_codes::METHOD_NOT_FOUND);
}

#[test]
fn security_black_list_intercepts_method_call() {
    let black: HashSet<NativeMember> =
        [NativeMember::new("com.example.Calc", "mul")].into_iter().collect();
    let runner = runner_with_calc(QLSecurityStrategy::black_list(black));
    let err = runner
        .execute("Calc.mul(6, 7)", HashMap::new(), &opts())
        .unwrap_err();
    assert_eq!(err.error_code(), error_codes::METHOD_NOT_FOUND);

    // set_security_strategy 运行期切换为 open 后放行(接线点验证)。
    runner.set_security_strategy(QLSecurityStrategy::open());
    assert_eq!(run_with(&runner, "Calc.mul(6, 7)"), DataValue::Int(42));
}

// ---- out var / out function 收集 -------------------------------------------

#[test]
fn out_var_names_and_attrs() {
    let runner = Express4Runner::new();
    let names = runner.get_out_var_names("a + b.c").unwrap();
    let expected: HashSet<String> = ["a".to_string(), "b".to_string()].into_iter().collect();
    assert_eq!(names, expected);

    let attrs = runner.get_out_var_attrs("a + b.c").unwrap();
    assert!(attrs.contains(&vec!["b".to_string(), "c".to_string()]));
}

#[test]
fn out_function_names() {
    let runner = Express4Runner::new();
    let functions = runner.get_out_function_names("foo(1) + bar()").unwrap();
    let expected: HashSet<String> = ["foo".to_string(), "bar".to_string()].into_iter().collect();
    assert_eq!(functions, expected);
}

// ---- 宏与编译期函数 ---------------------------------------------------------

#[test]
fn add_macro_expands_inline() {
    let runner = Express4Runner::new();
    // Java: addMacro("inc", "a + 1")。
    assert!(runner.add_macro("inc", "a + 1").unwrap());
    // 同名宏重复注册失败(defineMacroIfAbsent 语义)。
    assert!(!runner.add_macro("inc", "a + 2").unwrap());
    assert_eq!(run_with(&runner, "a = 5; inc"), DataValue::Int(6));
    // addOrReplaceMacro 覆盖。
    runner.add_or_replace_macro("inc", "a + 2").unwrap();
    assert_eq!(run_with(&runner, "a = 5; inc"), DataValue::Int(7));
}

/// 编译期函数:`fortyTwo()` 编译期直接展开为常量 42
/// (对齐 tests/stage3b_compile.rs 的 FortyTwo)。
struct FortyTwo;

impl CompileTimeFunction for FortyTwo {
    fn create_function_instruction(
        &self,
        _function_name: &str,
        _arguments: &[&Node],
        _operator_factory: &dyn OperatorFactory,
        code_generator: &mut dyn CodeGenerator,
    ) {
        let reporter = code_generator.error_reporter();
        code_generator.add_instruction(Box::new(ConstInstruction::new(
            reporter,
            DataValue::Int(42),
            None,
        )));
    }
}

#[test]
fn add_compile_time_function() {
    let runner = Express4Runner::new();
    assert!(runner.add_compile_time_function("fortyTwo", Rc::new(FortyTwo)));
    assert!(!runner.add_compile_time_function("fortyTwo", Rc::new(FortyTwo)));
    assert!(runner.get_compile_time_function("fortyTwo").is_some());
    assert_eq!(run_with(&runner, "fortyTwo()"), DataValue::Int(42));
}

// ---- parse cache:导出 → 导入 → 执行结果一致 ---------------------------------

#[test]
fn parse_cache_export_import_execute_consistent() {
    let runner = Express4Runner::new();
    let script = "a * 2 + 1";
    let context = ctx(&[("a", DataValue::Int(20))]);
    let direct = runner.execute(script, context.clone(), &opts()).unwrap();

    // 导出 → 导入 → 执行,结果与直接执行一致。
    let cache = runner.export_parse_cache(script).unwrap();
    let loaded: LoadedParseCache = runner.import_parse_cache(&cache).unwrap();
    assert!(loaded.is_bound_to(runner.identity()));
    let map_context: Rc<dyn qlexpress_rust::ExpressContext> =
        Rc::new(MapExpressContext::new(Rc::new(std::cell::RefCell::new(
            qlexpress_rust::runtime::data::index_map::IndexMap::from_entries(
                context
                    .into_iter()
                    .map(|(k, v)| (DataValue::Str(k), v))
                    .collect(),
            ),
        ))));
    let cached = runner
        .execute_with_loaded_cache(&loaded, map_context, &opts())
        .unwrap();
    assert_eq!(cached.result(), direct.result());

    // set_parse_cache 预热后,cache=true 的执行路径命中。
    runner.set_parse_cache(&cache).unwrap();
    let warm = runner
        .execute(
            script,
            ctx(&[("a", DataValue::Int(20))]),
            &QLOptions::builder().cache(true).build(),
        )
        .unwrap();
    assert_eq!(warm.result(), direct.result());
}

#[test]
fn execute_with_serializable_cache() {
    let runner = Express4Runner::new();
    let cache = runner.export_parse_cache("6 * 7").unwrap();
    let empty: Rc<dyn qlexpress_rust::ExpressContext> =
        Rc::new(MapExpressContext::new(Rc::new(std::cell::RefCell::new(
            qlexpress_rust::runtime::data::index_map::IndexMap::new(),
        ))));
    let result = runner.execute_with_cache(&cache, empty, &opts()).unwrap();
    assert_eq!(result.into_result(), DataValue::Int(42));
}

// ---- 超时:max_time_millis 触发 QLTimeoutException ---------------------------

#[test]
fn timeout_triggers_ql_timeout_exception() {
    let runner = Express4Runner::new();
    let ql_options = QLOptions::builder().timeout_millis(1).build();
    let err = runner
        .execute("x = 0; while (true) { x = x + 1 }", HashMap::new(), &ql_options)
        .unwrap_err();
    assert_eq!(err.kind(), QLExceptionKind::Timeout);
    assert_eq!(err.error_code(), error_codes::SCRIPT_TIME_OUT);
}

// ---- 错误:语法错误行列号、运行错误错误码 --------------------------------------

#[test]
fn syntax_error_carries_line_and_col() {
    let runner = Express4Runner::new();
    let err = runner.execute("1 +", HashMap::new(), &opts()).unwrap_err();
    assert_eq!(err.kind(), QLExceptionKind::Syntax);
    assert!(err.is_syntax());
    assert!(err.line_no() >= 1, "line number expected: {err:?}");
    assert!(err.col_no() >= 1, "column number expected: {err:?}");
}

#[test]
fn runtime_error_carries_error_code() {
    let runner = Express4Runner::new();
    let err = runner
        .execute("undefinedFunction(1)", HashMap::new(), &opts())
        .unwrap_err();
    // Java 语义:调用未定义函数报 FUNCTION_NOT_FOUND(Stage 6 对齐测试
    // avoidnullpointer/can_not_find_function.ql 校正,此前误为
    // FUNCTION_TYPE_MISMATCH)。
    assert_eq!(err.error_code(), error_codes::FUNCTION_NOT_FOUND);
}

// ---- 工具 ------------------------------------------------------------------

fn run_with(runner: &Express4Runner, script: &str) -> DataValue {
    runner
        .execute(script, HashMap::new(), &opts())
        .unwrap_or_else(|err| panic!("execute failed for {script:?}: {err:?}"))
        .into_result()
}

// 保留 QMethodFunction 引用:确认其作为 add_function_of_class_method 的
// 底层包装类型公开可用(Java QMethodFunction 为 public 类)。
#[allow(dead_code)]
fn qmethod_function_is_public(method: Rc<dyn qlexpress_rust::runtime::i_method::IMethod>) {
    let _ = QMethodFunction::new(None, method);
}
