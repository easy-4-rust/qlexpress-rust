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

    pub fn inner(&self) -> &QLException {
        &self.inner
    }

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
