//! 可序列化常量,对应 Java `com.alibaba.qlexpress4.api.parsecache.SerializableConstant`。
//! 职责:以「类型标签 + JSON 值」形式描述 CONST 指令的常量对象。

use serde::{Deserialize, Serialize};

/// 可序列化常量。对应 Java: com.alibaba.qlexpress4.api.parsecache.SerializableConstant
///
/// 字段对照:
/// - `type`(String):类型标签,取值为 `NULL / BOOLEAN / STRING / CHAR /
///   INT / LONG / BIG_INTEGER / FLOAT / DOUBLE / BIG_DECIMAL / META_CLASS`
///   (见 Exporter/Importer 的常量分派);
/// - `value`(Object):常量值(JSON 值;`NULL` 类型时缺省;`CHAR /
///   BIG_INTEGER / BIG_DECIMAL / META_CLASS` 以字符串承载)。
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct SerializableConstant {
    /// 类型标签。对应 Java 字段 `type`。
    #[serde(rename = "type", default, skip_serializing_if = "Option::is_none")]
    pub const_type: Option<String>,
    /// 常量值。对应 Java 字段 `value`。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub value: Option<serde_json::Value>,
}

impl SerializableConstant {
    /// 返回常量的序列化类型标签。
    ///
    /// 对应 Java：`SerializableConstant#getType()`。
    ///
    /// # 返回值
    ///
    /// 返回类型标签；Java 的 `null` 以 `None` 表示。
    pub fn get_type(&self) -> Option<&str> {
        self.const_type.as_deref()
    }

    /// 设置常量的序列化类型标签。
    ///
    /// 对应 Java：`SerializableConstant#setType(String)`。
    ///
    /// # 参数
    ///
    /// - `const_type`：新类型标签；`None` 对应 Java `null`。
    pub fn set_type(&mut self, const_type: Option<String>) {
        self.const_type = const_type;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// SOURCE_PARITY: SerializableConstant#getType/setType。
    #[test]
    fn type_accessors_preserve_java_nullability() {
        let mut constant = SerializableConstant::default();
        assert_eq!(constant.get_type(), None);
        constant.set_type(Some("LONG".to_string()));
        assert_eq!(constant.get_type(), Some("LONG"));
        constant.set_type(None);
        assert_eq!(constant.get_type(), None);
    }
}
