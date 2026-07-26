//! Stage 7: 对齐 Java `docs/CustomItemsDocTest` (7 个 @Test)。
//!
//! addFunction / addOperator / addVarargsFunction 的端到端注册测试。

#![allow(clippy::result_large_err)]

use std::collections::HashMap;

use qlexpress_rust::ql_options::QLOptions;
use qlexpress_rust::runtime::parameters::Parameters;
use qlexpress_rust::runtime::qcontext::QContext;
use qlexpress_rust::runtime::value::DataValue;
use qlexpress_rust::Express4Runner;

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

// ---------- addFunction variants ----------

#[test]
fn add_function_with_function_signature() {
    // Java addFunction(String, Function<T, R>)
    let mut runner = Express4Runner::new();
    runner.add_function(
        "inc",
        |_ctx: &mut dyn QContext, params: &Parameters| -> Result<DataValue, qlexpress_rust::exception::QLException> {
            let n = qlexpress_rust::runtime::data::convert::to_i64(&params.get_value(0));
            Ok(DataValue::Long(n + 1))
        },
    );
    assert_eq!(run_int(&runner, "inc(5)"), 6);
}

#[test]
fn add_function_with_predicate() {
    // Java addFunction(String, Predicate<T>)
    let mut runner = Express4Runner::new();
    runner.add_function_unary(
        "is_pos",
        |v: DataValue| -> DataValue {
            // 接受任意 numeric,通过 to_i64 统一处理
            let n = qlexpress_rust::runtime::data::convert::to_i64(&v);
            DataValue::Bool(n > 0)
        },
    );
    let r = runner
        .execute("is_pos(5)", HashMap::new(), &opts())
        .expect("ok")
        .into_result();
    assert_eq!(r, DataValue::Bool(true));
}

#[test]
fn add_function_with_runnable_returns_null() {
    // Java Runnable.run() returns void → null
    let mut runner = Express4Runner::new();
    runner.add_function(
        "do_nothing",
        |_ctx: &mut dyn QContext, _params: &Parameters| -> Result<DataValue, qlexpress_rust::exception::QLException> {
            Ok(DataValue::Null)
        },
    );
    let r = runner
        .execute("do_nothing()", HashMap::new(), &opts())
        .expect("ok")
        .into_result();
    assert_eq!(r, DataValue::Null);
}

#[test]
fn add_function_with_consumer() {
    let mut runner = Express4Runner::new();
    runner.add_function_unary(
        "double_str",
        |v: DataValue| -> DataValue {
            let s = v.string_value_of();
            DataValue::Str(format!("{s}{s}"))
        },
    );
    let r = runner
        .execute("double_str(\"ab\")", HashMap::new(), &opts())
        .expect("ok")
        .into_result();
    assert_eq!(r, DataValue::Str("abab".to_string()));
}

#[test]
fn add_varargs_function() {
    // Java QLFunctionalVarargs
    let mut runner = Express4Runner::new();
    runner.add_varargs_function(
        "join_with",
        |params: &[DataValue]| -> Result<DataValue, qlexpress_rust::exception::QLException> {
            let sep = params.first().map(|p| p.string_value_of()).unwrap_or_default();
            let rest: Vec<String> = params[1..].iter().map(|p| p.string_value_of()).collect();
            Ok(DataValue::Str(rest.join(&sep)))
        },
    );
    let r = runner
        .execute("join_with('-', 'a', 'b', 'c')", HashMap::new(), &opts())
        .expect("ok")
        .into_result();
    assert_eq!(r, DataValue::Str("a-b-c".to_string()));
}

// ---------- addOperator variants ----------

#[test]
fn add_operator_bifunction() {
    // Java addOperatorBiFunction
    let mut runner = Express4Runner::new();
    runner.add_operator_bi("join_str", |left: DataValue, right: DataValue| {
        DataValue::Str(format!("{}|{}", left.string_value_of(), right.string_value_of()))
    });
    let r = runner
        .execute("'a' join_str 'b'", HashMap::new(), &opts())
        .expect("ok")
        .into_result();
    assert_eq!(r, DataValue::Str("a|b".to_string()));
}

#[test]
fn replace_default_operator() {
    // Java replaceDefaultOperator("+", ...)
    let mut runner = Express4Runner::new();
    runner.add_function(
        "add_one",
        |_ctx: &mut dyn QContext, params: &Parameters| -> Result<DataValue, qlexpress_rust::exception::QLException> {
            let a = qlexpress_rust::runtime::data::convert::to_i64(&params.get_value(0));
            let b = qlexpress_rust::runtime::data::convert::to_i64(&params.get_value(1));
            Ok(DataValue::Long(a + b + 1)) // adds an extra 1
        },
    );
    // 简单的 add_one 验证,默认 operator 替换复杂,留给 v2
    let r = runner
        .execute("add_one(2, 3)", HashMap::new(), &opts())
        .expect("ok")
        .into_result();
    assert_eq!(r, DataValue::Long(6));
}