//! Stage 7: 对齐 Java `runtime/instruction/NewInstanceInstructionTest` (10 个 @Test)。
//!
//! 关键场景:构造器匹配、原始类型自动转型、数组参数、varargs、BigInteger
//! 隐式提升。Rust 端 `NativeType::constructor` 是 `Fn(&[DataValue]) -> Result<DataValue, QLException>`,
//! 接收 `Rc<dyn Fn>` — 简化为烟雾测试 + 主要场景覆盖。

#![allow(clippy::result_large_err)]

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use qlexpress_rust::aparser::import_manager::QLImport;
use qlexpress_rust::default_class_supplier::DefaultClassSupplier;
use qlexpress_rust::init_options::InitOptions;
use qlexpress_rust::ql_options::QLOptions;
use qlexpress_rust::runtime::native_type::NativeType;
use qlexpress_rust::runtime::value::DataValue;
use qlexpress_rust::security::ql_security_strategy::QLSecurityStrategy;
use qlexpress_rust::Express4Runner;

fn runner_with(calc: NativeType) -> Express4Runner {
    let mut supplier = DefaultClassSupplier::instance();
    supplier.register("com.example.Calc");
    let mut runner = Express4Runner::with_init_options(
        InitOptions::builder()
            .class_supplier(Rc::new(supplier))
            .add_default_import(vec![QLImport::import_cls("com.example.Calc")])
            .security_strategy(QLSecurityStrategy::open())
            .build(),
    );
    runner.register_native_type(calc);
    runner
}

fn opts() -> QLOptions {
    QLOptions::builder().build()
}

fn calc_with_constructor(
    ctor: impl Fn(&[DataValue]) -> Result<DataValue, qlexpress_rust::exception::QLException> + 'static,
) -> NativeType {
    let mut calc = NativeType::named("com.example.Calc");
    calc.constructor = Some(Rc::new(ctor));
    calc
}

#[test]
fn new_instance_with_1arg_constructor() {
    let runner = runner_with(calc_with_constructor(|args| {
        // 接受 1 个 int 参数,返回 "Calc(x)" 字符串
        match args.first() {
            Some(DataValue::Int(x)) => Ok(DataValue::Str(format!("Calc({x})"))),
            _ => Ok(DataValue::Null),
        }
    }));
    let r = runner
        .execute("new Calc(7)", HashMap::new(), &opts())
        .expect("ok")
        .into_result();
    assert_eq!(r, DataValue::Str("Calc(7)".to_string()));
}

#[test]
fn new_instance_with_2arg_constructor() {
    let runner = runner_with(calc_with_constructor(|args| {
        if args.len() != 2 {
            return Err(qlexpress_rust::exception::QLException::for_test(
                qlexpress_rust::exception::ql_exception::QLExceptionKind::Syntax,
                format!("expected 2 args, got {}", args.len()),
                qlexpress_rust::exception::error_codes::INVALID_NUMBER,
            ));
        }
        let a = qlexpress_rust::runtime::data::convert::to_i64(&args[0]);
        let b = qlexpress_rust::runtime::data::convert::to_i64(&args[1]);
        Ok(DataValue::Long(a + b))
    }));
    let r = runner
        .execute("new Calc(3, 4)", HashMap::new(), &opts())
        .expect("ok")
        .into_result();
    assert_eq!(r, DataValue::Long(7));
}

#[test]
fn new_instance_with_array_arg() {
    // 接受 list 参数
    let runner = runner_with(calc_with_constructor(|args| {
        if let Some(DataValue::List(_)) = args.first() {
            Ok(DataValue::Str("ok".to_string()))
        } else {
            Ok(DataValue::Str("not-list".to_string()))
        }
    }));
    let r = runner
        .execute("new Calc([1, 2, 3])", HashMap::new(), &opts())
        .expect("ok")
        .into_result();
    assert_eq!(r, DataValue::Str("ok".to_string()));
}

#[test]
fn new_instance_no_constructor_returns_error() {
    // Calc 没注册 constructor
    let mut calc = NativeType::named("com.example.Calc");
    // 不设 constructor
    let runner = runner_with(calc);
    let r = runner.execute("new Calc(1)", HashMap::new(), &opts());
    assert!(r.is_err());
}

#[test]
fn new_instance_with_string_arg() {
    let runner = runner_with(calc_with_constructor(|args| match args.first() {
        Some(DataValue::Str(s)) => Ok(DataValue::Str(format!("Calc({s})"))),
        _ => Ok(DataValue::Null),
    }));
    let r = runner
        .execute("new Calc(\"hello\")", HashMap::new(), &opts())
        .expect("ok")
        .into_result();
    assert_eq!(r, DataValue::Str("Calc(hello)".to_string()));
}

#[test]
fn new_instance_returns_data_value_for_use() {
    // 验证 new instance 的结果可以参与后续运算
    let runner = runner_with(calc_with_constructor(|args| {
        let x = qlexpress_rust::runtime::data::convert::to_i64(&args[0]);
        Ok(DataValue::Long(x * 2))
    }));
    let r = runner
        .execute("new Calc(5) + 3", HashMap::new(), &opts())
        .expect("ok")
        .into_result();
    assert_eq!(r, DataValue::Long(13));
}
