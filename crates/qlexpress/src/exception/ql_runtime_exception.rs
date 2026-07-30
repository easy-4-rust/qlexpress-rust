//! 对应 Java 类：com.alibaba.qlexpress4.exception.QLRuntimeException
//!
//! 运行时异常构造辅助，对应 `QLExceptionKind::Runtime`。

use super::ql_exception::{QLException, QLExceptionKind};
use crate::runtime::value::DataValue;

/// QVM 执行期间产生并可携带脚本抛出值的运行时异常。
/// 对应或承接 Java 源文件：`com/alibaba/qlexpress4/exception/QLRuntimeException.java`；具体对象路径见 `docs/对象级对照表.md`。
/// Runtime error, mirroring Java `QLRuntimeException`.
///
/// The catchable attachment lives on [`QLException::catch_obj`]; this wrapper
/// exists so call sites can express the Java type relationship explicitly.
#[derive(Clone, Debug)]
/// 对应 Java: com.alibaba.qlexpress4.exception.QLRuntimeException。
pub struct QLRuntimeException {
    inner: QLException,
}

impl QLRuntimeException {
    /// 构造测试场景使用的实例。
    /// 参数：`catch_obj`、`reason`、`error_code`；返回：`Self`。
    /// 对应或承接 Java 源文件：`com/alibaba/qlexpress4/exception/QLRuntimeException.java`，方法 `forTest`；Rust 侧按所有权与 `Result` 语义适配。
    /// Mirrors the Java "Visible for test"
    /// `QLRuntimeException(catchObj, reason, errorCode)` constructor.
    /// 对应 Java: com.alibaba.qlexpress4.exception.QLRuntimeException#forTest。
    pub fn for_test(catch_obj: Option<DataValue>, reason: &str, error_code: &str) -> Self {
        let mut inner = QLException::for_test(QLExceptionKind::Runtime, reason, error_code);
        if let Some(obj) = catch_obj {
            inner = inner.with_catch_obj(obj);
        }
        QLRuntimeException { inner }
    }

    /// 返回内部通用 QL 异常。
    /// 对应 Java: `QLRuntimeException` 继承 `QLException` 后暴露的基类状态。
    pub fn inner(&self) -> &QLException {
        &self.inner
    }

    /// 返回交给脚本 `catch` 变量的异常对象。
    /// 对应 Java: `QLRuntimeException#getCatchObj`。
    pub fn catch_obj(&self) -> Option<&DataValue> {
        self.inner.catch_obj()
    }

    /// 将运行时异常消费并转换为通用 QL 异常。
    /// 对应 Java: `QLRuntimeException` 向 `QLException` 的继承转换。
    pub fn into_exception(self) -> QLException {
        self.inner
    }
}
