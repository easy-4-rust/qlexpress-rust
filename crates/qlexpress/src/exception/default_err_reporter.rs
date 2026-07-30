//! 对应 Java 类：com.alibaba.qlexpress4.exception.DefaultErrReporter
//!
//! 默认错误报告器：携带堆栈与源位置，输出含完整 trace 的 `QLException`。

use super::error_codes::format_msg;
use super::error_reporter::ErrorReporter;
use super::ql_exception::QLException;
use crate::runtime::value::DataValue;

/// 使用当前语法节点位置构造 QL 异常的默认错误报告器。
/// 对应或承接 Java 源文件：`com/alibaba/qlexpress4/exception/DefaultErrReporter.java`；具体对象路径见 `docs/对象级对照表.md`。
/// Reporter bound to a script position, mirroring Java `DefaultErrReporter`.
#[derive(Clone, Debug)]
/// 对应 Java: com.alibaba.qlexpress4.exception.DefaultErrReporter。
pub struct DefaultErrReporter {
    script: String,
    token_start_pos: i32,
    line: i32,
    col: i32,
    lexeme: String,
}

impl DefaultErrReporter {
    /// 构造实例。对应 Java 源码 `com/alibaba/qlexpress4/exception/DefaultErrReporter.java:18` 的 `DefaultErrReporter::<init>`。
    pub fn new(
        script: impl Into<String>,
        token_start_pos: i32,
        line: i32,
        col: i32,
        lexeme: impl Into<String>,
    ) -> Self {
        DefaultErrReporter {
            script: script.into(),
            token_start_pos,
            line,
            col,
            lexeme: lexeme.into(),
        }
    }

    /// 返回发生错误的原始脚本。
    /// 对应 Java: `DefaultErrReporter` 保存的 `script`。
    pub fn script(&self) -> &str {
        &self.script
    }

    /// 返回错误 token 的字符起始偏移。
    /// 对应 Java: `DefaultErrReporter` 保存的 `tokenStartPos`。
    pub fn token_start_pos(&self) -> i32 {
        self.token_start_pos
    }

    /// 返回错误 token 的一基行号。
    /// 对应 Java: `DefaultErrReporter` 保存的 `line`。
    pub fn line(&self) -> i32 {
        self.line
    }

    /// 返回错误 token 的一基列号。
    /// 对应 Java: `DefaultErrReporter` 保存的 `col`。
    pub fn col(&self) -> i32 {
        self.col
    }

    /// 返回触发错误的词素。
    /// 对应 Java: `DefaultErrReporter` 保存的 `lexeme`。
    pub fn lexeme(&self) -> &str {
        &self.lexeme
    }
}

impl ErrorReporter for DefaultErrReporter {
    /// 向下转型支持(Java `instanceof DefaultErrReporter`)。
    fn as_any(&self) -> Option<&dyn std::any::Any> {
        Some(self)
    }

    fn report_format_with_catch(
        &self,
        catch_obj: Option<DataValue>,
        error_code: &str,
        format: &str,
        args: &[String],
    ) -> QLException {
        QLException::report_runtime_err_with_attach(
            &self.script,
            self.token_start_pos,
            self.line,
            self.col,
            &self.lexeme,
            error_code,
            &format_msg(format, args),
            catch_obj,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::exception::error_codes;

    #[test]
    fn default_reporter_attaches_position() {
        let reporter = DefaultErrReporter::new("a.b()", 2, 1, 3, "b");
        let err = reporter.report(error_codes::FIELD_NOT_FOUND, "'b' field not found");
        assert_eq!(err.line_no(), 1);
        assert_eq!(err.col_no(), 3);
        assert_eq!(err.err_lexeme(), "b");
        assert!(err
            .to_string()
            .contains("[Error FIELD_NOT_FOUND: 'b' field not found]"));
    }
}
