//! 可序列化源码位置,对应 Java `com.alibaba.qlexpress4.api.parsecache.SerializableSource`。
//! 职责:以纯数据形式描述一条指令在脚本中的源码位置(供 JSON 序列化)。

use serde::{Deserialize, Serialize};

/// 可序列化源码位置。对应 Java: com.alibaba.qlexpress4.api.parsecache.SerializableSource
///
/// 字段对照(Java POJO ↔ serde JSON,字段名一致):
/// - `start`(int):脚本中的绝对起始偏移(0 基);
/// - `line`(int):1 基行号;
/// - `col`(int):0 基列号(Java 导出时 `col - 1`);
/// - `lexeme`(String):位置处的词素,可为 null。
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct SerializableSource {
    /// 脚本中的绝对起始偏移(0 基)。对应 Java 字段 `start`。
    #[serde(default)]
    pub start: i32,
    /// 1 基行号。对应 Java 字段 `line`。
    #[serde(default)]
    pub line: i32,
    /// 0 基列号。对应 Java 字段 `col`。
    #[serde(default)]
    pub col: i32,
    /// 词素。对应 Java 字段 `lexeme`(可空)。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub lexeme: Option<String>,
}
