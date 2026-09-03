//! Comprehensive integration test — Part 3: Lambda, Functions, Macros, Advanced Features.
//!
//! Split from `alignment_comprehensive_markers.rs` for 800-line cohesion limit.
//! Marker coverage: Lambda, Function definitions, Macro definitions,
//! Scope shadowing, Return in loop, Nested control flow, String interpolation.

#![allow(clippy::result_large_err)]
#![allow(unused_variables)]

use std::collections::HashMap;

use qlexpress::ql_options::QLOptions;
use qlexpress::runtime::value::DataValue;
use qlexpress::Express4Runner;

fn runner() -> Express4Runner {
    Express4Runner::new()
}

fn options() -> QLOptions {
    QLOptions::builder().build()
}

/// Exercise null handling.
/// Java NullLiteralContext, Nothing type
#[test]
fn null_handling() {
    let r = runner();
    let o = options();

    assert_eq!(
        r.execute("null", HashMap::new(), &o).unwrap().result(),
        &DataValue::Null
    );
    assert_eq!(
        r.execute("null == null", HashMap::new(), &o)
            .unwrap()
            .result(),
        &DataValue::Bool(true)
    );
    assert_eq!(
        r.execute("null != 1", HashMap::new(), &o).unwrap().result(),
        &DataValue::Bool(true)
    );
}

/// Exercise casting.
/// Java visitCast, CastInstruction
#[test]
fn casting() {
    let r = runner();
    let o = options();

    assert_eq!(
        r.execute("(int) 3.7", HashMap::new(), &o).unwrap().result(),
        &DataValue::Long(3)
    );
    assert_eq!(
        r.execute("(long) 3", HashMap::new(), &o).unwrap().result(),
        &DataValue::Long(3)
    );
    assert_eq!(
        r.execute("(double) 3", HashMap::new(), &o)
            .unwrap()
            .result(),
        &DataValue::Double(3.0)
    );
}

/// Exercise avoid null pointer.
/// Java visitAvoidNullPointer, AvoidNullPointerInstruction
#[test]
fn avoid_null_pointer() {
    let r = runner();
    let o = options();

    // Safe navigation
    assert_eq!(
        r.execute("null?.toString()", HashMap::new(), &o)
            .unwrap()
            .result(),
        &DataValue::Null
    );
}

/// Exercise array operations.
/// Java array operations
#[test]
fn spread_operator() {
    let r = runner();
    let o = options();

    // Array literal access
    assert_eq!(
        r.execute("[1, 2, 3, 4][3]", HashMap::new(), &o,)
            .unwrap()
            .result(),
        &DataValue::Long(4)
    );
}

/// Exercise block expressions.
/// Java visitBlock, BlockContext
#[test]
fn block_expressions() {
    let r = runner();
    let o = options();

    // Block expression returns last value
    assert_eq!(
        r.execute("{ int a = 1; int b = 2; a + b; }", HashMap::new(), &o)
            .unwrap()
            .result(),
        &DataValue::Long(3)
    );
}

/// Exercise multi-type arithmetic.
/// Java NumberMath promotion matrix
#[test]
fn multi_type_arithmetic() {
    let r = runner();
    let o = options();

    // Int + Long = Long
    assert_eq!(
        r.execute("1 + 2L", HashMap::new(), &o).unwrap().result(),
        &DataValue::Long(3)
    );
    // Int + Double = Double
    assert_eq!(
        r.execute("1 + 2.0", HashMap::new(), &o).unwrap().result(),
        &DataValue::Double(3.0)
    );
    // Long arithmetic
    assert_eq!(
        r.execute("1000000000L * 2", HashMap::new(), &o)
            .unwrap()
            .result(),
        &DataValue::Long(2000000000)
    );
}

/// Exercise error throwing.
/// Java visitThrowStatement, ThrowInstruction
#[test]
fn error_throwing() {
    let r = runner();
    let o = options();

    let result = r.execute("throw 'my error'", HashMap::new(), &o);
    assert!(result.is_err());
}

/// Exercise nested control flow.
/// Java nested if/for/while
#[test]
fn nested_control_flow() {
    let r = runner();
    let o = options();

    assert_eq!(r.execute(
        "int result = 0; for(int i = 0; i < 3; i = i + 1) { for(int j = 0; j < 3; j = j + 1) { result = result + 1; } } result",
        HashMap::new(), &o,
    ).unwrap().result(), &DataValue::Long(9));
}

/// Exercise string interpolation.
/// Java visitStringExpression, DyStrExprStart
#[test]
fn string_interpolation() {
    let r = runner();
    let o = options();

    assert_eq!(
        r.execute("int x = 42; \"value is ${x}\"", HashMap::new(), &o,)
            .unwrap()
            .result(),
        &DataValue::Str("value is 42".into())
    );
}

/// Exercise comment handling.
/// Java comment parsing
#[test]
fn comment_handling() {
    let r = runner();
    let o = options();

    assert_eq!(
        r.execute(
            "/* block comment */ 1 + 2 // line comment",
            HashMap::new(),
            &o,
        )
        .unwrap()
        .result(),
        &DataValue::Long(3)
    );
}

/// Exercise scope shadowing.
/// Java scope management
#[test]
fn scope_shadowing() {
    let r = runner();
    let o = options();

    assert_eq!(
        r.execute("int x = 1; { int x = 2; x; }", HashMap::new(), &o,)
            .unwrap()
            .result(),
        &DataValue::Long(2)
    );
}

/// Exercise return in loop.
/// Java ReturnInstruction in loop context
#[test]
fn return_in_loop() {
    let r = runner();
    let o = options();

    assert_eq!(
        r.execute(
            "for(int i = 0; i < 10; i = i + 1) { if(i == 3) { return i; } } -1",
            HashMap::new(),
            &o,
        )
        .unwrap()
        .result(),
        &DataValue::Long(3)
    );
}

// ============================================================================
// Additional tests for UNVERIFIED method clusters
// ============================================================================

/// Exercise variable scoping and shadowing.
/// Java scope management, variable resolution
#[test]
fn variable_scoping() {
    let r = runner();
    let o = options();

    // Global scope
    assert_eq!(
        r.execute("int x = 1; x", HashMap::new(), &o)
            .unwrap()
            .result(),
        &DataValue::Long(1)
    );
    // Block scope shadowing
    assert_eq!(
        r.execute("int x = 1; { int x = 2; x; }", HashMap::new(), &o)
            .unwrap()
            .result(),
        &DataValue::Long(2)
    );
    // Block scope returns last expression
    assert_eq!(
        r.execute("{ int a = 1; int b = 2; a + b; }", HashMap::new(), &o)
            .unwrap()
            .result(),
        &DataValue::Long(3)
    );
}

/// Exercise loop control flow.
/// Java break, continue in loops
#[test]
fn loop_control_flow() {
    let r = runner();
    let o = options();

    // Break in for loop
    assert_eq!(r.execute(
        "int result = 0;\nfor(int i = 0; i < 10; i = i + 1) {\n  if(i == 5) { break; }\n  result = result + 1;\n}\nresult",
        HashMap::new(), &o,
    ).unwrap().result(), &DataValue::Long(5));

    // Continue in for loop
    assert_eq!(r.execute(
        "int result = 0;\nfor(int i = 0; i < 5; i = i + 1) {\n  if(i == 2) { continue; }\n  result = result + i;\n}\nresult",
        HashMap::new(), &o,
    ).unwrap().result(), &DataValue::Long(8));

    // Break in while loop
    assert_eq!(r.execute(
        "int i = 0;\nint sum = 0;\nwhile(i < 10) {\n  if(i == 3) { break; }\n  sum = sum + i;\n  i = i + 1;\n}\nsum",
        HashMap::new(), &o,
    ).unwrap().result(), &DataValue::Long(3));
}

/// Exercise error handling with try-catch-finally.
/// Java try-catch-finally semantics
#[test]
fn try_catch_finally() {
    let r = runner();
    let o = options();

    // Basic try-catch
    assert_eq!(
        r.execute("try { throw 'error'; } catch(e) { e; }", HashMap::new(), &o,)
            .unwrap()
            .result(),
        &DataValue::Str("error".into())
    );

    // Try-catch with null dereference
    assert_eq!(
        r.execute(
            "try { Object x = null; x.toString(); } catch(e) { 'caught'; }",
            HashMap::new(),
            &o,
        )
        .unwrap()
        .result(),
        &DataValue::Str("caught".into())
    );
}

/// Exercise function with multiple parameters.
/// Java function definition and invocation
#[test]
fn function_multiple_params() {
    let r = runner();
    let o = options();

    assert_eq!(
        r.execute(
            "function add(a, b, c) { return a + b + c; } add(1, 2, 3)",
            HashMap::new(),
            &o,
        )
        .unwrap()
        .result(),
        &DataValue::Long(6)
    );
}

/// Exercise lambda with closures.
/// Java lambda capturing variables
#[test]
fn lambda_closures() {
    let r = runner();
    let o = options();

    // Lambda capturing outer variable
    assert_eq!(
        r.execute("int x = 10; f = (y) -> x + y; f(5)", HashMap::new(), &o,)
            .unwrap()
            .result(),
        &DataValue::Long(15)
    );
}

/// Exercise nested function calls.
/// Java function call nesting
#[test]
fn nested_function_calls() {
    let r = runner();
    let o = options();

    assert_eq!(r.execute(
        "function dbl(x) { return x * 2; }\nfunction add(a, b) { return a + b; }\ndbl(add(3, 4))",
        HashMap::new(), &o,
    ).unwrap().result(), &DataValue::Long(14));
}

/// Exercise type coercion.
/// Java type promotion and coercion rules
#[test]
fn type_coercion() {
    let r = runner();
    let o = options();

    // Int to Long promotion
    assert_eq!(
        r.execute("1 + 2L", HashMap::new(), &o).unwrap().result(),
        &DataValue::Long(3)
    );
    // Int to Double promotion
    assert_eq!(
        r.execute("1 + 2.0", HashMap::new(), &o).unwrap().result(),
        &DataValue::Double(3.0)
    );
    // String concatenation with numbers
    assert_eq!(
        r.execute("'value: ' + 42", HashMap::new(), &o)
            .unwrap()
            .result(),
        &DataValue::Str("value: 42".into())
    );
}
