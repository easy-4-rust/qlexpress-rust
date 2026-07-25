//! Error wrapping helpers, mirroring Java `ThrowUtils`.

use crate::exception::error_codes;
use crate::exception::error_reporter::ErrorReporter;
use crate::exception::QLException;

/// Java `ThrowUtils.wrapThrowable(Throwable, ErrorReporter, errCode,
/// errMsg, args...)`: a `QLRuntimeException` is rethrown unchanged.
///
/// In Rust every script-side failure is already a [`QLException`] (the
/// `QLRuntimeException` analogue), so — exactly like the Java `instanceof`
/// branch — the error passes through unchanged. The extra arguments are
/// kept for call-site parity with Java and to document intent.
pub fn wrap_throwable(
    err: QLException,
    _error_reporter: &dyn ErrorReporter,
    _err_code: &str,
    _err_msg: &str,
    _args: &[String],
) -> QLException {
    err
}

/// Java `ThrowUtils.reportUserDefinedException`: `INVALID_ARGUMENT` keeps
/// its code, everything else becomes `BIZ_EXCEPTION`.
///
/// The Rust migration represents Java `UserDefineException` as a
/// [`QLException`] whose code is already `INVALID_ARGUMENT` (argument
/// errors) or another code (business errors), mirroring how the Java
/// catch sites re-report only the message.
pub fn report_user_defined_exception(
    error_reporter: &dyn ErrorReporter,
    err: &QLException,
) -> QLException {
    if err.error_code() == error_codes::INVALID_ARGUMENT {
        error_reporter.report(error_codes::INVALID_ARGUMENT, err.reason())
    } else {
        error_reporter.report(error_codes::BIZ_EXCEPTION, err.reason())
    }
}
