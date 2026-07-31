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

/// 让闭包直接实现 Java 单抽象方法接口 `ExceptionFactory<T>`。
impl<T, F> ExceptionFactory<T> for F
where
    F: Fn(&str, i32, i32, &str, &str, &str, &str) -> T,
{
    fn new_exception(
        &self,
        message: &str,
        line_no: i32,
        col_no: i32,
        err_lexeme: &str,
        error_code: &str,
        reason: &str,
        snippet: &str,
    ) -> T {
        (self)(
            message, line_no, col_no, err_lexeme, error_code, reason, snippet,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// SOURCE_PARITY: Java `ExceptionFactory#newException` 的七个参数必须按
    /// 原顺序、不丢失地交给具体工厂。
    #[test]
    fn closure_receives_all_java_arguments_in_order() {
        let factory = |message: &str,
                       line_no: i32,
                       col_no: i32,
                       lexeme: &str,
                       code: &str,
                       reason: &str,
                       snippet: &str| {
            format!("{message}|{line_no}|{col_no}|{lexeme}|{code}|{reason}|{snippet}")
        };
        assert_eq!(
            factory.new_exception("m", 2, 3, "x", "E", "r", "s"),
            "m|2|3|x|E|r|s"
        );
    }
}
