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

    pub fn inner(&self) -> &QLException {
        &self.inner
    }

    pub fn catch_obj(&self) -> Option<&DataValue> {
        self.inner.catch_obj()
    }

    pub fn into_exception(self) -> QLException {
        self.inner
    }
}
