//! 多线程负载与稳定性验收。

use std::collections::HashMap;
use std::sync::{Arc, Barrier};
use std::thread;
use std::time::{Duration, Instant};

use qlexpress::init_options::InitOptions;
use qlexpress::ql_options::QLOptions;
use qlexpress::runtime::value::DataValue;
use qlexpress::security::ql_security_strategy::QLSecurityStrategy;
use qlexpress::Express4Runner;

const SCRIPTS: &[&str] = &[
    "a * b + c",
    "sum = 0; for (i = 0; i < n; i++) { sum = sum + i; }; sum",
    "function fib(n) { if (n <= 1) { return n; }; return fib(n - 1) + fib(n - 2); }; fib(12)",
    "items = [1,2,3,4,5,6,7,8,9,10]; items.map(x -> x * 2).filter(x -> x > 10)",
    "m = {'tenant':'acme','score':score}; m.score >= 80 ? 'allow' : 'review'",
];

struct WorkerResult {
    latencies_micros: Vec<u64>,
    errors: usize,
}

pub fn run(duration: Duration, threads: usize) -> Result<(), String> {
    if duration.is_zero() || threads == 0 {
        return Err("duration and threads must be greater than zero".to_string());
    }
    let barrier = Arc::new(Barrier::new(threads + 1));
    let deadline = Instant::now() + duration;
    let handles = (0..threads)
        .map(|worker| {
            let barrier = Arc::clone(&barrier);
            thread::spawn(move || {
                let runner = Express4Runner::with_init_options(
                    InitOptions::builder()
                        .security_strategy(QLSecurityStrategy::open())
                        .build(),
                );
                let options = QLOptions::builder()
                    .cache(true)
                    .timeout_millis(1_000)
                    .build();
                let mut latencies_micros = Vec::new();
                let mut errors = 0usize;
                let mut sequence = 0usize;
                barrier.wait();
                while Instant::now() < deadline {
                    let script = SCRIPTS[(worker + sequence) % SCRIPTS.len()];
                    let mut context = HashMap::new();
                    context.insert("a".to_string(), DataValue::Long(17));
                    context.insert("b".to_string(), DataValue::Long(31));
                    context.insert("c".to_string(), DataValue::Long(5));
                    context.insert("n".to_string(), DataValue::Int(200));
                    context.insert("score".to_string(), DataValue::Int(88));
                    let started = Instant::now();
                    if runner.execute(script, context, &options).is_err() {
                        errors += 1;
                    }
                    latencies_micros.push(started.elapsed().as_micros() as u64);
                    sequence += 1;
                }
                WorkerResult {
                    latencies_micros,
                    errors,
                }
            })
        })
        .collect::<Vec<_>>();
    barrier.wait();

    let mut latencies = Vec::new();
    let mut errors = 0usize;
    for handle in handles {
        let result = handle
            .join()
            .map_err(|_| "load worker panicked".to_string())?;
        latencies.extend(result.latencies_micros);
        errors += result.errors;
    }
    if latencies.is_empty() {
        return Err("load test produced no samples".to_string());
    }
    latencies.sort_unstable();
    let executions = latencies.len();
    let percentile = |percent: usize| -> u64 {
        let index = ((executions - 1) * percent) / 100;
        latencies[index]
    };
    let throughput = executions as f64 / duration.as_secs_f64();
    let p95 = percentile(95);
    let p99 = percentile(99);
    println!(
        "{{\"threads\":{threads},\"duration_seconds\":{},\"executions\":{executions},\"errors\":{errors},\"throughput_ops_sec\":{throughput:.2},\"p50_micros\":{},\"p95_micros\":{p95},\"p99_micros\":{p99},\"max_micros\":{}}}",
        duration.as_secs(),
        percentile(50),
        latencies[executions - 1]
    );
    if errors != 0 {
        return Err(format!("load test observed {errors} execution errors"));
    }
    if throughput < 100.0 {
        return Err(format!(
            "load test throughput {throughput:.2} ops/s is below 100 ops/s"
        ));
    }
    if p99 > 250_000 {
        return Err(format!("load test p99 {p99}µs exceeds 250000µs"));
    }
    Ok(())
}
