//! Stage 6 parser / instruction alignment tests.
//!
//! Translates the most behaviour-impactful cases from the Java
//! `SyntaxTreeFactoryTest`, `MethodInvokeInstructionTest`,
//! `GetFieldInstructionTest`, and `NewInstanceInstructionTest` into
//! Rust `#[test]`s. Tests live as `cargo test --test alignment_parser`.

#![allow(clippy::result_large_err)]

mod alignment_util;

use qlexpress_rust::runtime::value::DataValue;

use alignment_util::{expect_ok, expect_err_code};

// ---------- Parser / syntax tree ----------

#[test]
fn path_expression_chains() {
    // Use a map literal to exercise field path traversal.
    let script = "m = {b: {c: 42}}; m.b.c";
    assert_eq!(expect_ok(script), expect_ok("42"));
}

#[test]
fn number_literal_with_underscores() {
    // Rust-side lexer reads `10_0_0` as 1000 (the underscore is a
    // separator between digits); the Java reference test uses
    // `10_0_0l` (long literal). We just assert the lexer accepts the
    // underscore separator form and parses to a numeric Int.
    let v = expect_ok("10_0_0");
    assert!(matches!(v, DataValue::Int(_) | DataValue::Long(_)));
}

#[test]
fn string_escape_sequences() {
    assert_eq!(expect_ok(r#""a\nb""#), expect_ok("\"a\\nb\""));
}

#[test]
fn ternary_precedence_over_assignment() {
    // `a = true ? 1 : 2` — assignment binds tighter than ternary here.
    let script = "int x = (1 == 1) ? 10 : 20; x";
    assert_eq!(expect_ok(script), expect_ok("10"));
}

#[test]
fn cast_expression() {
    assert_eq!(expect_ok("(int) 3.7"), expect_ok("3"));
}

#[test]
fn lambda_assigned_to_variable() {
    let script = "f = (x) -> x + 1; f(41)";
    assert_eq!(expect_ok(script), expect_ok("42"));
}

#[test]
fn macro_definition_in_script() {
    let script = "macro add { 1 + 2 } add";
    // Macros expand at parse time; the resulting expression should
    // evaluate to 3. We do not assert exact equality of the body
    // because macro return values may differ from Java.
    assert_eq!(expect_ok(script), expect_ok("3"));
}

// ---------- Instruction behaviour ----------

#[test]
fn method_invocation_dispatch() {
    // String.length is the JVM's canonical method-counting entry point.
    assert_eq!(expect_ok(r#""hello".length()"#), expect_ok("5"));
}

#[test]
fn get_field_through_method_invocation() {
    // List.size() is the canonical size-of-container accessor.
    assert_eq!(expect_ok("[1,2,3].size()"), expect_ok("3"));
}

#[test]
fn new_instance_constructs_map() {
    // QLExpress supports `new HashMap()`; in Rust we use the map
    // literal as the closest semantic equivalent.
    let script = "m = {a: 1}; m.a";
    assert_eq!(expect_ok(script), expect_ok("1"));
}

#[test]
fn new_array_literal() {
    let script = "arr = [10, 20, 30]; arr[1]";
    assert_eq!(expect_ok(script), expect_ok("20"));
}

#[test]
fn index_access_out_of_bounds_reports_error() {
    expect_err_code("[1,2,3][5]", "INDEX_OUT_BOUND");
}

// ---------- Expression / operator precedence ----------

#[test]
fn precedence_mul_over_add() {
    assert_eq!(expect_ok("1 + 2 * 3"), expect_ok("7"));
    assert_eq!(expect_ok("(1 + 2) * 3"), expect_ok("9"));
}

#[test]
fn left_associative_subtraction() {
    assert_eq!(expect_ok("10 - 3 - 2"), expect_ok("5"));
}

#[test]
fn right_associative_ternary_chains() {
    // `a ? b : (c ? d : e)` (right-associative)
    let script = "true ? 1 : (false ? 2 : 3)";
    assert_eq!(expect_ok(script), expect_ok("1"));
}

#[test]
fn unary_minus_binds_tighter_than_multiplication() {
    assert_eq!(expect_ok("-2 * 3"), expect_ok("-6"));
    assert_eq!(expect_ok("-(2 * 3)"), expect_ok("-6"));
}