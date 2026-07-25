//! 可序列化 catch 表项,对应 Java `com.alibaba.qlexpress4.api.parsecache.SerializableCatchEntry`。
//! 职责:描述 try-catch 异常表中「异常类型 → 处理 Lambda」的一条映射。

use serde::{Deserialize, Serialize};

use super::serializable_lambda_definition::SerializableLambdaDefinition;

/// 可序列化 catch 表项。对应 Java: com.alibaba.qlexpress4.api.parsecache.SerializableCatchEntry
///
/// 字段对照:
/// - `exceptionClassName`(String):异常类型的 Java 全限定名;
/// - `handler`(SerializableLambdaDefinition):该异常的处理 Lambda。
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SerializableCatchEntry {
    /// 异常类型名。对应 Java 字段 `exceptionClassName`。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub exception_class_name: Option<String>,
    /// 处理 Lambda。对应 Java 字段 `handler`。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub handler: Option<SerializableLambdaDefinition>,
}
