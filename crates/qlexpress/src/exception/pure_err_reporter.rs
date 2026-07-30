//! 对应 Java 类：com.alibaba.qlexpress4.exception.PureErrReporter
//!
//! 纯文本错误报告器：仅返回错误码与原始消息，不含堆栈，用于编译期校验等场景。

use super::error_codes::format_msg;
use super::error_reporter::ErrorReporter;
use super::ql_exception::{QLException, QLExceptionKind};
use crate::runtime::value::DataValue;

/// `PureErrReporter` 结构体的 Rust 实现，保留对应对象的领域职责与公开契约。
/// 对应或承接 Java 源文件：`com/alibaba/qlexpress4/exception/PureErrReporter.java`；具体对象路径见 `docs/对象级对照表.md`。
/// Reporter without script context, mirroring Java `PureErrReporter`.
///
/// Builds a bare runtime exception (dummy diagnostic, no snippet). Note that
/// like the Java version, the catch object is *not* propagated here (Java
/// passes `null` to the exception constructor).
#[derive(Clone, Copy, Debug, Default)]
/// 对应 Java: com.alibaba.qlexpress4.exception.PureErrReporter。
pub struct PureErrReporter;

impl PureErrReporter {
    /// Java `PureErrReporter.INSTANCE` singleton — cheap to construct.
    pub const INSTANCE: PureErrReporter = PureErrReporter;
}

impl ErrorReporter for PureErrReporter {
    fn report_format_with_catch(
        &self,
        _catch_obj: Option<DataValue>,
        error_code: &str,
        format: &str,
        args: &[String],
    ) -> QLException {
        // Java maps a SCRIPT_TIME_OUT code to QLTimeOutException
        // (`QLException.reportRuntimeErr*`); mirror the kind selection.
        let kind = if error_code == super::error_codes::SCRIPT_TIME_OUT {
            QLExceptionKind::Timeout
        } else {
            QLExceptionKind::Runtime
        };
        QLException::for_test(kind, format_msg(format, args), error_code)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::exception::error_codes;
    use crate::exception::error_reporter::ErrorReporter;

    #[test]
    fn pure_reporter_formats_and_keeps_code() {
        let reporter = PureErrReporter::INSTANCE;
        let err = reporter.report_format(
            error_codes::FUNCTION_NOT_FOUND,
            error_codes::error_msg(error_codes::FUNCTION_NOT_FOUND),
            &["myFn".to_string()],
        );
        assert_eq!(err.error_code(), error_codes::FUNCTION_NOT_FOUND);
        assert_eq!(err.reason(), "function 'myFn' not found");
        assert_eq!(err.kind(), QLExceptionKind::Runtime);
    }

    #[test]
    fn report_uses_reason_verbatim() {
        let reporter = PureErrReporter::INSTANCE;
        let err = reporter.report(error_codes::SYNTAX_ERROR, "100% bad");
        assert_eq!(err.reason(), "100% bad");
    }
}
