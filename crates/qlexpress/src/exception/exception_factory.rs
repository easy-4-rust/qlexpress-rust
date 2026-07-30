/// `ExceptionFactory` 接口的 Rust 实现，保留对应对象的领域职责与公开契约。
/// 对应或承接 Java 源文件：`com/alibaba/qlexpress4/exception/ExceptionFactory.java`；具体对象路径见 `docs/对象级对照表.md`。
/// Factory of typed exceptions, mirroring Java `ExceptionFactory<T>`.
/// 对应 Java: com.alibaba.qlexpress4.exception.ExceptionFactory。
pub trait ExceptionFactory<T> {
    #[allow(clippy::too_many_arguments)]
    /// 根据脚本位置、错误码与原因构造标准 QL 异常。
    /// 对应 Java: `ExceptionFactory#newException`。
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
