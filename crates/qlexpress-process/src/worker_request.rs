//! Worker 标准输入协议。

use qlexpress::ResourceLimits;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

/// 单次 Worker 执行请求。
///
/// Worker 每个进程只处理一个请求，避免租户间状态、缓存和 capability 泄漏。
/// 只接受 JSON 可表达的数据，不携带 Native 对象或宿主函数。
/// 对应 Java: 无（Rust 进程执行协议）。
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct WorkerRequest {
    /// 不可信脚本源码。
    pub script: String,
    /// JSON 可表达的外部变量。
    #[serde(default)]
    pub context: Map<String, Value>,
    /// 审计和缓存隔离使用的租户标识。
    #[serde(default = "default_tenant")]
    pub tenant_id: String,
    /// 可选的进程内资源预算；省略时使用安全默认值。
    #[serde(default)]
    pub resource_limits: Option<ResourceLimits>,
}

fn default_tenant() -> String {
    "worker".to_string()
}
