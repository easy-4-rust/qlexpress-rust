//! 可序列化 Lambda 参数,对应 Java `com.alibaba.qlexpress4.api.parsecache.SerializableParam`。
//! 职责:以纯数据形式描述 Lambda 参数声明(名称 + 类型名)。

use serde::{Deserialize, Serialize};

/// 可序列化 Lambda 参数。对应 Java: com.alibaba.qlexpress4.api.parsecache.SerializableParam
///
/// 字段对照:
/// - `name`(String):参数名;
/// - `className`(String):参数声明类型的 Java 全限定名(如
///   `java.lang.Integer`;Java 侧为 `Class.getName()`)。
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct SerializableParam {
    /// 参数名。对应 Java 字段 `name`。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// 参数类型名。对应 Java 字段 `className`。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub class_name: Option<String>,
}
