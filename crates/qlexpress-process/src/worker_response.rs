//! Worker 标准输出协议。

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// 单次隔离进程执行的 JSON 响应。
///
/// `ok` 为真时只有 `result` 有值；失败时通过稳定的 `error_code` 和
/// 面向诊断的 `reason` 描述结果。
/// 对应 Java: 无（Rust 进程执行协议）。
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct WorkerResponse {
    /// 是否成功。
    pub ok: bool,
    /// 成功时的 JSON 结果。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<Value>,
    /// 失败时的稳定错误码。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error_code: Option<String>,
    /// 失败原因。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
}

impl WorkerResponse {
    /// 创建成功响应。
    ///
    /// # Arguments
    ///
    /// * `result` - 可序列化为 JSON 的规则结果。
    ///
    /// # Returns
    ///
    /// 返回 `ok = true` 且错误字段为空的响应。
    pub fn success(result: Value) -> Self {
        Self {
            ok: true,
            result: Some(result),
            error_code: None,
            reason: None,
        }
    }

    /// 创建失败响应。
    ///
    /// # Arguments
    ///
    /// * `error_code` - 供调用方稳定判断失败类别的错误码。
    /// * `reason` - 用于日志和诊断的具体原因。
    ///
    /// # Returns
    ///
    /// 返回 `ok = false` 且结果字段为空的响应。
    pub fn failure(error_code: impl Into<String>, reason: impl Into<String>) -> Self {
        Self {
            ok: false,
            result: None,
            error_code: Some(error_code.into()),
            reason: Some(reason.into()),
        }
    }
}
