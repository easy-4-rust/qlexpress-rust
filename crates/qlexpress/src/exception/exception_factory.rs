/// Factory of typed exceptions, mirroring Java `ExceptionFactory<T>`.
pub trait ExceptionFactory<T> {
    #[allow(clippy::too_many_arguments)]
    fn new_exception(
        &self,
        message: &str,
        line_no: i32,
        col_no: i32,
        err_lexeme: &str,
        error_code: &str,
        reason: &str,
        snippet: &str,
    ) -> T;
}
