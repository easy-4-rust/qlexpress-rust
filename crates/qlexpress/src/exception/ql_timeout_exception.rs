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

    /// 执行 `inner` 公开操作。对应 Java 源码 `com/alibaba/qlexpress4/exception/QLTimeoutException.java:1` 的 `QLTimeoutException`；该方法为 Rust 同职责适配接口。
    pub fn inner(&self) -> &QLException {
        &self.inner
    }

    /// 执行 `into_exception` 公开操作。对应 Java 源码 `com/alibaba/qlexpress4/exception/QLTimeoutException.java:1` 的 `QLTimeoutException`；该方法为 Rust 同职责适配接口。
    pub fn into_exception(self) -> QLException {
        self.inner
    }
}
