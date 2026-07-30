//! 格式化异常消息及其源码片段。

/// 格式化错误消息以及从脚本中截取的定位片段。
///
/// 对应 Java: `com.alibaba.qlexpress4.exception.ExMessageUtil.ExMessage`。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExMessage {
    pub(crate) message: String,
    pub(crate) snippet: String,
}
