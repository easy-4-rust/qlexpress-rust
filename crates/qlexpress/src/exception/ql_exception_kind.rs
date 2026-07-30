//! QlExpress Rust 统一异常的类别判别。

/// Java 异常子类在 Rust 统一错误模型中的类别。
///
/// 对应 Java: `QLSyntaxException`、`QLRuntimeException` 与
/// `QLTimeoutException` 的继承层级。
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum QLExceptionKind {
    /// 解析或词法阶段错误。
    Syntax,
    /// 运行阶段错误，可携带脚本可捕获对象。
    Runtime,
    /// 脚本执行超过超时限制。
    Timeout,
}
