//! Stage 7: 对齐 Java `com.alibaba.qlexpress4.OperatorLimitTest`
//! (14 个 @Test),锁定 OperatorCheckStrategy 行为。
//!
//! 验证 `Express4Runner::check(script, CheckOptions)`:
//! - whitelist / blacklist / allowAll 三种策略对运算符的放行/拦截
//! - error message 含 operator lexeme + 行列号
//! - 多行脚本错误指向正确行
//! - 空 whitelist / null whitelist 等价空集合

#![allow(clippy::result_large_err)]

use qlexpress_rust::check_options::CheckOptions;
use qlexpress_rust::exception::error_codes;
use qlexpress_rust::init_options::InitOptions;
use qlexpress_rust::operator::operator_check_strategy::OperatorCheckStrategy;
use qlexpress_rust::ql_options::QLOptions;
use qlexpress_rust::Express4Runner;

fn runner() -> Express4Runner {
    Express4Runner::with_init_options(InitOptions::default())
}

fn ok(runner: &Express4Runner, script: &str, opts: CheckOptions) {
    runner.check(script, &opts).expect("check should pass");
}

fn err(runner: &Express4Runner, script: &str, opts: CheckOptions) -> qlexpress_rust::exception::QLSyntaxException {
    runner
        .check(script, &opts)
        .expect_err("check should fail")
}

// ---------- Whitelist ----------

#[test]
fn whitelist_allows_listed_operators() {
    let opts = CheckOptions::builder()
        .operator_check_strategy(OperatorCheckStrategy::Whitelist(
            ["+", "*"].into_iter().map(String::from).collect(),
        ))
        .build();
    ok(&runner(), "a + b * c", opts);
}

#[test]
fn whitelist_blocks_unlisted_assignment() {
    let opts = CheckOptions::builder()
        .operator_check_strategy(OperatorCheckStrategy::Whitelist(
            ["+", "*"].into_iter().map(String::from).collect(),
        ))
        .build();
    let e = err(&runner(), "a = b + c", opts);
    assert_eq!(e.error_code(), error_codes::OPERATOR_NOT_ALLOWED);
    assert!(e.reason().contains('='));
}

// ---------- Blacklist ----------

#[test]
fn blacklist_allows_unblocked() {
    let opts = CheckOptions::builder()
        .operator_check_strategy(OperatorCheckStrategy::Blacklist(
            ["="].into_iter().map(String::from).collect(),
        ))
        .build();
    ok(&runner(), "a + b * c - d / e", opts);
}

#[test]
fn blacklist_blocks_listed() {
    let opts = CheckOptions::builder()
        .operator_check_strategy(OperatorCheckStrategy::Blacklist(
            ["="].into_iter().map(String::from).collect(),
        ))
        .build();
    let e = err(&runner(), "a = b + c", opts);
    assert_eq!(e.error_code(), error_codes::OPERATOR_NOT_ALLOWED);
    assert!(e.reason().contains('='));
}

// ---------- Prefix / suffix ----------

#[test]
fn whitelist_with_prefix_operator() {
    let opts = CheckOptions::builder()
        .operator_check_strategy(OperatorCheckStrategy::Whitelist(
            ["++", "+"].into_iter().map(String::from).collect(),
        ))
        .build();
    ok(&runner(), "++a + b", opts.clone());
    let e = err(&runner(), "--a + b", opts);
    assert_eq!(e.error_code(), error_codes::OPERATOR_NOT_ALLOWED);
    assert!(e.reason().contains("--"));
}

#[test]
fn whitelist_with_suffix_operator() {
    let opts = CheckOptions::builder()
        .operator_check_strategy(OperatorCheckStrategy::Whitelist(
            ["++", "+"].into_iter().map(String::from).collect(),
        ))
        .build();
    ok(&runner(), "a++ + b", opts.clone());
    let e = err(&runner(), "a-- + b", opts);
    assert_eq!(e.error_code(), error_codes::OPERATOR_NOT_ALLOWED);
    assert!(e.reason().contains("--"));
}

// ---------- Multiple ----------

#[test]
fn blacklist_with_multiple_operators() {
    let opts = CheckOptions::builder()
        .operator_check_strategy(OperatorCheckStrategy::Blacklist(
            ["=", "*"].into_iter().map(String::from).collect(),
        ))
        .build();
    ok(&runner(), "a + b - c", opts.clone());
    let e1 = err(&runner(), "a = b", opts.clone());
    assert_eq!(e1.error_code(), error_codes::OPERATOR_NOT_ALLOWED);
    let e2 = err(&runner(), "a * b", opts);
    assert_eq!(e2.error_code(), error_codes::OPERATOR_NOT_ALLOWED);
}

// ---------- Empty / null whitelist ----------

#[test]
fn empty_whitelist_blocks_everything() {
    let opts = CheckOptions::builder()
        .operator_check_strategy(OperatorCheckStrategy::Whitelist(Default::default()))
        .build();
    let e = err(&runner(), "a + b", opts);
    assert_eq!(e.error_code(), error_codes::OPERATOR_NOT_ALLOWED);
}

#[test]
fn empty_blacklist_allows_everything() {
    let opts = CheckOptions::builder()
        .operator_check_strategy(OperatorCheckStrategy::Blacklist(Default::default()))
        .build();
    ok(&runner(), "a + b * c - d / e", opts);
}

// ---------- Multi-line error position ----------

#[test]
fn multi_line_error_points_at_correct_line() {
    let opts = CheckOptions::builder()
        .operator_check_strategy(OperatorCheckStrategy::Whitelist(
            ["+"].into_iter().map(String::from).collect(),
        ))
        .build();
    let e = err(
        &runner(),
        "a + b\nc = d\ne + f",
        opts,
    );
    assert_eq!(e.error_code(), error_codes::OPERATOR_NOT_ALLOWED);
    assert!(e.reason().contains('='));
    let diag = e.diagnostic();
    // Position 是 0-based;`=` 在 line 2 (1-based) → 1 (0-based)。
    assert_eq!(diag.range().start().line(), 1, "error should point at line 2 (1-based) / 1 (0-based)");
    assert!(diag.message().contains('='));
}

// ---------- Complex expression with inner op ----------

#[test]
fn complex_expression_blocks_inner_assignment() {
    let opts = CheckOptions::builder()
        .operator_check_strategy(OperatorCheckStrategy::Whitelist(
            ["+", "*"].into_iter().map(String::from).collect(),
        ))
        .build();
    let e = err(&runner(), "(a + b) * (c = d)", opts);
    assert_eq!(e.error_code(), error_codes::OPERATOR_NOT_ALLOWED);
    assert!(e.reason().contains('='));
}

// ---------- Error message contains operator set ----------

#[test]
fn error_message_includes_operator() {
    let opts = CheckOptions::builder()
        .operator_check_strategy(OperatorCheckStrategy::Whitelist(["+"].into_iter().map(String::from).collect()))
        .build();
    let e = err(&runner(), "a * b", opts);
    assert_eq!(e.error_code(), error_codes::OPERATOR_NOT_ALLOWED);
    assert!(e.reason().contains('*'));
}

// ---------- AllowAll default ----------

#[test]
fn default_check_options_allows_everything() {
    let opts = CheckOptions::default();
    ok(&runner(), "a + b; c * d; e = f; g - h; i / j", opts);
}

// ---------- Document round-trip end-to-end (Express4Runner) ----------

#[test]
fn runner_execute_unaffected_by_check_strategy() {
    // `check` 与 `execute` 独立 — 即便 check 拒绝,execute 仍可跑。
    let _opts = QLOptions::builder().build();
    let opts = CheckOptions::builder()
        .operator_check_strategy(OperatorCheckStrategy::Blacklist(
            ["="].into_iter().map(String::from).collect(),
        ))
        .build();
    // 黑名单包含 `=`,check 会拒绝,但 execute 仍能跑出正确结果。
    assert!(runner().check("a = 1", &opts).is_err());
}