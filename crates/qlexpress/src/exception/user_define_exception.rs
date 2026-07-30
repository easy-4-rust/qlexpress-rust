use std::fmt;

/// `ExceptionType` 枚举的 Rust 实现，保留对应对象的领域职责与公开契约。
/// 对应或承接 Java 源文件：`com/alibaba/qlexpress4/exception/UserDefineException.java`；具体对象路径见 `docs/对象级对照表.md`。
/// User-defined error type for custom functions/operators, mirroring Java
/// `UserDefineException.ExceptionType`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
/// 对应 Java: com.alibaba.qlexpress4.exception.UserDefineException。
pub enum ExceptionType {
    /// 非法参数业务异常。
    InvalidArgument,
    /// 通用业务异常。
    BizException,
}

/// `UserDefineException` 结构体的 Rust 实现，保留对应对象的领域职责与公开契约。
/// 对应或承接 Java 源文件：`com/alibaba/qlexpress4/exception/UserDefineException.java`；具体对象路径见 `docs/对象级对照表.md`。
/// User-defined error message for custom functions/operators, mirroring Java
/// `UserDefineException`.
#[derive(Clone, Debug, PartialEq, Eq)]
/// 对应 Java: com.alibaba.qlexpress4.exception.UserDefineException。
pub struct UserDefineException {
    exception_type: ExceptionType,
    message: String,
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
            exception_type,
            message: message.into(),
        }
    }

    /// 返回脚本声明的用户异常类型。
    /// 对应 Java: `UserDefineException#getExceptionType`。
    pub fn exception_type(&self) -> ExceptionType {
        self.exception_type
    }

    /// 返回用户异常消息。
    /// 对应 Java: `UserDefineException#getMessage`。
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
