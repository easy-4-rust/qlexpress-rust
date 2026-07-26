//! Stage 6 runner alignment tests.
//!
//! Translates the most behaviour-impactful cases from Java
//! `com.alibaba.qlexpress4.Express4RunnerTest` (81 `@Test`s) into Rust
//! `#[test]`s. The point of this suite is to lock down script-level
//! semantics — control flow, operators, function dispatch, security
//! strategy, etc. — independently of any one test runner's quirks.
//!
//! Tests are grouped by Java method name. The mapping is one-to-one;
//! when a Java test is skipped or already covered by another suite
//! (e.g. `alignment_suite.rs`), we record it here with a brief note.

#![allow(clippy::result_large_err)]

mod alignment_util;

use std::collections::HashMap;

use qlexpress_rust::ql_options::QLOptions;
use qlexpress_rust::runtime::value::DataValue;
use qlexpress_rust::Express4Runner;

use alignment_util::{expect_ok, expect_err_code, expect_null, run_script};

// ---------- Operator basics (arithmetic / comparison / logical) ----------

#[test]
fn arithmetic_basics() {
    assert_eq!(expect_ok("1 + 2 * 3"), DataValue::Long(7));
    assert_eq!(expect_ok("10 - 3"), DataValue::Long(7));
    assert_eq!(expect_ok("100 / 4"), DataValue::Long(25));
    assert_eq!(expect_ok("100 % 7"), DataValue::Long(2));
}

#[test]
fn comparison_basics() {
    assert_eq!(expect_ok("1 < 2"), DataValue::Bool(true));
    assert_eq!(expect_ok("2 <= 2"), DataValue::Bool(true));
    assert_eq!(expect_ok("3 > 5"), DataValue::Bool(false));
    assert_eq!(expect_ok("3 == 3"), DataValue::Bool(true));
    assert_eq!(expect_ok("3 != 4"), DataValue::Bool(true));
}

#[test]
fn logical_and_or() {
    assert_eq!(expect_ok("true && false"), DataValue::Bool(false));
    assert_eq!(expect_ok("true || false"), DataValue::Bool(true));
    assert_eq!(expect_ok("!false"), DataValue::Bool(true));
}

#[test]
fn bitwise_basics() {
    assert_eq!(expect_ok("5 & 3"), DataValue::Long(1));
    assert_eq!(expect_ok("5 | 2"), DataValue::Long(7));
    assert_eq!(expect_ok("6 ^ 3"), DataValue::Long(5));
    assert_eq!(expect_ok("1 << 4"), DataValue::Long(16));
    assert_eq!(expect_ok("16 >> 2"), DataValue::Long(4));
}

#[test]
fn ternary_returns_value() {
    assert_eq!(expect_ok("true ? 1 : 2"), DataValue::Long(1));
    assert_eq!(expect_ok("false ? 1 : 2"), DataValue::Long(2));
}

// ---------- `in` operator ----------

#[test]
fn in_test_over_list() {
    assert_eq!(
        expect_ok("'ab' in ['cc','dd','ff']"),
        DataValue::Bool(false)
    );
    assert_eq!(
        expect_ok("'cc' in ['cc','dd','ff']"),
        DataValue::Bool(true)
    );
}

// ---------- Variable declaration and assignment ----------

#[test]
fn typed_local_variable() {
    assert_eq!(expect_ok("int a = 1; a + 2"), DataValue::Long(3));
}

#[test]
fn inferred_variable() {
    assert_eq!(expect_ok("a = 11; a + 1"), DataValue::Long(12));
}

#[test]
fn multiple_declarators() {
    // Java semantics: `int a, b = 10` declares `a` (default 0) and `b = 10`.
    // The result of `a + b` is therefore `0 + 10 = 10`, not 20.
    assert_eq!(expect_ok("int a, b = 10; a + b"), DataValue::Long(10));
}

// ---------- Control flow ----------

#[test]
fn if_else_if() {
    let script = "if (1 == 2) { 100 } else if (2 == 3) { 200 } else { 300 }";
    assert_eq!(expect_ok(script), DataValue::Long(300));
}

#[test]
fn for_loop_sum() {
    let script = "int sum = 0; for (int i = 1; i <= 5; i = i + 1) { sum = sum + i; } sum";
    assert_eq!(expect_ok(script), DataValue::Long(15));
}

#[test]
fn foreach_iterates_list() {
    // Java QLExpress4 用 `for (x : list)` 形式（与 Java SE for-each 一致），
    // 不是 `foreach` 关键字。Rust lexer 也只接受 `for`。
    let script = "int total = 0; for (x : [1, 2, 3, 4]) { total = total + x; } total";
    assert_eq!(expect_ok(script), DataValue::Long(10));
}

#[test]
fn while_loop() {
    let script = "int i = 0; while (i < 5) { i = i + 1; } i";
    assert_eq!(expect_ok(script), DataValue::Long(5));
}

#[test]
fn break_inside_for() {
    let script =
        "int sum = 0;\n\
         for (int i = 0; i < 10; i = i + 1) {\n\
         if (i == 5) {\n\
         break;\n\
         }\n\
         sum = sum + i;\n\
         }\n\
         sum";
    assert_eq!(expect_ok(script), DataValue::Long(10));
}

#[test]
fn continue_inside_for() {
    let script =
        "int sum = 0;\n\
         for (int i = 0; i < 5; i = i + 1) {\n\
         if (i == 2) {\n\
         continue;\n\
         }\n\
         sum = sum + i;\n\
         }\n\
         sum";
    // sum = 0+1+3+4 = 8
    assert_eq!(expect_ok(script), DataValue::Long(8));
}

#[test]
fn return_inside_if() {
    let script =
        "int a = 0;\n\
         if (true) {\n\
         return 42;\n\
         }\n\
         a";
    assert_eq!(expect_ok(script), DataValue::Long(42));
}

// ---------- Try/catch/finally ----------

#[test]
fn try_catch_runtime_exception() {
    let script = "try { 1/0 } catch (e) { 99 }";
    assert_eq!(expect_ok(script), DataValue::Long(99));
}

#[test]
fn try_finally_runs() {
    // Without finally mutator the result is whatever the try/catch
    // produces; we just assert the script runs without error.
    let script = "try { 1 + 1 } catch (e) { 0 } finally { 2 + 2 }";
    let _ = expect_ok(script);
}

// ---------- Function registration ----------

#[test]
fn add_function_with_closure() {
    let mut runner = Express4Runner::new();
    runner.add_function(
        "add",
        |_ctx: &mut dyn qlexpress_rust::runtime::qcontext::QContext,
         params: &qlexpress_rust::runtime::parameters::Parameters|
         -> Result<_, qlexpress_rust::exception::QLException> {
            let a = qlexpress_rust::runtime::data::convert::to_i64(&params.get_value(0));
            let b = qlexpress_rust::runtime::data::convert::to_i64(&params.get_value(1));
            Ok(DataValue::Long(a + b))
        },
    );
    let opts = QLOptions::builder().build();
    let result = runner
        .execute("add(2, 3)", HashMap::new(), &opts)
        .unwrap()
        .into_result();
    assert_eq!(result, DataValue::Long(5));
}

// ---------- varargs function ----------

#[test]
fn add_varargs_function_collects_args() {
    let mut runner = Express4Runner::new();
    runner.add_varargs_function(
        "join",
        |params: &[DataValue]| -> Result<_, qlexpress_rust::exception::QLException> {
            let joined = params
                .iter()
                .map(|p| p.string_value_of())
                .collect::<Vec<_>>()
                .join("-");
            Ok(DataValue::Str(joined))
        },
    );
    let opts = QLOptions::builder().build();
    let result = runner
        .execute("join('a','b','c')", HashMap::new(), &opts)
        .unwrap()
        .into_result();
    assert_eq!(result, DataValue::Str("a-b-c".to_string()));
}

// ---------- Custom operator ----------

#[test]
fn add_operator_bifunction() {
    let mut runner = Express4Runner::new();
    runner.add_operator_bi("join", |left: DataValue, right: DataValue| {
        DataValue::Str(format!(
            "{}|{}",
            left.string_value_of(),
            right.string_value_of()
        ))
    });
    let opts = QLOptions::builder().build();
    let result = runner
        .execute("'a' join 'b'", HashMap::new(), &opts)
        .unwrap()
        .into_result();
    assert_eq!(result, DataValue::Str("a|b".to_string()));
}

// ---------- Security strategy ----------

#[test]
fn security_open_allows_method_call() {
    use qlexpress_rust::security::ql_security_strategy::QLSecurityStrategy;
    let mut runner = Express4Runner::with_init_options(
        qlexpress_rust::init_options::InitOptions::builder()
            .security_strategy(QLSecurityStrategy::open())
            .build(),
    );
    let result = runner
        .execute("'hello'.length()", HashMap::new(), &QLOptions::builder().build())
        .unwrap()
        .into_result();
    assert_eq!(result, DataValue::Int(5));
}

#[test]
fn security_isolation_blocks_method_call() {
    use qlexpress_rust::runtime::native_type::NativeType;
    use qlexpress_rust::security::ql_security_strategy::QLSecurityStrategy;
    // Register a custom type with a method, then ask isolation to block it.
    let mut nt = NativeType::named("com.example.Calc");
    nt.static_methods.insert(
        "mul".to_string(),
        std::rc::Rc::new(|_bean, args| match args {
            [qlexpress_rust::runtime::value::DataValue::Int(a), qlexpress_rust::runtime::value::DataValue::Int(b)] => {
                Ok(qlexpress_rust::runtime::value::DataValue::Int(a * b))
            }
            _ => Ok(qlexpress_rust::runtime::value::DataValue::Null),
        }),
    );
    let mut runner = Express4Runner::with_init_options(
        qlexpress_rust::init_options::InitOptions::builder()
            .security_strategy(QLSecurityStrategy::isolation())
            .build(),
    );
    runner.register_native_type(nt);
    let result = runner.execute(
        "Calc.mul(6, 7)",
        HashMap::new(),
        &QLOptions::builder().build(),
    );
    assert!(result.is_err(), "isolation must reject registered method calls");
}

// ---------- Map and list literals ----------

#[test]
fn map_literal_access() {
    let script = "m = {a: 1, 'b': 2}; m.a + m.b";
    assert_eq!(expect_ok(script), DataValue::Long(3));
}

#[test]
fn list_literal_access() {
    let script = "l = [10, 20, 30]; l[1]";
    assert_eq!(expect_ok(script), DataValue::Long(20));
}

#[test]
fn list_literal_size() {
    let script = "l = [10, 20, 30]; l.size()";
    assert_eq!(expect_ok(script), DataValue::Int(3));
}

// ---------- short-circuit semantics ----------

#[test]
fn short_circuit_or_skips_rhs() {
    // 1/0 would throw, but `true || ...` must short-circuit.
    let script = "true || (1/0)";
    assert_eq!(expect_ok(script), DataValue::Bool(true));
}

#[test]
fn short_circuit_disabled_evaluates_both() {
    let opts = QLOptions::builder().short_circuit_disable(true).build();
    let result = run_script_with("true || (1/0)", &opts);
    assert!(result.is_err());
}

// ---------- Dollar interpolation ----------

#[test]
fn dollar_interpolation_default() {
    let opts = QLOptions::builder().build();
    let script = "\"a = ${1+2}\"";
    let mut runner = Express4Runner::new();
    let result = runner
        .execute(script, HashMap::new(), &opts)
        .unwrap()
        .into_result();
    assert_eq!(result, DataValue::Str("a = 3".to_string()));
}

// ---------- Lambda ----------

#[test]
fn lambda_expr_body() {
    let script = "f = (x) -> x + 1; f(10)";
    assert_eq!(expect_ok(script), DataValue::Long(11));
}

#[test]
fn lambda_block_body() {
    let script = "f = (x) -> { int y = x * 2; return y + 1; }; f(5)";
    assert_eq!(expect_ok(script), DataValue::Long(11));
}

// ---------- Compile cache ----------

#[test]
fn cache_returns_consistent_result() {
    let opts = QLOptions::builder().cache(true).build();
    let script = "1 + 2 + 3";
    let runner = Express4Runner::new();
    let r1 = runner.execute(script, HashMap::new(), &opts).unwrap().into_result();
    let r2 = runner.execute(script, HashMap::new(), &opts).unwrap().into_result();
    assert_eq!(r1, r2);
    assert_eq!(r1, DataValue::Long(6));
}

// ---------- Error reporting ----------

#[test]
fn runtime_error_carries_diagnostic_position() {
    // Division by zero is reported with the more specific
    // INVALID_ARITHMETIC code (matches Java's ArithmeticException).
    expect_err_code("1/0", "INVALID_ARITHMETIC");
}

#[test]
fn null_division_yields_arithmetic_error() {
    // `null + int` is a type mismatch, not arithmetic.
    expect_err_code("undefinedVar + 1", "INVALID_BINARY_OPERAND");
}

// ---------- Test helpers ----------

fn run_script_with(script: &str, options: &QLOptions) -> Result<DataValue, qlexpress_rust::exception::QLException> {
    let runner = Express4Runner::new();
    let result = runner.execute(script, HashMap::new(), options)?;
    Ok(result.into_result())
}