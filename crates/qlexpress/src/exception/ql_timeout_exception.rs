use super::ql_exception::{QLException, QLExceptionKind};
use crate::runtime::value::DataValue;

/// 脚本超过 `QLOptions.timeout_millis` 后报告的超时异常。
/// 对应或承接 Java 源文件：`com/alibaba/qlexpress4/exception/QLTimeoutException.java`；具体对象路径见 `docs/对象级对照表.md`。
/// Script-timeout error, mirroring Java `QLTimeoutException`.
#[derive(Clone, Debug)]
/// 对应 Java: com.alibaba.qlexpress4.exception.QLTimeoutException。
pub struct QLTimeoutException {
    inner: QLException,
}

impl QLTimeoutException {
    /// 构造测试场景使用的实例。
    /// 参数：`catch_obj`、`reason`、`error_code`；返回：`Self`。
    /// 对应或承接 Java 源文件：`com/alibaba/qlexpress4/exception/QLTimeoutException.java`，方法 `forTest`；Rust 侧按所有权与 `Result` 语义适配。
    /// Mirrors the Java "Visible for test"
    /// `QLTimeoutException(catchObj, reason, errorCode)` constructor.
    /// 对应 Java: com.alibaba.qlexpress4.exception.QLTimeoutException#forTest。
    pub fn for_test(catch_obj: Option<DataValue>, reason: &str, error_code: &str) -> Self {
        let mut inner = QLException::for_test(QLExceptionKind::Timeout, reason, error_code);
        if let Some(obj) = catch_obj {
            inner = inner.with_catch_obj(obj);
        }
        QLTimeoutException { inner }
    }

    /// 返回内部通用 QL 异常。
    /// 对应 Java: `QLTimeoutException` 继承 `QLException` 后暴露的基类状态。
    pub fn inner(&self) -> &QLException {
        &self.inner
    }

    /// 将超时异常消费并转换为通用 QL 异常。
    /// 对应 Java: `QLTimeoutException` 向 `QLException` 的继承转换。
    pub fn into_exception(self) -> QLException {
        self.inner
    }
}
