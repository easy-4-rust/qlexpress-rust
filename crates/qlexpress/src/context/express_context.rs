//! 外部变量上下文接口,对应 Java `com.alibaba.qlexpress4.runtime.context.ExpressContext`。

use crate::exception::QLException;
use crate::ql_options::Attachments;
use crate::runtime::value::QValue;

/// 外部变量上下文。对应 Java: com.alibaba.qlexpress4.runtime.context.ExpressContext
/// (职责:脚本运行期按变量名向宿主环境查询外部变量)。
///
/// Java 签名 `Value get(Map<String, Object> attachments, String variableName)`
/// 允许返回 `null`(表示宿主环境中不存在该变量);Rust 用 `Option<QValue>`
/// 表达这一可空语义,`Ok(None)` 即 Java 的 `null` 返回。
///
/// 注意 Java 允许 `get` 内抛出运行时异常(如 `DynamicVariableContext`
/// 执行动态脚本失败),Rust 以 `Result` 传播,与 Java 的非受检异常上抛一致。
pub trait ExpressContext {
    /// 按变量名取外部变量。对应 Java 方法 `get(attachments, variableName)`。
    ///
    /// - `attachments`: 用户随 `QLOptions` 传入的附加数据(Java `qlOptions.getAttachments()`)。
    /// - 返回 `Ok(None)` 表示 Java 返回 `null`(变量不存在);
    ///   返回 `Ok(Some(...))` 表示命中(值本身仍可为 `DataValue::Null`,
    ///   对应 Java 的 `Value.NULL_VALUE`,如 `EmptyContext`)。
    fn get(
        &self,
        attachments: &Attachments,
        variable_name: &str,
    ) -> Result<Option<QValue>, QLException>;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runtime::value::DataValue;

    struct RecordingContext;

    impl ExpressContext for RecordingContext {
        fn get(
            &self,
            attachments: &Attachments,
            variable_name: &str,
        ) -> Result<Option<QValue>, QLException> {
            if variable_name == "attachment" {
                return Ok(attachments.get("value").cloned().map(QValue::Data));
            }
            if variable_name == "explicit_null" {
                return Ok(Some(QValue::Data(DataValue::NULL_VALUE)));
            }
            Ok(None)
        }
    }

    /// SOURCE_PARITY: Java `ExpressContext#get` 同时接收 attachments 与变量名；
    /// Java null（未命中）和 `Value.NULL_VALUE`（命中但值为 null）必须可区分。
    #[test]
    fn contract_preserves_attachments_missing_and_explicit_null() {
        let mut attachments = Attachments::new();
        attachments.insert("value".to_string(), DataValue::Int(42));
        let context = RecordingContext;

        let attachment = context
            .get(&attachments, "attachment")
            .unwrap()
            .expect("attachment must exist");
        assert!(matches!(attachment.get(), DataValue::Int(42)));
        assert!(context.get(&attachments, "missing").unwrap().is_none());
        let explicit_null = context
            .get(&attachments, "explicit_null")
            .unwrap()
            .expect("explicit null is still a present Value");
        assert!(explicit_null.get().is_null());
    }
}
