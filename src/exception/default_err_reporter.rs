use super::error_codes::format_msg;
use super::error_reporter::ErrorReporter;
use super::ql_exception::QLException;
use crate::runtime::value::DataValue;

/// Reporter bound to a script position, mirroring Java `DefaultErrReporter`.
#[derive(Clone, Debug)]
pub struct DefaultErrReporter {
    script: String,
    token_start_pos: i32,
    line: i32,
    col: i32,
    lexeme: String,
}

impl DefaultErrReporter {
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

    pub fn script(&self) -> &str {
        &self.script
    }

    pub fn token_start_pos(&self) -> i32 {
        self.token_start_pos
    }

    pub fn line(&self) -> i32 {
        self.line
    }

    pub fn col(&self) -> i32 {
        self.col
    }

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
        assert!(err.to_string().contains("[Error FIELD_NOT_FOUND: 'b' field not found]"));
    }
}
