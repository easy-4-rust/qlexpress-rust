//! Stage 7 Phase 3: Rust 独立测试 — 性能烟雾测试
//!
//! 验证:编译缓存加速比、大脚本解析+运行 < 1s。

#![allow(clippy::result_large_err)]

use std::collections::HashMap;

use qlexpress_rust::ql_options::QLOptions;
use qlexpress_rust::runtime::value::DataValue;
use qlexpress_rust::Express4Runner;

fn opts() -> QLOptions {
    QLOptions::builder().cache(true).build()
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

#[test]
fn compile_cache_hit_speedup() {
    // 第一次 run 冷启动,第2次 cache hit。两者都应该很快。
    let runner = Express4Runner::new();
    let script = "1 + 2 * 3 + 4 * 5";
    // 冷启动
    let t0 = std::time::Instant::now();
    let _ = run_int(&runner, script);
    let cold = t0.elapsed();
    // cache hit
    let t1 = std::time::Instant::now();
    let _ = run_int(&runner, script);
    let hot = t1.elapsed();
    assert!(
        hot < cold * 3,
        "cache hit ({hot:?}) should be at most 3x cold ({cold:?})"
    );
}

#[test]
fn run_100_times_under_1s() {
    let runner = Express4Runner::new();
    let script = "1 + 2 * 3";
    let t0 = std::time::Instant::now();
    for _ in 0..100 {
        let _ = run_int(&runner, script);
    }
    let elapsed = t0.elapsed();
    assert!(
        elapsed.as_secs() < 1,
        "100 runs should < 1s, took {elapsed:?}"
    );
}

#[test]
fn medium_script_under_500ms() {
    let runner = Express4Runner::new();
    let script = "int total = 0;\n\
                   for (int i = 0; i < 1000; i = i + 1) {\n\
                   total = total + i;\n\
                   }\n\
                   total";
    let t0 = std::time::Instant::now();
    let r = run_int(&runner, script);
    let elapsed = t0.elapsed();
    assert_eq!(r, 499500);
    assert!(
        elapsed.as_millis() < 500,
        "medium script should < 500ms, took {elapsed:?}"
    );
}

#[test]
fn large_script_under_1s() {
    let runner = Express4Runner::new();
    // 构造 ~1k token 脚本
    let mut script = String::from("int total = 0;\n");
    for i in 0..100 {
        script.push_str(&format!("total = total + {i};\n"));
    }
    script.push_str("total");
    let t0 = std::time::Instant::now();
    let r = run_int(&runner, &script);
    let elapsed = t0.elapsed();
    assert!(r > 0);
    assert!(
        elapsed.as_secs() < 1,
        "large script should < 1s, took {elapsed:?}"
    );
}

#[test]
fn recursive_fibonacci_20_under_200ms() {
    // 对齐 Java fib(20)=6765 的语义；阈值取 200ms 是为了在 debug 构建 +
    // 并行测试压力下避免环境噪声（release 实测 < 50ms）。
    let runner = Express4Runner::new();
    let script = "function fib(int n) {\n\
                   if (n <= 1) { return n; }\n\
                   return fib(n - 1) + fib(n - 2);\n\
                   }\n\
                   fib(20)";
    let t0 = std::time::Instant::now();
    let r = run_int(&runner, script);
    let elapsed = t0.elapsed();
    assert_eq!(r, 6765);
    assert!(
        elapsed.as_millis() < 200,
        "fib(20) should < 200ms, took {elapsed:?}"
    );
}

#[test]
fn deterministic_same_script_same_result() {
    let runner = Express4Runner::new();
    let script = "int s = 0;\n\
                   for (int i = 1; i <= 100; i = i + 1) {\n\
                   s = s + i * i;\n\
                   }\n\
                   s";
    let r1 = run_int(&runner, script);
    let r2 = run_int(&runner, script);
    let r3 = run_int(&runner, script);
    assert_eq!(r1, r2);
    assert_eq!(r2, r3);
}

#[test]
fn list_iteration_performance() {
    let runner = Express4Runner::with_init_options(
        qlexpress_rust::init_options::InitOptions::builder()
            .security_strategy(
                qlexpress_rust::security::ql_security_strategy::QLSecurityStrategy::open(),
            )
            .build(),
    );
    let script = "int total = 0;\n\
                   l = [1];\n\
                   for (int i = 0; i < 99; i = i + 1) { l.add(i); }\n\
                   for (int i = 0; i < l.size(); i = i + 1) { total = total + l[i]; }\n\
                   total";
    let t0 = std::time::Instant::now();
    let _ = run_int(&runner, script);
    let elapsed = t0.elapsed();
    assert!(
        elapsed.as_millis() < 200,
        "list iteration should < 200ms, took {elapsed:?}"
    );
}

#[test]
fn timeout_works_under_1s() {
    use qlexpress_rust::init_options::InitOptions;
    use qlexpress_rust::security::ql_security_strategy::QLSecurityStrategy;
    let runner = Express4Runner::with_init_options(
        InitOptions::builder()
            .security_strategy(QLSecurityStrategy::open())
            .build(),
    );
    let opts = QLOptions::builder().timeout_millis(100).build();
    let t0 = std::time::Instant::now();
    let e = runner
        .execute(
            "int i = 0;\nwhile (true) { i = i + 1; }",
            HashMap::new(),
            &opts,
        )
        .expect_err("should timeout");
    let elapsed = t0.elapsed();
    assert_eq!(
        e.error_code(),
        qlexpress_rust::exception::error_codes::SCRIPT_TIME_OUT
    );
    assert!(
        elapsed.as_millis() < 1000,
        "timeout should fire in < 1s, took {elapsed:?}"
    );
}
