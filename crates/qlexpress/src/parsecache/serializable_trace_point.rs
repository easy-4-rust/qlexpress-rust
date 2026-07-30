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

impl SerializableTracePoint {
    /// 返回 trace 类型名。
    ///
    /// 对应 Java：`SerializableTracePoint#getType()`。
    ///
    /// # 返回值
    ///
    /// 返回 `TraceType.name()` 字符串；Java `null` 以 `None` 表示。
    pub fn get_type(&self) -> Option<&str> {
        self.trace_type.as_deref()
    }

    /// 设置 trace 类型名。
    ///
    /// 对应 Java：`SerializableTracePoint#setType(String)`。
    ///
    /// # 参数
    ///
    /// - `trace_type`：新的 trace 类型名；`None` 对应 Java `null`。
    pub fn set_type(&mut self, trace_type: Option<String>) {
        self.trace_type = trace_type;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// SOURCE_PARITY: SerializableTracePoint#getType/setType。
    #[test]
    fn type_accessors_preserve_java_nullability() {
        let mut point = SerializableTracePoint::default();
        assert_eq!(point.get_type(), None);
        point.set_type(Some("OPERATOR".to_string()));
        assert_eq!(point.get_type(), Some("OPERATOR"));
        point.set_type(None);
        assert_eq!(point.get_type(), None);
    }
}
