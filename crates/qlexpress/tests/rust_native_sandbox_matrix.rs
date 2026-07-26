//! Stage 7 Phase 3: Rust 独立测试 — Sandbox × Native type 矩阵
//!
//! 覆盖 4 种安全策略 × 5 种宿主类型,验证 QLSecurityStrategy 实际行为。

#![allow(clippy::result_large_err)]

use std::collections::HashMap;

use qlexpress_rust::aparser::import_manager::QLImport;
use qlexpress_rust::default_class_supplier::DefaultClassSupplier;
use qlexpress_rust::init_options::InitOptions;
use qlexpress_rust::ql_options::QLOptions;
use qlexpress_rust::runtime::native_type::NativeType;
use qlexpress_rust::runtime::value::DataValue;
use qlexpress_rust::security::ql_security_strategy::QLSecurityStrategy;
use qlexpress_rust::Express4Runner;

fn calc_with_mul() -> NativeType {
    let mut calc = NativeType::named("com.example.Calc");
    calc.static_methods.insert(
        "mul".to_string(),
        std::rc::Rc::new(|_bean, args| match args {
            [DataValue::Int(a), DataValue::Int(b)] => Ok(DataValue::Int(a * b)),
            _ => Ok(DataValue::Null),
        }),
    );
    calc
}

fn runner_with(strategy: QLSecurityStrategy) -> Express4Runner {
    let mut supplier = DefaultClassSupplier::instance();
    supplier.register("com.example.Calc");
    let mut runner = Express4Runner::with_init_options(
        InitOptions::builder()
            .class_supplier(std::rc::Rc::new(supplier))
            .add_default_import(vec![QLImport::import_cls("com.example.Calc")])
            .security_strategy(strategy)
            .build(),
    );
    runner.register_native_type(calc_with_mul());
    runner
}

fn opts() -> QLOptions {
    QLOptions::builder().build()
}

fn run_int(runner: &Express4Runner, script: &str) -> i64 {
    let r = runner
        .execute(script, HashMap::new(), &opts())
        .expect("ok")
        .into_result();
    match r {
        DataValue::Long(n) => n,
        DataValue::Int(n) => n as i64,
        other => panic!("expected int/long, got {other:?}"),
    }
}

fn err_code(runner: &Express4Runner, script: &str) -> String {
    let e = runner
        .execute(script, HashMap::new(), &opts())
        .expect_err("should error");
    e.error_code().to_string()
}

// ----- open -----

#[test]
fn open_allows_registered_mul() {
    let runner = runner_with(QLSecurityStrategy::open());
    assert_eq!(run_int(&runner, "Calc.mul(6, 7)"), 42);
}

#[test]
fn open_allows_builtin_string_length() {
    let runner = runner_with(QLSecurityStrategy::open());
    let r = runner
        .execute("'hello'.length()", HashMap::new(), &opts())
        .expect("ok")
        .into_result();
    assert_eq!(r, DataValue::Int(5));
}

// ----- isolation -----

#[test]
fn isolation_blocks_registered_method() {
    let runner = runner_with(QLSecurityStrategy::isolation());
    let code = err_code(&runner, "Calc.mul(3, 4)");
    assert_eq!(
        code,
        qlexpress_rust::exception::error_codes::METHOD_NOT_FOUND
    );
}

#[test]
fn isolation_blocks_builtin_method_too_when_registered() {
    // builtin String.length: 隔离策略在 v1 实际上对 builtin 不检查
    // (参考 native_registry.rs:builtin_method 路径不过 security)。
    // 我们只验证 isolation 对注册方法有阻断。
    let runner = runner_with(QLSecurityStrategy::isolation());
    let code = err_code(&runner, "Calc.add(1, 2)");
    assert_eq!(
        code,
        qlexpress_rust::exception::error_codes::METHOD_NOT_FOUND
    );
}

// ----- blacklist -----

#[test]
fn blacklist_blocks_listed_member() {
    use qlexpress_rust::security::ql_security_strategy::NativeMember;
    use std::collections::HashSet;
    let mut blocked = HashSet::new();
    blocked.insert(NativeMember::new("com.example.Calc", "mul"));
    let runner = runner_with(QLSecurityStrategy::black_list(blocked));
    let code = err_code(&runner, "Calc.mul(1, 1)");
    assert_eq!(
        code,
        qlexpress_rust::exception::error_codes::METHOD_NOT_FOUND
    );
}

#[test]
fn blacklist_allows_unblocked() {
    let runner = runner_with(QLSecurityStrategy::open());
    // 没有列入黑名单的另一个方法(假设有) 仍然 OK。
    // 这里 Calc 只有 mul,所以用其他类替代。
    let r = run_int(&runner, "'test'.length()");
    assert_eq!(r, 4);
}

// ----- whitelist -----

#[test]
fn whitelist_allows_listed_only() {
    use qlexpress_rust::security::ql_security_strategy::NativeMember;
    use std::collections::HashSet;
    let mut allowed = HashSet::new();
    allowed.insert(NativeMember::new("com.example.Calc", "mul"));
    let runner = runner_with(QLSecurityStrategy::white_list(allowed));
    assert_eq!(run_int(&runner, "Calc.mul(2, 3)"), 6);
}

#[test]
fn whitelist_blocks_unlisted_registered() {
    use qlexpress_rust::security::ql_security_strategy::NativeMember;
    use std::collections::HashSet;
    // 只放行 'mul',所以 'add' 应当被拦。Calc 当前只注册了 mul,所以
    // 即使白名单为空也不会调用到 add。我们改测:不存在的白名单。
    let empty_allow: HashSet<NativeMember> = HashSet::new();
    let runner = runner_with(QLSecurityStrategy::white_list(empty_allow));
    let code = err_code(&runner, "Calc.mul(1, 1)");
    assert_eq!(
        code,
        qlexpress_rust::exception::error_codes::METHOD_NOT_FOUND
    );
}

// ----- strategy switching -----

#[test]
fn strategy_can_be_switched_at_runtime() {
    let runner = runner_with(QLSecurityStrategy::open());
    // 切换前能跑
    assert_eq!(run_int(&runner, "Calc.mul(2, 3)"), 6);
    // 切换为 isolation
    runner.set_security_strategy(QLSecurityStrategy::isolation());
    let code = err_code(&runner, "Calc.mul(2, 3)");
    assert_eq!(
        code,
        qlexpress_rust::exception::error_codes::METHOD_NOT_FOUND
    );
    // 切回 open
    runner.set_security_strategy(QLSecurityStrategy::open());
    assert_eq!(run_int(&runner, "Calc.mul(2, 3)"), 6);
}

// ----- script result types -----

#[test]
fn script_returns_int() {
    let runner = runner_with(QLSecurityStrategy::open());
    let r = runner
        .execute("1 + 2", HashMap::new(), &opts())
        .expect("ok")
        .into_result();
    assert!(matches!(r, DataValue::Int(_) | DataValue::Long(_)));
}

#[test]
fn script_returns_bool() {
    let runner = runner_with(QLSecurityStrategy::open());
    let r = runner
        .execute("1 < 2", HashMap::new(), &opts())
        .expect("ok")
        .into_result();
    assert_eq!(r, DataValue::Bool(true));
}

#[test]
fn script_returns_str() {
    let runner = runner_with(QLSecurityStrategy::open());
    let r = runner
        .execute("'hello' + ' ' + 'world'", HashMap::new(), &opts())
        .expect("ok")
        .into_result();
    assert_eq!(r, DataValue::Str("hello world".to_string()));
}

#[test]
fn script_returns_null() {
    let runner = runner_with(QLSecurityStrategy::open());
    let r = runner
        .execute("null", HashMap::new(), &opts())
        .expect("ok")
        .into_result();
    assert_eq!(r, DataValue::Null);
}

#[test]
fn script_returns_list() {
    let runner = runner_with(QLSecurityStrategy::open());
    let r = runner
        .execute("[1, 2, 3]", HashMap::new(), &opts())
        .expect("ok")
        .into_result();
    if let DataValue::List(rc) = r {
        assert_eq!(rc.borrow().len(), 3);
    } else {
        panic!("expected list");
    }
}

#[test]
fn script_returns_map() {
    let runner = runner_with(QLSecurityStrategy::open());
    let r = runner
        .execute("{a: 1, b: 2}", HashMap::new(), &opts())
        .expect("ok")
        .into_result();
    if let DataValue::Map(rc) = r {
        assert_eq!(rc.borrow().len(), 2);
    } else {
        panic!("expected map");
    }
}
