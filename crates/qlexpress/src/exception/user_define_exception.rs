use std::fmt;

pub use super::exception_type::ExceptionType;

/// 用户脚本或宿主扩展显式报告的业务异常包装。
/// 对应或承接 Java 源文件：`com/alibaba/qlexpress4/exception/UserDefineException.java`；具体对象路径见 `docs/对象级对照表.md`。
/// User-defined error message for custom functions/operators, mirroring Java
/// `UserDefineException`.
#[derive(Clone, Debug, PartialEq, Eq)]
/// 对应 Java: com.alibaba.qlexpress4.exception.UserDefineException。
pub struct UserDefineException {
    exception_type: Option<ExceptionType>,
    message: Option<String>,
}

impl UserDefineException {
    /// 创建对象实例。
    /// 参数：`message`；返回：`Self`。
    /// 对应或承接 Java 源文件：`com/alibaba/qlexpress4/exception/UserDefineException.java`，构造器 `<init>`；Rust 侧按所有权与 `Result` 语义适配。
    /// Java `UserDefineException(String)` — defaults to `BIZ_EXCEPTION`.
    /// 对应 Java: com.alibaba.qlexpress4.exception.UserDefineException#new。
    pub fn new(message: impl Into<String>) -> Self {
        Self::with_type(ExceptionType::BizException, message)
    }

    /// 附加 type 配置并返回新值。
    /// 参数：`exception_type`、`message`；返回：`Self`。
    /// 对应或承接 Java 源文件：`com/alibaba/qlexpress4/exception/UserDefineException.java`，方法 `withType`；Rust 侧按所有权与 `Result` 语义适配。
    /// Java `UserDefineException(ExceptionType, String)`.
    /// 对应 Java: com.alibaba.qlexpress4.exception.UserDefineException#withType。
    pub fn with_type(exception_type: ExceptionType, message: impl Into<String>) -> Self {
        UserDefineException {
            exception_type: Some(exception_type),
            message: Some(message.into()),
        }
    }

    /// 从 Java 构造器允许的可空类型和消息创建异常。
    /// 对应 Java: com.alibaba.qlexpress4.exception.UserDefineException#UserDefineException。
    pub fn from_options(exception_type: Option<ExceptionType>, message: Option<String>) -> Self {
        UserDefineException {
            exception_type,
            message,
        }
    }

    /// 返回脚本声明的用户异常类型。
    /// 对应 Java: `UserDefineException#getExceptionType`。
    pub fn exception_type(&self) -> Option<ExceptionType> {
        self.exception_type
    }

    /// 返回脚本声明的用户异常类型。
    ///
    /// 对应 Java：`UserDefineException#getType()`。
    ///
    /// # 返回值
    /// 返回构造异常时指定的 [`ExceptionType`]。
    pub fn get_type(&self) -> Option<ExceptionType> {
        self.exception_type
    }

    /// 返回用户异常消息。
    /// 对应 Java: `UserDefineException#getMessage`。
    pub fn message(&self) -> Option<&str> {
        self.message.as_deref()
    }
}

impl fmt::Display for UserDefineException {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(message) = &self.message {
            f.write_str(message)
        } else {
            Ok(())
        }
    }
}

impl std::error::Error for UserDefineException {}

#[cfg(test)]
mod tests {
    use super::*;

    /// `SOURCE_PARITY`：Java 单参数构造器默认使用 `BIZ_EXCEPTION`，显式
    /// 构造器与 `getType()` 保留指定类别。
    #[test]
    fn get_type_preserves_java_constructor_contract() {
        assert_eq!(
            UserDefineException::new("business").get_type(),
            Some(ExceptionType::BizException)
        );
        assert_eq!(
            UserDefineException::with_type(ExceptionType::InvalidArgument, "argument").get_type(),
            Some(ExceptionType::InvalidArgument)
        );
    }

    #[test]
    fn preserves_java_null_type_and_message() {
        let error = UserDefineException::from_options(None, None);
        assert_eq!(error.get_type(), None);
        assert_eq!(error.message(), None);
        assert_eq!(error.to_string(), "");
    }
}
