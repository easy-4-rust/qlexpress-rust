//! Worker 标准输出协议。

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// 单次 Worker 执行响应。
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
    pub fn success(result: Value) -> Self {
        Self {
            ok: true,
            result: Some(result),
            error_code: None,
            reason: None,
        }
    }

    /// 创建失败响应。
    pub fn failure(error_code: impl Into<String>, reason: impl Into<String>) -> Self {
        Self {
            ok: false,
            result: None,
            error_code: Some(error_code.into()),
            reason: Some(reason.into()),
        }
    }
}
