//! Stage 7: 对齐 Java `test/issue/Issue427Test` (8 个 @Test)。
//!
//! 核心回归:空循环体(for/while/forEach) + 后续表达式不应被
//! RETURN 或 null 干扰。Java 端在 Issue427 修复后明确:
//! - 空 body 不应传播 RETURN 信号
//! - 空 body 不应让后续表达式被吞

#![allow(clippy::result_large_err)]

use std::collections::HashMap;

use qlexpress::ql_options::QLOptions;
use qlexpress::Express4Runner;

fn run_int(script: &str) -> i64 {
    let runner = Express4Runner::new();
    let r = runner
        .execute(script, HashMap::new(), &QLOptions::builder().build())
        .expect("ok")
        .into_result();
    match r {
        qlexpress::runtime::value::DataValue::Long(n) => n,
        qlexpress::runtime::value::DataValue::Int(n) => n as i64,
        other => panic!("expected int/long, got {other:?}"),
    }
}

#[test]
fn empty_for_loop_should_not_affect_subsequent_expression() {
    assert_eq!(run_int("for (int i = 0; i < 5; i = i + 1) {} 1;"), 1);
}

#[test]
fn empty_for_loop_condition_never_met_should_not_affect_subsequent_expression() {
    assert_eq!(
        run_int("for (int i = 0; i < 0; i = i + 1) {i = i + 1;} 1;"),
        1
    );
}

#[test]
fn empty_while_loop_should_not_affect_subsequent_expression() {
    assert_eq!(run_int("while (false) {} 1;"), 1);
}

#[test]
fn for_loop_with_explicit_return_should_return_correctly() {
    let script = "for (int i = 0; i < 5; i = i + 1) { return 42; } 1;";
    let runner = Express4Runner::new();
    let r = runner
        .execute(script, HashMap::new(), &QLOptions::builder().build())
        .expect("ok")
        .into_result();
    // return inside for:脚本应该立即返回 42
    assert_eq!(r, qlexpress::runtime::value::DataValue::Long(42));
}

#[test]
fn empty_for_each_loop_should_not_affect_subsequent_expression() {
    // Java: a = []; for(item : a){} 1; → 1
    let script = "a = [];\nfor (item : a) {}\n1;";
    assert_eq!(run_int(script), 1);
}

#[test]
fn empty_for_loop_with_semicolon_body_should_work() {
    // for(...) {;} 1;
    let script = "for (int i = 0; i < 5; i = i + 1) {;} 1;";
    assert_eq!(run_int(script), 1);
}

#[test]
fn non_empty_for_loop_should_still_work() {
    let script = "a = 0;\nfor (int i = 0; i < 5; i = i + 1) { a = a + i; }\na;";
    // 0+1+2+3+4 = 10
    assert_eq!(run_int(script), 10);
}

#[test]
fn empty_for_loop_multiple_statements_after() {
    // for(...) {} a = 10; b = 20; a + b; → 30
    let script = "for (int i = 0; i < 5; i = i + 1) {}\na = 10;\nb = 20;\na + b;";
    assert_eq!(run_int(script), 30);
}
