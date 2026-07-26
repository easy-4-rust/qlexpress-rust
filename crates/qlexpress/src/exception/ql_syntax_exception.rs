use super::ql_exception::{QLException, QLExceptionKind};

/// `QLSyntaxException` 结构体的 Rust 实现，保留对应对象的领域职责与公开契约。
/// 对应或承接 Java 源文件：`com/alibaba/qlexpress4/exception/QLSyntaxException.java`；具体对象路径见 `docs/对象级对照表.md`。
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

    /// 返回内部通用 QL 异常。
    /// 对应 Java: `QLSyntaxException` 继承 `QLException` 后暴露的基类状态。
    pub fn inner(&self) -> &QLException {
        &self.inner
    }

    /// 将语法异常消费并转换为通用 QL 异常。
    /// 对应 Java: `QLSyntaxException` 向 `QLException` 的继承转换。
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
