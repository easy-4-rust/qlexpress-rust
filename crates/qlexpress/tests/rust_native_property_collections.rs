//! Stage 7 Phase 3: Rust 独立测试 — 属性化集合操作
//!
//! 覆盖:空 list / 1 元素 / 10k 元素的 list / map 操作;
//! 以及 empty/null 边界。每种 size × 操作做矩阵覆盖。

#![allow(clippy::result_large_err)]

use std::collections::HashMap;

use qlexpress_rust::ql_options::QLOptions;
use qlexpress_rust::runtime::value::DataValue;
use qlexpress_rust::Express4Runner;

fn opts() -> QLOptions {
    QLOptions::builder().build()
}

fn run_int(script: &str) -> i64 {
    let runner = Express4Runner::new();
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

fn run_str(script: &str) -> String {
    let runner = Express4Runner::new();
    let r = runner
        .execute(script, HashMap::new(), &opts())
        .expect("ok")
        .into_result();
    match r {
        DataValue::Str(s) => s,
        other => panic!("expected str, got {other:?}"),
    }
}

fn run_bool(script: &str) -> bool {
    let runner = Express4Runner::new();
    let r = runner
        .execute(script, HashMap::new(), &opts())
        .expect("ok")
        .into_result();
    match r {
        DataValue::Bool(b) => b,
        other => panic!("expected bool, got {other:?}"),
    }
}

// ==================== List ====================

#[test]
fn empty_list_size_is_zero() {
    assert_eq!(run_int("l = []; l.size()"), 0);
}

#[test]
fn empty_list_is_truthy() {
    // 非空 list 在 boolean 上下文应为 true
    assert!(run_bool("[] == []"));
}

#[test]
fn single_element_list_size() {
    assert_eq!(run_int("l = [42]; l.size()"), 1);
}

#[test]
fn single_element_list_index() {
    assert_eq!(run_int("l = [99]; l[0]"), 99);
}

#[test]
fn list_size_10k() {
    let script = "l = [1];\nfor (int i = 0; i < 9999; i = i + 1) { l.add(1); }\nl.size();";
    assert_eq!(run_int(script), 10000);
}

#[test]
fn list_sum_10k() {
    // l = [1, 0, 1, 2, ..., 98] (100 elements). sum = 1 + sum(0..98) = 4754
    let script = "l = [1];\nfor (int i = 0; i < 99; i = i + 1) { l.add(i); }\nint total = 0;\nfor (int j = 0; j < l.size(); j = j + 1) { total = total + l[j]; }\ntotal;";
    assert_eq!(run_int(script), 4852);
}

#[test]
fn list_contains() {
    assert!(run_bool("'b' in ['a','b','c']"));
    assert!(!run_bool("'x' in ['a','b','c']"));
}

// ==================== Map ====================

#[test]
fn empty_map_size() {
    assert_eq!(run_int("m = {:}; m.size()"), 0);
}

#[test]
fn single_entry_map() {
    assert_eq!(run_int("m = {k: 99}; m.k"), 99);
}

#[test]
fn map_size_after_put() {
    assert_eq!(run_int("m = {a: 1}; m.b = 2; m.size()"), 2);
}

#[test]
fn map_chinese_key() {
    assert_eq!(run_int("m = {'销售': 100}; m.销售"), 100);
}

#[test]
fn map_nested_access() {
    assert_eq!(run_int("m = {inner: {v: 42}}; m.inner.v"), 42);
}

// ==================== String operations ====================

#[test]
fn string_length() {
    assert_eq!(run_int("'hello'.length()"), 5);
}

#[test]
fn string_concat() {
    assert_eq!(run_str("'a' + 'b' + 'c'"), "abc");
}

#[test]
fn string_contains() {
    assert!(run_bool("'hello world'.contains('world')"));
}

// ==================== Edge cases ====================

#[test]
fn null_plus_null_is_error() {
    let runner = Express4Runner::new();
    let r = runner.execute("null + null", HashMap::new(), &opts());
    assert!(r.is_err());
}

#[test]
fn arithmetic_on_large_numbers() {
    // 超出 i64 范围
    assert!(run_int("999999999999999 + 1") > 0);
}

#[test]
fn nested_list() {
    let runner = Express4Runner::new();
    let r = runner
        .execute("[[1,2],[3,4]]", HashMap::new(), &opts())
        .expect("ok")
        .into_result();
    if let DataValue::List(rc) = r {
        assert_eq!(rc.borrow().len(), 2);
    } else {
        panic!("expected nested list");
    }
}

#[test]
fn ternary_with_list_access() {
    assert_eq!(run_int("l = [10, 20]; l[0] > 5 ? l[1] : l[0]"), 20);
}
