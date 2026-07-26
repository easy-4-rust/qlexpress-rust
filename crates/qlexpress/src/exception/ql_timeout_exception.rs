use super::ql_exception::{QLException, QLExceptionKind};
use crate::runtime::value::DataValue;

/// Script-timeout error, mirroring Java `QLTimeoutException`.
#[derive(Clone, Debug)]
pub struct QLTimeoutException {
    inner: QLException,
}

impl QLTimeoutException {
    /// Mirrors the Java "Visible for test"
    /// `QLTimeoutException(catchObj, reason, errorCode)` constructor.
    pub fn for_test(catch_obj: Option<DataValue>, reason: &str, error_code: &str) -> Self {
        let mut inner = QLException::for_test(QLExceptionKind::Timeout, reason, error_code);
        if let Some(obj) = catch_obj {
            inner = inner.with_catch_obj(obj);
        }
        QLTimeoutException { inner }
    }

    /// 返回内部通用 QL 异常。
    /// 对应 Java: `QLTimeoutException` 继承 `QLException` 后暴露的基类状态。
    pub fn inner(&self) -> &QLException {
        &self.inner
    }

    /// 将超时异常消费并转换为通用 QL 异常。
    /// 对应 Java: `QLTimeoutException` 向 `QLException` 的继承转换。
    pub fn into_exception(self) -> QLException {
        self.inner
    }
}
