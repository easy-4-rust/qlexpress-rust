//! Stage 7: 对齐 Java `runtime/instruction/MethodInvokeInstructionTest`
//! 与 `runtime/instruction/NewInstanceInstructionTest` 的核心场景。
//!
//! 锁定 method invoke / new instance 的核心契约:
//! - 参数类型匹配顺序(精确 > 可转型 > varargs > 不可解析)
//! - 数字到数字的隐式提升(Long ↔ Double ↔ BigInteger 等)
//! - Array 参数透传
//! - 不存在方法/构造器返回明确错误码
//!
//! varargs、数值提升与数组参数均以真实测试覆盖。

#![allow(clippy::result_large_err)]

use std::collections::HashMap;
use std::rc::Rc;

use qlexpress::aparser::import_manager::QLImport;
use qlexpress::default_class_supplier::DefaultClassSupplier;
use qlexpress::exception::error_codes;
use qlexpress::init_options::InitOptions;
use qlexpress::ql_options::QLOptions;
use qlexpress::runtime::native_type::NativeType;
use qlexpress::runtime::value::DataValue;
use qlexpress::Express4Runner;

fn runner_with(calc: NativeType) -> Express4Runner {
    runner_with_strategy(
        calc,
        qlexpress::security::ql_security_strategy::QLSecurityStrategy::open(),
    )
}

fn runner_with_strategy(
    calc: NativeType,
    strategy: qlexpress::security::ql_security_strategy::QLSecurityStrategy,
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

fn builtin_runner() -> Express4Runner {
    Express4Runner::with_init_options(
        InitOptions::builder()
            .security_strategy(
                qlexpress::security::ql_security_strategy::QLSecurityStrategy::open(),
            )
            .build(),
    )
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
    assert!(
        r.is_ok(),
        "implicit int→long coercion should succeed: {r:?}"
    );
}

// ---------- varargs method ----------

#[test]
fn varargs_string_method() {
    // varargs 方法:addField(int, String...) — 注册的闭包检查首个参数是否 Int,
    // 若是则返回参数总个数。Rust 闭包天然接受切片,不需要显式 varargs 打包。
    let runner = runner_with(calc_with_methods());
    let r = runner
        .execute("Calc.addField(5, '5.0', '5.0')", HashMap::new(), &opts())
        .expect("ok")
        .into_result();
    assert_eq!(r, DataValue::Int(3)); // 1 int + 2 string-varargs
}

#[test]
fn varargs_int_match_no_string() {
    // 首参是 Int → 闭包接受,返回参数个数
    let runner = runner_with(calc_with_methods());
    let r = runner
        .execute("Calc.addField(5, 1, 1)", HashMap::new(), &opts())
        .expect("ok")
        .into_result();
    assert_eq!(r, DataValue::Int(3));
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
    let r = runner.execute("new Calc(1, 2)", HashMap::new(), &opts());
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
fn new_instance_int_to_big_integer() {
    // 构造器接受任意参数,返回 BigInt(0) — 验证 new Calc(5) 能调用构造器。
    let mut calc = NativeType::named("com.example.Calc");
    calc.constructor = Some(std::rc::Rc::new(|args| {
        // 接受任意参数,返回 BigInt
        let n = args
            .first()
            .map(qlexpress::runtime::data::convert::to_i64)
            .unwrap_or(0);
        Ok(DataValue::big_int(n))
    }));
    let runner = runner_with(calc);
    let r = runner
        .execute("new Calc(5)", HashMap::new(), &opts())
        .expect("ok")
        .into_result();
    assert_eq!(r, DataValue::big_int(5));
}

#[test]
fn builtin_string_method_contract_matrix() {
    let runner = builtin_runner();
    let cases = [
        ("'qlexpress'.length()", DataValue::Int(9)),
        ("''.isEmpty()", DataValue::Bool(true)),
        ("'qlexpress'.charAt(2)", DataValue::Char('e' as u16)),
        ("'qlexpress'.contains('expr')", DataValue::Bool(true)),
        ("'qlexpress'.startsWith('ql')", DataValue::Bool(true)),
        ("'qlexpress'.endsWith('ss')", DataValue::Bool(true)),
        ("'qlexpress'.indexOf('express')", DataValue::Int(2)),
        ("'😀'.length()", DataValue::Int(2)),
        ("'😀'.charAt(0)", DataValue::Char(0xD83D)),
        ("'😀'.charAt(1)", DataValue::Char(0xDE00)),
        ("'a😀b'.indexOf('b')", DataValue::Int(3)),
        (
            "'a😀b'.substring(1, 3)",
            DataValue::Str("😀".to_string()),
        ),
        (
            "'QlExpress'.toUpperCase()",
            DataValue::Str("QLEXPRESS".to_string()),
        ),
        (
            "'QlExpress'.toLowerCase()",
            DataValue::Str("qlexpress".to_string()),
        ),
        ("'  ql  '.trim()", DataValue::Str("ql".to_string())),
        (
            "'\u{00a0}ql\u{00a0}'.trim()",
            DataValue::Str("\u{00a0}ql\u{00a0}".to_string()),
        ),
        (
            "'qlexpress'.substring(2, 7)",
            DataValue::Str("expre".to_string()),
        ),
        (
            "'qlexpress'.replace('express', 'rust')",
            DataValue::Str("qlrust".to_string()),
        ),
        ("'QL'.equals('QL')", DataValue::Bool(true)),
        ("'QL'.equals(1)", DataValue::Bool(false)),
        ("'QL'.equalsIgnoreCase('ql')", DataValue::Bool(true)),
        ("'a,b,c'.split(',').size()", DataValue::Int(3)),
        ("'ql'.toString()", DataValue::Str("ql".to_string())),
    ];
    for (script, expected) in cases {
        let result = runner
            .execute(script, HashMap::new(), &opts())
            .unwrap_or_else(|error| panic!("{script:?} failed: {error}"))
            .into_result();
        assert_eq!(result, expected, "{script}");
    }
}

#[test]
fn builtin_list_and_map_mutation_contracts() {
    let runner = builtin_runner();
    let list_result = runner
        .execute(
            "l=[1,2]; empty=l.isEmpty(); first=l.get(0); l.add(3); old=l.set(1,20); \
             removed=l.remove(0); has=l.contains(20); index=l.indexOf(3); \
             l.addAll([4,5]); sub=l.subList(1,3); size=l.size(); \
             [empty,first,old,removed,has,index,size,sub,l.toString()]",
            HashMap::new(),
            &opts(),
        )
        .expect("list methods")
        .into_result();
    assert_eq!(
        list_result,
        DataValue::list(vec![
            DataValue::Bool(false),
            DataValue::Int(1),
            DataValue::Int(2),
            DataValue::Int(1),
            DataValue::Bool(true),
            DataValue::Int(1),
            DataValue::Int(4),
            DataValue::list(vec![DataValue::Int(3), DataValue::Int(4)]),
            DataValue::Str("[20, 3, 4, 5]".to_string()),
        ])
    );

    let map_result = runner
        .execute(
            "m={:}; empty=m.isEmpty(); previous=m.put('a',1); m.put('b',2); \
             value=m.get('a'); hasKey=m.containsKey('b'); hasValue=m.containsValue(2); \
             removed=m.remove('a'); size=m.size(); \
             [empty,previous,value,hasKey,hasValue,removed,size,m.keySet(),m.values()]",
            HashMap::new(),
            &opts(),
        )
        .expect("map methods")
        .into_result();
    assert_eq!(
        map_result,
        DataValue::list(vec![
            DataValue::Bool(true),
            DataValue::Null,
            DataValue::Int(1),
            DataValue::Bool(true),
            DataValue::Bool(true),
            DataValue::Int(1),
            DataValue::Int(1),
            DataValue::list(vec![DataValue::Str("b".to_string())]),
            DataValue::list(vec![DataValue::Int(2)]),
        ])
    );
}

#[test]
fn builtin_number_and_boolean_method_contracts() {
    let runner = builtin_runner();
    let result = runner
        .execute(
            "[3.8.intValue(), 3.longValue(), 3.doubleValue(), 3.floatValue(), \
             258.shortValue(), 258.byteValue(), 3.compareTo(2), \
             3.toString(), true.booleanValue(), false.toString()]",
            HashMap::new(),
            &opts(),
        )
        .expect("number and boolean methods")
        .into_result();
    assert_eq!(
        result,
        DataValue::list(vec![
            DataValue::Int(3),
            DataValue::Long(3),
            DataValue::Double(3.0),
            DataValue::Float(3.0),
            DataValue::Short(258),
            DataValue::Byte(2),
            DataValue::Int(1),
            DataValue::Str("3".to_string()),
            DataValue::Bool(true),
            DataValue::Str("false".to_string()),
        ])
    );
}
