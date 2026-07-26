/// Factory of typed exceptions, mirroring Java `ExceptionFactory<T>`.
pub trait ExceptionFactory<T> {
    #[allow(clippy::too_many_arguments)]
    /// 执行 `new_exception` 公开操作。对应 Java 源码 `com/alibaba/qlexpress4/exception/ExceptionFactory.java:8` 的 `ExceptionFactory#newException`。
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
