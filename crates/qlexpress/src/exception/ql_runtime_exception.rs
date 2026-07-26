//! 对应 Java 类：com.alibaba.qlexpress4.exception.QLRuntimeException
//!
//! 运行时异常构造辅助，对应 `QLExceptionKind::Runtime`。

use super::ql_exception::{QLException, QLExceptionKind};
use crate::runtime::value::DataValue;

/// Runtime error, mirroring Java `QLRuntimeException`.
///
/// The catchable attachment lives on [`QLException::catch_obj`]; this wrapper
/// exists so call sites can express the Java type relationship explicitly.
#[derive(Clone, Debug)]
pub struct QLRuntimeException {
    inner: QLException,
}

impl QLRuntimeException {
    /// Mirrors the Java "Visible for test"
    /// `QLRuntimeException(catchObj, reason, errorCode)` constructor.
    pub fn for_test(catch_obj: Option<DataValue>, reason: &str, error_code: &str) -> Self {
        let mut inner = QLException::for_test(QLExceptionKind::Runtime, reason, error_code);
        if let Some(obj) = catch_obj {
            inner = inner.with_catch_obj(obj);
        }
        QLRuntimeException { inner }
    }

    /// 执行 `inner` 公开操作。对应 Java 源码 `com/alibaba/qlexpress4/exception/QLRuntimeException.java:1` 的 `QLRuntimeException`；该方法为 Rust 同职责适配接口。
    pub fn inner(&self) -> &QLException {
        &self.inner
    }

    /// 执行 `catch_obj` 公开操作。对应 Java 源码 `com/alibaba/qlexpress4/exception/QLRuntimeException.java:1` 的 `QLRuntimeException`；该方法为 Rust 同职责适配接口。
    pub fn catch_obj(&self) -> Option<&DataValue> {
        self.inner.catch_obj()
    }

    /// 执行 `into_exception` 公开操作。对应 Java 源码 `com/alibaba/qlexpress4/exception/QLRuntimeException.java:1` 的 `QLRuntimeException`；该方法为 Rust 同职责适配接口。
    pub fn into_exception(self) -> QLException {
        self.inner
    }
}
