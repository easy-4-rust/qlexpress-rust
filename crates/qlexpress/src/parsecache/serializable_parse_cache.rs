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
///
/// 对应 Java: com.alibaba.qlexpress4.api.parsecache.SerializableParseCache。
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
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

#[cfg(test)]
mod tests {
    use super::*;

    /// SOURCE_PARITY: Java 无参构造器及六组 JavaBean getter/setter 在
    /// Rust 中适配为 `Default` 与公开字段读写，同时保持 camelCase JSON。
    #[test]
    fn public_fields_preserve_java_bean_and_json_contract() {
        let mut cache = SerializableParseCache::default();
        assert_eq!(cache.model_version, 0);
        assert_eq!(cache.producer_version, None);
        assert_eq!(cache.script, None);
        assert_eq!(cache.script_hash, None);
        assert_eq!(cache.main, None);
        assert_eq!(cache.trace_points, None);

        cache.model_version = 1;
        cache.producer_version = Some("0.1.0-alpha.2".to_string());
        cache.script = Some("1 + 2".to_string());
        cache.script_hash = Some("abc123".to_string());
        cache.main = Some(SerializableLambdaDefinition {
            name: Some("main".to_string()),
            max_stack_size: 2,
            ..SerializableLambdaDefinition::default()
        });
        cache.trace_points = Some(vec![SerializableTracePoint {
            trace_type: Some("VALUE".to_string()),
            token: Some("1".to_string()),
            line: 1,
            col: 1,
            position: 0,
            children: None,
        }]);

        let json = serde_json::to_value(&cache).expect("serialize parse cache");
        assert_eq!(json["modelVersion"], 1);
        assert_eq!(json["producerVersion"], "0.1.0-alpha.2");
        assert_eq!(json["script"], "1 + 2");
        assert_eq!(json["scriptHash"], "abc123");
        assert_eq!(json["main"]["name"], "main");
        assert_eq!(json["main"]["maxStackSize"], 2);
        assert_eq!(json["tracePoints"][0]["type"], "VALUE");

        let restored: SerializableParseCache =
            serde_json::from_value(json).expect("deserialize parse cache");
        assert_eq!(restored, cache);
    }
}
