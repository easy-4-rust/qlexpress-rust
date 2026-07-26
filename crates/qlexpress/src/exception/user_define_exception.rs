use std::fmt;

/// User-defined error type for custom functions/operators, mirroring Java
/// `UserDefineException.ExceptionType`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ExceptionType {
    InvalidArgument,
    BizException,
}

/// User-defined error message for custom functions/operators, mirroring Java
/// `UserDefineException`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UserDefineException {
    exception_type: ExceptionType,
    message: String,
}

impl UserDefineException {
    /// Java `UserDefineException(String)` — defaults to `BIZ_EXCEPTION`.
    pub fn new(message: impl Into<String>) -> Self {
        Self::with_type(ExceptionType::BizException, message)
    }

    /// Java `UserDefineException(ExceptionType, String)`.
    pub fn with_type(exception_type: ExceptionType, message: impl Into<String>) -> Self {
        UserDefineException {
            exception_type,
            message: message.into(),
        }
    }

    /// 执行 `exception_type` 公开操作。对应 Java 源码 `com/alibaba/qlexpress4/exception/UserDefineException.java:1` 的 `UserDefineException`；该方法为 Rust 同职责适配接口。
    pub fn exception_type(&self) -> ExceptionType {
        self.exception_type
    }

    /// 执行 `message` 公开操作。对应 Java 源码 `com/alibaba/qlexpress4/exception/UserDefineException.java:1` 的 `UserDefineException`；该方法为 Rust 同职责适配接口。
    pub fn message(&self) -> &str {
        &self.message
    }
}

impl fmt::Display for UserDefineException {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for UserDefineException {}
