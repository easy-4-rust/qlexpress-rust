use super::ql_exception::QLException;
use crate::runtime::value::DataValue;

/// How parser/instructions report errors, mirroring Java `ErrorReporter`.
///
/// The single required method is [`Self::report_format_with_catch`]; the
/// others are the Java `default` convenience methods.
pub trait ErrorReporter {
    /// Java `report(Object catchObj, String errorCode, String reason)`:
    /// `reason` is used as-is (not as a format string).
    fn report_with_catch(
        &self,
        catch_obj: Option<DataValue>,
        error_code: &str,
        reason: &str,
    ) -> QLException {
        self.report_format_with_catch(catch_obj, error_code, reason, &[])
    }

    /// Java `report(String errorCode, String reason)`.
    fn report(&self, error_code: &str, reason: &str) -> QLException {
        self.report_with_catch(None, error_code, reason)
    }

    /// Java `reportFormat(String errorCode, String format, Object... args)`.
    fn report_format(&self, error_code: &str, format: &str, args: &[String]) -> QLException {
        self.report_format_with_catch(None, error_code, format, args)
    }

    /// Java `reportFormatWithCatch(Object catchObj, String errorCode,
    /// String format, Object... args)`.
    fn report_format_with_catch(
        &self,
        catch_obj: Option<DataValue>,
        error_code: &str,
        format: &str,
        args: &[String],
    ) -> QLException;
}
