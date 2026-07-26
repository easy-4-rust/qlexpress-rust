use super::ql_exception::{QLException, QLExceptionKind};

/// Syntax-phase error, mirroring Java `QLSyntaxException`.
///
/// Wrapper around the unified [`QLException`] with
/// [`QLExceptionKind::Syntax`]; convert with [`Self::into_exception`].
#[derive(Clone, Debug)]
pub struct QLSyntaxException {
    inner: QLException,
}

impl QLSyntaxException {
    /// Wrap an already-built [`QLException`] (must have `Syntax` kind).
    pub(crate) fn from_exception(inner: QLException) -> Self {
        debug_assert_eq!(inner.kind(), QLExceptionKind::Syntax);
        QLSyntaxException { inner }
    }

    /// 执行 `inner` 公开操作。对应 Java 源码 `com/alibaba/qlexpress4/exception/QLSyntaxException.java:1` 的 `QLSyntaxException`；该方法为 Rust 同职责适配接口。
    pub fn inner(&self) -> &QLException {
        &self.inner
    }

    /// 执行 `into_exception` 公开操作。对应 Java 源码 `com/alibaba/qlexpress4/exception/QLSyntaxException.java:1` 的 `QLSyntaxException`；该方法为 Rust 同职责适配接口。
    pub fn into_exception(self) -> QLException {
        self.inner
    }
}

impl std::ops::Deref for QLSyntaxException {
    type Target = QLException;

    /// Transparent access to the wrapped [`QLException`] diagnostics
    /// (`error_code()`, `line_no()`, `col_no()`, `reason()`, ...).
    fn deref(&self) -> &QLException {
        &self.inner
    }
}
