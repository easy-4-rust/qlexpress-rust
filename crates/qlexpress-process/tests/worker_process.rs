//! 独立 Worker 的进程边界、JSON 协议和强制回收验收。

use std::time::Duration;

use qlexpress_process::{ProcessWorker, WorkerLimits, WorkerRequest};
use serde_json::{Map, Value};

fn worker_binary() -> &'static str {
    env!("CARGO_BIN_EXE_qlexpress-process")
}

// 受限进程契约是 Unix 专属：Windows 上 os_limits::apply 按设计拒绝
// （需要 Job Object 等外部沙箱提供硬限制），worker 返回 WORKER_ERROR。
#[cfg(unix)]
#[test]
fn executes_in_fresh_restricted_process() {
    let mut context = Map::new();
    context.insert("price".into(), Value::from(40));
    let request = WorkerRequest {
        script: "price + 2".into(),
        context,
        tenant_id: "tenant-a".into(),
        resource_limits: None,
    };
    // 该用例验证进程边界和协议，不验证生产默认延迟；LLVM coverage 插桩会显著
    // 放大一次性子进程的冷启动时间。超时强制回收由下一个 1ms 用例独立验收。
    let functional_limits = WorkerLimits {
        wall_timeout: Duration::from_secs(5),
        ..WorkerLimits::default()
    };
    let response = ProcessWorker::new(worker_binary(), functional_limits)
        .execute(&request)
        .unwrap();
    assert!(response.ok, "{response:?}");
    assert_eq!(response.result, Some(Value::from(42)));
}

#[test]
fn supervisor_kills_worker_at_wall_clock_deadline() {
    let request = WorkerRequest {
        script: "while (true) {}".into(),
        context: Map::new(),
        tenant_id: "hostile".into(),
        resource_limits: None,
    };
    let limits = WorkerLimits {
        wall_timeout: Duration::from_millis(1),
        ..WorkerLimits::default()
    };
    let response = ProcessWorker::new(worker_binary(), limits)
        .execute(&request)
        .unwrap();
    assert!(!response.ok);
    assert_eq!(response.error_code.as_deref(), Some("WORKER_WALL_TIMEOUT"));
}
