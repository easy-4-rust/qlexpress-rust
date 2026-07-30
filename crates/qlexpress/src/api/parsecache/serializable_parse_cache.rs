//! 可序列化编译缓存,对应 Java `com.alibaba.qlexpress4.api.parsecache.SerializableParseCache`。
//! 职责:一个脚本编译产物(Lambda 定义 + trace 点)的完整可序列化形态。

use serde::{Deserialize, Serialize};

use super::serializable_lambda_definition::SerializableLambdaDefinition;
use super::serializable_trace_point::SerializableTracePoint;

/// 可序列化编译缓存。对应 Java: com.alibaba.qlexpress4.api.parsecache.SerializableParseCache
///
/// 字段对照:
/// - `modelVersion`(int):模型版本(当前为 1,见 Exporter `MODEL_VERSION`);
/// - `producerVersion`(String):产出方版本(Java 取包实现版本);
/// - `script`(String):脚本原文;
/// - `scriptHash`(String):脚本 SHA-256(十六进制);
/// - `main`(SerializableLambdaDefinition):主 Lambda 定义;
/// - `tracePoints`(List&lt;SerializableTracePoint&gt;):表达式 trace 点(可选)。
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
/// 对应 Java: com.alibaba.qlexpress4.api.parsecache.SerializableParseCache。
pub struct SerializableParseCache {
    /// 模型版本。对应 Java 字段 `modelVersion`。
    #[serde(default)]
    pub model_version: i32,
    /// 产出方版本。对应 Java 字段 `producerVersion`(可空)。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub producer_version: Option<String>,
    /// 脚本原文。对应 Java 字段 `script`。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub script: Option<String>,
    /// 脚本 SHA-256(十六进制)。对应 Java 字段 `scriptHash`。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub script_hash: Option<String>,
    /// 主 Lambda 定义。对应 Java 字段 `main`。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub main: Option<SerializableLambdaDefinition>,
    /// 表达式 trace 点。对应 Java 字段 `tracePoints`(可空)。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub trace_points: Option<Vec<SerializableTracePoint>>,
}
