//! 可序列化 trace 点,对应 Java `com.alibaba.qlexpress4.api.parsecache.SerializableTracePoint`。
//! 职责:以纯数据形式描述表达式 trace 树的一个节点(递归含子节点)。

use serde::{Deserialize, Serialize};

/// 可序列化 trace 点。对应 Java: com.alibaba.qlexpress4.api.parsecache.SerializableTracePoint
///
/// 字段对照:
/// - `type`(String):trace 类型,Java `TraceType.name()`(如 `OPERATOR /
///   FUNCTION / METHOD / FIELD / IF / VARIABLE / VALUE ...`);
/// - `token`(String):词素;
/// - `line` / `col` / `position`(int):源码位置(1 基行、1 基列、绝对偏移);
/// - `children`(List&lt;SerializableTracePoint&gt;):子 trace 点。
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct SerializableTracePoint {
    /// trace 类型名(Java `TraceType.name()`)。对应 Java 字段 `type`。
    #[serde(rename = "type", default, skip_serializing_if = "Option::is_none")]
    pub trace_type: Option<String>,
    /// 词素。对应 Java 字段 `token`。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub token: Option<String>,
    /// 1 基行号。对应 Java 字段 `line`。
    #[serde(default)]
    pub line: i32,
    /// 1 基列号。对应 Java 字段 `col`。
    #[serde(default)]
    pub col: i32,
    /// 绝对偏移。对应 Java 字段 `position`。
    #[serde(default)]
    pub position: i32,
    /// 子 trace 点。对应 Java 字段 `children`。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub children: Option<Vec<SerializableTracePoint>>,
}
