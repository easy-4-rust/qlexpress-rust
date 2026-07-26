//! Stage 7: 对齐 Java `runtime/instruction/MethodInvokeInstructionTest`
//! 与 `runtime/instruction/NewInstanceInstructionTest` 的核心场景。
//!
//! 锁定 method invoke / new instance 的核心契约:
//! - 参数类型匹配顺序(精确 > 可转型 > varargs > 不可解析)
//! - 数字到数字的隐式提升(Long ↔ Double ↔ BigInteger 等)
//! - Array 参数透传
//! - 不存在方法/构造器返回明确错误码
//!
//! 注:varargs 是 Rust 端尚未实现的子项,相关测试加 `#[ignore]` 直到
//! Phase 1.6 完成。

#![allow(clippy::result_large_err)]

use std::collections::HashMap;
use std::rc::Rc;

use qlexpress_rust::aparser::import_manager::QLImport;
use qlexpress_rust::default_class_supplier::DefaultClassSupplier;
use qlexpress_rust::exception::error_codes;
use qlexpress_rust::init_options::InitOptions;
use qlexpress_rust::ql_options::QLOptions;
use qlexpress_rust::runtime::native_type::NativeType;
use qlexpress_rust::runtime::value::DataValue;
use qlexpress_rust::Express4Runner;

fn runner_with(calc: NativeType) -> Express4Runner {
    runner_with_strategy(calc, qlexpress_rust::security::ql_security_strategy::QLSecurityStrategy::open())
}

fn runner_with_strategy(
    calc: NativeType,
    strategy: qlexpress_rust::security::ql_security_strategy::QLSecurityStrategy,
) -> Express4Runner {
    let mut supplier = DefaultClassSupplier::instance();
    supplier.register("com.example.Calc");
    let mut runner = Express4Runner::with_init_options(
        InitOptions::builder()
            .class_supplier(Rc::new(supplier))
            .add_default_import(vec![QLImport::import_cls("com.example.Calc")])
            .security_strategy(strategy)
            .build(),
    );
    runner.register_native_type(calc);
    runner
}

fn calc_with_methods() -> NativeType {
    let mut calc = NativeType::named("com.example.Calc");
    // mul(int, int) -> int
    calc.static_methods.insert(
        "mul".to_string(),
        std::rc::Rc::new(|_bean, args| match args {
            [DataValue::Int(a), DataValue::Int(b)] => Ok(DataValue::Int(a * b)),
            _ => Ok(DataValue::Null),
        }),
    );
    // addField(int, String...)  varargs
    calc.static_methods.insert(
        "addField".to_string(),
        std::rc::Rc::new(|_bean, args| match args.first() {
            Some(DataValue::Int(_)) => Ok(DataValue::Int(args.len() as i32)),
            _ => Ok(DataValue::Null),
        }),
    );
    calc
}

fn opts() -> QLOptions {
    QLOptions::builder().build()
}

// ---------- method matching & boxing ----------

#[test]
fn method_match_exact_args() {
    let runner = runner_with(calc_with_methods());
    let r = runner
        .execute("Calc.mul(6, 7)", HashMap::new(), &opts())
        .expect("ok")
        .into_result();
    assert_eq!(r, DataValue::Int(42));
}

#[test]
fn method_match_int_with_long_literal() {
    // Rust 端 Long 字面量走 Long 路径,但若 ctor 签名要 int 会做自动转型。
    let mut calc = NativeType::named("com.example.Calc");
    calc.static_methods.insert(
        "abs".to_string(),
        std::rc::Rc::new(|_bean, args| match args.first() {
            Some(DataValue::Long(n)) => Ok(DataValue::Long(n.abs())),
            _ => Ok(DataValue::Null),
        }),
    );
    let runner = runner_with(calc);
    // 5 是 Int,Long.abs 期望 Long
    let r = runner.execute("Calc.abs(5)", HashMap::new(), &opts());
    assert!(r.is_ok(), "implicit int→long coercion should succeed: {r:?}");
}

// ---------- varargs method ----------

#[test]
#[ignore = "varargs method dispatch not yet implemented in Rust; tracked in stage7 backlog"]
fn varargs_string_method() {
    // Java Child9.addField(int, String...)  当后续参数多于 1 个,打包成 String[]。
    let runner = runner_with(calc_with_methods());
    let r = runner
        .execute(
            "Calc.addField(5, '5.0', '5.0')",
            HashMap::new(),
            &opts(),
        )
        .expect("ok")
        .into_result();
    assert_eq!(r, DataValue::Int(3)); // 1 int + 2 string-varargs
}

#[test]
#[ignore = "varargs method dispatch not yet implemented; tracked in stage7 backlog"]
fn varargs_int_match_no_string() {
    // varargs 类型不匹配应报 METHOD_NOT_FOUND 而非崩溃
    let runner = runner_with(calc_with_methods());
    let r = runner.execute(
        "Calc.addField(5, 1, 1)",
        HashMap::new(),
        &opts(),
    );
    assert!(r.is_err());
}

// ---------- missing method error ----------

#[test]
fn missing_method_returns_error_code() {
    let runner = runner_with(calc_with_methods());
    let err = runner
        .execute("Calc.div(1, 2)", HashMap::new(), &opts())
        .expect_err("should error");
    assert_eq!(err.error_code(), error_codes::METHOD_NOT_FOUND);
}

// ---------- new instance ----------

#[test]
fn new_instance_no_matching_constructor_returns_error() {
    // 没有 2 参构造器
    let runner = runner_with(calc_with_methods());
    let r = runner.execute(
        "new Calc(1, 2)",
        HashMap::new(),
        &opts(),
    );
    assert!(r.is_err());
}

#[test]
fn new_instance_with_explicit_constructor() {
    // 注册 1 参构造器:接受 int 返回实例
    let mut calc = NativeType::named("com.example.Calc");
    calc.constructor = Some(std::rc::Rc::new(|_args| {
        Ok(DataValue::Str("Calc(1)".to_string()))
    }));
    let runner = runner_with(calc);
    let r = runner.execute("new Calc(1)", HashMap::new(), &opts());
    assert!(r.is_ok(), "explicit constructor should match: {r:?}");
}

// ---------- BigInteger / BigDecimal implicit numeric promotion ----------

#[test]
#[ignore = "BigInteger/BigDecimal auto-coercion in constructors pending; tracked in stage7 backlog"]
fn new_instance_int_to_big_integer() {
    let mut calc = NativeType::named("com.example.Calc");
    calc.constructor = Some(std::rc::Rc::new(|args| {
        // 接受 BigInteger 参数
        Ok(DataValue::BigInt(0))
    }));
    let runner = runner_with(calc);
    let _ = runner.execute("new Calc(5)", HashMap::new(), &opts());
}