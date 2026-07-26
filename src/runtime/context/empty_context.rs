//! 空外部上下文,对应 Java `com.alibaba.qlexpress4.runtime.context.EmptyContext`。

use crate::exception::QLException;
use crate::ql_options::Attachments;
use crate::runtime::context::express_context::ExpressContext;
use crate::runtime::value::{DataValue, QValue};

/// 空上下文。对应 Java: com.alibaba.qlexpress4.runtime.context.EmptyContext
/// (职责:任何变量名都返回 `Value.NULL_VALUE`,即「存在但为 null」)。
///
/// 对应 Java `ExpressContext.EMPTY_CONTEXT` 的默认实例语义:
/// 注意它返回的不是 Java `null`,而是 `NULL_VALUE`,
/// 因此 `QvmGlobalScope.getSymbol` 会把它当作「命中外部变量」处理。
pub struct EmptyContext;

impl EmptyContext {
    /// 构造空上下文。对应 Java `new EmptyContext()`。
    pub fn new() -> Self {
        EmptyContext
    }
}

impl Default for EmptyContext {
    fn default() -> Self {
        Self::new()
    }
}

impl ExpressContext for EmptyContext {
    /// 对应 Java 方法 `get`:恒返回 `Value.NULL_VALUE`。
    fn get(
        &self,
        _attachments: &Attachments,
        _variable_name: &str,
    ) -> Result<Option<QValue>, QLException> {
        Ok(Some(QValue::Data(DataValue::NULL_VALUE)))
    }
}
