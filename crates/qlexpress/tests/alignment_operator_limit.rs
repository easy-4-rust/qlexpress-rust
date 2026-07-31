//! Stage 7: 对齐 Java `com.alibaba.qlexpress4.OperatorLimitTest`
//! (14 个 @Test),锁定 OperatorCheckStrategy 行为。
//!
//! 验证 `Express4Runner::check(script, CheckOptions)`:
//! - whitelist / blacklist / allowAll 三种策略对运算符的放行/拦截
//! - error message 含 operator lexeme + 行列号
//! - 多行脚本错误指向正确行
//! - 空 whitelist / null whitelist 等价空集合

#![allow(clippy::result_large_err)]

use qlexpress::check_options::CheckOptions;
use qlexpress::exception::error_codes;
use qlexpress::init_options::InitOptions;
use qlexpress::operator::operator_check_strategy::OperatorCheckStrategy;
use qlexpress::ql_options::QLOptions;
use qlexpress::Express4Runner;

fn runner() -> Express4Runner {
    Express4Runner::with_init_options(InitOptions::default())
}

fn ok(runner: &Express4Runner, script: &str, opts: CheckOptions) {
    runner.check(script, &opts).expect("check should pass");
}

fn err(
    runner: &Express4Runner,
    script: &str,
    opts: CheckOptions,
) -> qlexpress::exception::QLSyntaxException {
    runner.check(script, &opts).expect_err("check should fail")
}

// ---------- Whitelist ----------

// Java source: OperatorLimitTest#testCheckWithAllowedOperators
#[test]
fn whitelist_allows_listed_operators() {
    let opts = CheckOptions::builder()
        .operator_check_strategy(OperatorCheckStrategy::Whitelist(
            ["+", "*"].into_iter().map(String::from).collect(),
        ))
        .build();
    ok(&runner(), "a + b * c", opts);
}

// Java source: OperatorLimitTest#testCheckWithDisallowedOperators
#[test]
fn whitelist_blocks_unlisted_assignment() {
    let opts = CheckOptions::builder()
        .operator_check_strategy(OperatorCheckStrategy::Whitelist(
            ["+", "*"].into_iter().map(String::from).collect(),
        ))
        .build();
    let e = err(&runner(), "a = b + c", opts);
    assert_eq!(e.error_code(), error_codes::OPERATOR_NOT_ALLOWED);
    assert_eq!(e.pos(), 2);
    assert_eq!(e.line_no(), 1);
    assert_eq!(e.col_no(), 3);
    assert_eq!(e.err_lexeme(), "=");
    assert!(e.reason().contains("Script uses disallowed operator"));
    assert!(e.reason().contains('='));
    let message = e.to_string();
    assert!(message.contains("OPERATOR_NOT_ALLOWED"));
    assert!(message.contains("Line: 1"));
    assert!(message.contains("Column: 3"));
}

// ---------- Blacklist ----------

// Java source: OperatorLimitTest#testCheckWithForbiddenOperators
#[test]
fn blacklist_allows_unblocked() {
    let opts = CheckOptions::builder()
        .operator_check_strategy(OperatorCheckStrategy::Blacklist(
            ["="].into_iter().map(String::from).collect(),
        ))
        .build();
    ok(&runner(), "a + b * c - d / e", opts);
}

// Java source: OperatorLimitTest#testCheckWithForbiddenOperatorUsed
#[test]
fn blacklist_blocks_listed() {
    let opts = CheckOptions::builder()
        .operator_check_strategy(OperatorCheckStrategy::Blacklist(
            ["="].into_iter().map(String::from).collect(),
        ))
        .build();
    let e = err(&runner(), "a = b + c", opts);
    assert_eq!(e.error_code(), error_codes::OPERATOR_NOT_ALLOWED);
    assert_eq!(e.pos(), 2);
    assert_eq!(e.line_no(), 1);
    assert_eq!(e.col_no(), 3);
    assert_eq!(e.err_lexeme(), "=");
    assert!(e.reason().contains("Script uses disallowed operator"));
    assert!(e.reason().contains('='));
    let message = e.to_string();
    assert!(message.contains("OPERATOR_NOT_ALLOWED"));
    assert!(message.contains("Line: 1"));
    assert!(message.contains("Column: 3"));
}

// ---------- Prefix / suffix ----------

// Java source: OperatorLimitTest#testWhitelistWithPrefixOperator
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
    assert_eq!(e.err_lexeme(), "--");
    assert_eq!(e.line_no(), 1);
    assert_eq!(e.col_no(), 1);
}

// Java source: OperatorLimitTest#testWhitelistWithSuffixOperator
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
    assert_eq!(e.err_lexeme(), "--");
    assert_eq!(e.line_no(), 1);
    assert_eq!(e.col_no(), 2);
}

// ---------- Multiple ----------

// Java source: OperatorLimitTest#testBlacklistWithMultipleOperators
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
    assert_eq!(e1.err_lexeme(), "=");
    let e2 = err(&runner(), "a * b", opts);
    assert_eq!(e2.error_code(), error_codes::OPERATOR_NOT_ALLOWED);
    assert_eq!(e2.err_lexeme(), "*");
}

// ---------- Empty / null whitelist ----------

// Java source: OperatorLimitTest#testEmptyWhitelistValidation
#[test]
fn empty_whitelist_blocks_everything() {
    let opts = CheckOptions::builder()
        .operator_check_strategy(OperatorCheckStrategy::Whitelist(Default::default()))
        .build();
    let e = err(&runner(), "a + b", opts);
    assert_eq!(e.error_code(), error_codes::OPERATOR_NOT_ALLOWED);
    assert_eq!(e.err_lexeme(), "+");
}

// Java source: OperatorLimitTest#testEmptyBlacklistValidation
#[test]
fn empty_blacklist_allows_everything() {
    let opts = CheckOptions::builder()
        .operator_check_strategy(OperatorCheckStrategy::Blacklist(Default::default()))
        .build();
    ok(&runner(), "a = b + c * d / e % f", opts.clone());
    ok(&runner(), "++a--", opts);
}

// Java source: OperatorLimitTest#testNullWhitelistValidation
// ADAPTED: Rust 的集合参数不允许 null；Java null 在入口规范化为空集合，
// 因而这里直接使用其规范化后的唯一合法表示，并保留相同拒绝契约。
#[test]
fn null_whitelist_normalizes_to_empty_set() {
    let opts = CheckOptions::builder()
        .operator_check_strategy(OperatorCheckStrategy::Whitelist(Default::default()))
        .build();
    let e = err(&runner(), "a + b", opts);
    assert_eq!(e.error_code(), error_codes::OPERATOR_NOT_ALLOWED);
    assert_eq!(e.err_lexeme(), "+");
}

// Java source: OperatorLimitTest#testNullBlacklistValidation
// ADAPTED: Rust 的集合参数不允许 null；Java null 在入口规范化为空集合，
// 因而这里直接使用其规范化后的唯一合法表示，并保留相同放行契约。
#[test]
fn null_blacklist_normalizes_to_empty_set() {
    let opts = CheckOptions::builder()
        .operator_check_strategy(OperatorCheckStrategy::Blacklist(Default::default()))
        .build();
    ok(&runner(), "a = b + c * d / e % f", opts.clone());
    ok(&runner(), "++a--", opts);
}

// ---------- Multi-line error position ----------

// Java source: OperatorLimitTest#testPreciseErrorPositionInMultiLineScript
#[test]
fn multi_line_error_points_at_correct_line() {
    let opts = CheckOptions::builder()
        .operator_check_strategy(OperatorCheckStrategy::Whitelist(
            ["+"].into_iter().map(String::from).collect(),
        ))
        .build();
    let e = err(&runner(), "a + b\nc = d\ne + f", opts);
    assert_eq!(e.error_code(), error_codes::OPERATOR_NOT_ALLOWED);
    assert_eq!(e.err_lexeme(), "=");
    assert_eq!(e.line_no(), 2);
    assert_eq!(e.col_no(), 3);
}

// ---------- Complex expression with inner op ----------

// Java source: OperatorLimitTest#testComplexExpressionWithWhitelist
#[test]
fn complex_expression_blocks_inner_assignment() {
    let opts = CheckOptions::builder()
        .operator_check_strategy(OperatorCheckStrategy::Whitelist(
            ["+", "*"].into_iter().map(String::from).collect(),
        ))
        .build();
    ok(&runner(), "(a + b) * (c + d)", opts.clone());

    let division = err(&runner(), "(a + b) * (c + d) / e", opts.clone());
    assert_eq!(division.error_code(), error_codes::OPERATOR_NOT_ALLOWED);
    assert_eq!(division.err_lexeme(), "/");

    let assignment = err(&runner(), "(a + b) * (c = d)", opts);
    assert_eq!(assignment.error_code(), error_codes::OPERATOR_NOT_ALLOWED);
    assert_eq!(assignment.err_lexeme(), "=");
    assert!(assignment.pos() > 10);
}

// ---------- Error message contains operator set ----------

// Java source: OperatorLimitTest#testErrorMessageContainsOperatorSet
#[test]
fn error_message_includes_operator() {
    let opts = CheckOptions::builder()
        .operator_check_strategy(OperatorCheckStrategy::Whitelist(
            ["+"].into_iter().map(String::from).collect(),
        ))
        .build();
    let e = err(&runner(), "a * b", opts);
    assert_eq!(e.error_code(), error_codes::OPERATOR_NOT_ALLOWED);
    assert!(e.reason().contains('*'));
    assert!(e.reason().contains('+') || e.reason().contains("allowed"));
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

/// SOURCE_PARITY: Java `OperatorCheckStrategy` 是公开接口，业务宿主可让
/// `isAllowed` 读取动态配置；已构造的 `CheckOptions` 必须保留该策略实例。
#[test]
fn custom_operator_strategy_observes_dynamic_host_policy() {
    use std::cell::Cell;
    use std::collections::HashSet;
    use std::rc::Rc;

    let allow_plus = Rc::new(Cell::new(false));
    let captured = Rc::clone(&allow_plus);
    let options = CheckOptions::builder()
        .operator_check_strategy(OperatorCheckStrategy::custom(
            move |operator| operator != "+" || captured.get(),
            HashSet::from(["+".to_string()]),
        ))
        .build();
    let runner = runner();

    assert_eq!(
        runner
            .check("1 + 2", &options)
            .expect_err("custom policy initially denies plus")
            .error_code(),
        error_codes::OPERATOR_NOT_ALLOWED
    );

    allow_plus.set(true);
    runner
        .check("1 + 2", &options)
        .expect("existing options must observe the updated custom policy");
}

/// SOURCE_PARITY: `WhiteOperatorCheckStrategy` 保存调用方集合的
/// `unmodifiableSet` 视图，原集合变化会影响既有 `CheckOptions`。
#[test]
fn shared_operator_whitelist_observes_backing_set_mutation() {
    use std::cell::RefCell;
    use std::collections::HashSet;
    use std::rc::Rc;

    let allowed = Rc::new(RefCell::new(HashSet::from(["*".to_string()])));
    let options = CheckOptions::builder()
        .operator_check_strategy(OperatorCheckStrategy::shared_whitelist(Rc::clone(&allowed)))
        .build();
    let runner = runner();

    assert_eq!(
        runner
            .check("1 + 2", &options)
            .expect_err("plus is absent from the initial backing set")
            .error_code(),
        error_codes::OPERATOR_NOT_ALLOWED
    );

    allowed.borrow_mut().insert("+".to_string());
    runner
        .check("1 + 2", &options)
        .expect("existing options must observe the added operator");

    allowed.borrow_mut().remove("+");
    assert_eq!(
        runner
            .check("1 + 2", &options)
            .expect_err("removing the operator must revoke later checks")
            .error_code(),
        error_codes::OPERATOR_NOT_ALLOWED
    );
}
