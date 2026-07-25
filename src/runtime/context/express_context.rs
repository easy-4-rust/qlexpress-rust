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
