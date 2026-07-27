//! 本地可重复灰度与自动回滚演练。

use std::collections::HashMap;

use qlexpress_rust::ql_options::QLOptions;
use qlexpress_rust::runtime::value::DataValue;
use qlexpress_rust::Express4Runner;

const STABLE_RULE: &str = "amount >= 1000 ? 'REVIEW' : 'PASS'";
const GOOD_CANDIDATE_RULE: &str = "1000 > amount ? 'PASS' : 'REVIEW'";
const BAD_CANDIDATE_RULE: &str = "amount >= 1000 ? 'PASS' : 'REVIEW'";

pub fn run() -> Result<(), String> {
    let good = exercise(GOOD_CANDIDATE_RULE)?;
    if good.rolled_back || good.mismatches != 0 {
        return Err("good candidate unexpectedly rolled back".to_string());
    }
    let bad = exercise(BAD_CANDIDATE_RULE)?;
    if !bad.rolled_back || bad.mismatches == 0 {
        return Err("bad candidate did not trigger rollback".to_string());
    }
    println!(
        "{{\"traffic\":1000,\"canary_percent\":10,\"good_candidate\":{{\"mismatches\":0,\"rolled_back\":false}},\"bad_candidate\":{{\"mismatches\":{},\"rolled_back\":true}},\"post_rollback_errors\":{}}}",
        bad.mismatches, bad.post_rollback_errors
    );
    Ok(())
}

struct DrillResult {
    mismatches: usize,
    rolled_back: bool,
    post_rollback_errors: usize,
}

fn exercise(candidate: &str) -> Result<DrillResult, String> {
    let runner = Express4Runner::new();
    let options = QLOptions::builder().cache(true).build();
    let mut mismatches = 0usize;
    let mut rolled_back = false;
    let mut post_rollback_errors = 0usize;
    for request in 0..1_000 {
        let amount = if request % 3 == 0 { 1_500 } else { 200 };
        let expected = execute(&runner, STABLE_RULE, amount, &options)?;
        let use_candidate = !rolled_back && request % 10 == 0;
        let actual = execute(
            &runner,
            if use_candidate {
                candidate
            } else {
                STABLE_RULE
            },
            amount,
            &options,
        )?;
        if actual != expected {
            mismatches += 1;
            // 首次灰度不一致立即撤回候选规则，余量请求全部走稳定规则。
            rolled_back = true;
        } else if rolled_back && actual != expected {
            post_rollback_errors += 1;
        }
    }
    Ok(DrillResult {
        mismatches,
        rolled_back,
        post_rollback_errors,
    })
}

fn execute(
    runner: &Express4Runner,
    script: &str,
    amount: i32,
    options: &QLOptions,
) -> Result<DataValue, String> {
    let mut context = HashMap::new();
    context.insert("amount".to_string(), DataValue::Int(amount));
    runner
        .execute(script, context, options)
        .map(|result| result.into_result())
        .map_err(|error| format!("canary execution: {error}"))
}
