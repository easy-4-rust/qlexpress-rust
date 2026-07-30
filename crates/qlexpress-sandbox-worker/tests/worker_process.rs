//! 独立 Worker 的进程边界、JSON 协议和强制回收验收。

use std::time::Duration;

use qlexpress_sandbox_worker::{SandboxWorker, WorkerLimits, WorkerRequest};
use serde_json::{Map, Value};

fn worker_binary() -> &'static str {
    env!("CARGO_BIN_EXE_qlexpress-sandbox-worker")
}

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
    let response = SandboxWorker::new(worker_binary(), WorkerLimits::default())
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
    let response = SandboxWorker::new(worker_binary(), limits)
        .execute(&request)
        .unwrap();
    assert!(!response.ok);
    assert_eq!(response.error_code.as_deref(), Some("WORKER_WALL_TIMEOUT"));
}
