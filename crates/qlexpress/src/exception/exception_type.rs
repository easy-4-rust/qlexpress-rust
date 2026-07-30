//! 用户自定义异常类别。

/// 自定义函数或操作符报告的业务异常类别。
///
/// 对应 Java:
/// `com.alibaba.qlexpress4.exception.UserDefineException.ExceptionType`。
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ExceptionType {
    /// 非法参数业务异常。
    InvalidArgument,
    /// 通用业务异常。
    BizException,
}
