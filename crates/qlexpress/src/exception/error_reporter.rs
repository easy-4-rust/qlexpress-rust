//! 对应 Java 类：com.alibaba.qlexpress4.exception.ErrorReporter
//!
//! 错误报告器 trait：在 AST 节点处收集诊断信息（位置、错误码、参数）
//! 并生成 `QLException`。

use super::ql_exception::QLException;
use crate::runtime::value::DataValue;

/// `ErrorReporter` 接口的 Rust 实现，保留对应对象的领域职责与公开契约。
/// 对应或承接 Java 源文件：`com/alibaba/qlexpress4/exception/ErrorReporter.java`；具体对象路径见 `docs/对象级对照表.md`。
/// How parser/instructions report errors, mirroring Java `ErrorReporter`.
///
/// The single required method is [`Self::report_format_with_catch`]; the
/// others are the Java `default` convenience methods.
pub trait ErrorReporter {
    /// 校验或报告 with catch。
    /// 参数：`catch_obj`、`error_code`、`reason`；返回：`QLException`。
    /// 对应或承接 Java 源文件：`com/alibaba/qlexpress4/exception/ErrorReporter.java`，方法 `reportWithCatch`。
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

    /// 处理 report 对应的接口职责。
    /// 参数：`error_code`、`reason`；返回：`QLException`。
    /// 对应或承接 Java 源文件：`com/alibaba/qlexpress4/exception/ErrorReporter.java`，方法 `report`。
    /// Java `report(String errorCode, String reason)`.
    fn report(&self, error_code: &str, reason: &str) -> QLException {
        self.report_with_catch(None, error_code, reason)
    }

    /// 校验或报告 format。
    /// 参数：`error_code`、`format`、`args`；返回：`QLException`。
    /// 对应或承接 Java 源文件：`com/alibaba/qlexpress4/exception/ErrorReporter.java`，方法 `reportFormat`。
    /// Java `reportFormat(String errorCode, String format, Object... args)`.
    fn report_format(&self, error_code: &str, format: &str, args: &[String]) -> QLException {
        self.report_format_with_catch(None, error_code, format, args)
    }

    /// 校验或报告 format with catch。
    /// 参数：`catch_obj`、`error_code`、`format`、`args`；返回：`QLException`。
    /// 对应或承接 Java 源文件：`com/alibaba/qlexpress4/exception/ErrorReporter.java`，方法 `reportFormatWithCatch`。
    /// Java `reportFormatWithCatch(Object catchObj, String errorCode,
    /// String format, Object... args)`.
    fn report_format_with_catch(
        &self,
        catch_obj: Option<DataValue>,
        error_code: &str,
        format: &str,
        args: &[String],
    ) -> QLException;

    /// 向下转型支持(Java `instanceof DefaultErrReporter` 的 Rust 等价物),
    /// 供 `api/parsecache` Exporter 读取源码位置。默认 `None`。
    fn as_any(&self) -> Option<&dyn std::any::Any> {
        None
    }
}
