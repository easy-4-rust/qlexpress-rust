//! “每工作线程独立 Runner”的并发模型验收。

use std::collections::HashMap;
use std::thread;

use qlexpress::ql_options::QLOptions;
use qlexpress::runtime::value::DataValue;
use qlexpress::Express4Runner;

/// 对应 Java: 无（Rust 原生适配）。

pub fn run(threads: usize, iterations: usize) -> Result<(), String> {
    if threads == 0 || iterations == 0 {
        return Err("threads and iterations must be greater than zero".to_string());
    }
    let handles = (0..threads)
        .map(|worker| {
            thread::spawn(move || -> Result<u64, String> {
                // Express4Runner 使用 Rc/RefCell，不跨线程共享；每个工作线程
                // 创建并复用自己的 runner 与编译缓存。
                let runner = Express4Runner::new();
                let options = QLOptions::builder().cache(true).build();
                let mut checksum = 0u64;
                for iteration in 0..iterations {
                    let value = (worker * iterations + iteration) as i64;
                    let mut context = HashMap::new();
                    context.insert("value".to_string(), DataValue::Long(value));
                    let result = runner
                        .execute(
                            "function square(x) { return x * x; }; square(value) + 7",
                            context,
                            &options,
                        )
                        .map_err(|error| {
                            format!("worker {worker}, iteration {iteration}: {error}")
                        })?
                        .into_result();
                    let DataValue::Long(result) = result else {
                        return Err(format!(
                            "worker {worker}, iteration {iteration}: unexpected {result:?}"
                        ));
                    };
                    let expected = value.wrapping_mul(value).wrapping_add(7);
                    if result != expected {
                        return Err(format!(
                            "worker {worker}, iteration {iteration}: {result} != {expected}"
                        ));
                    }
                    checksum = checksum.wrapping_add(result as u64);
                }
                Ok(checksum)
            })
        })
        .collect::<Vec<_>>();

    let mut checksum = 0u64;
    for handle in handles {
        checksum = checksum.wrapping_add(
            handle
                .join()
                .map_err(|_| "concurrency worker panicked".to_string())??,
        );
    }
    println!(
        "{{\"model\":\"runner-per-worker\",\"threads\":{threads},\"iterations_per_thread\":{iterations},\"executions\":{},\"checksum\":{checksum}}}",
        threads * iterations
    );
    Ok(())
}
