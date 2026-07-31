use super::ql_exception::{QLException, QLExceptionKind};

/// 解析或静态检查失败时携带源码位置的语法异常。
/// 对应或承接 Java 源文件：`com/alibaba/qlexpress4/exception/QLSyntaxException.java`；具体对象路径见 `docs/对象级对照表.md`。
/// Syntax-phase error, mirroring Java `QLSyntaxException`.
///
/// Wrapper around the unified [`QLException`] with
/// [`QLExceptionKind::Syntax`]; convert with [`Self::into_exception`].
#[derive(Clone, Debug)]
/// 对应 Java: com.alibaba.qlexpress4.exception.QLSyntaxException。
pub struct QLSyntaxException {
    inner: QLException,
}

impl QLSyntaxException {
    /// Wrap an already-built [`QLException`] (must have `Syntax` kind).
    /// 对应 Java: com.alibaba.qlexpress4.exception.QLSyntaxException#fromException。
    pub(crate) fn from_exception(inner: QLException) -> Self {
        debug_assert_eq!(inner.kind(), QLExceptionKind::Syntax);
        QLSyntaxException { inner }
    }

    /// 返回内部通用 QL 异常。
    /// 对应 Java: `QLSyntaxException` 继承 `QLException` 后暴露的基类状态。
    pub fn inner(&self) -> &QLException {
        &self.inner
    }

    /// 将语法异常消费并转换为通用 QL 异常。
    /// 对应 Java: `QLSyntaxException` 向 `QLException` 的继承转换。
    pub fn into_exception(self) -> QLException {
        self.inner
    }
}

impl std::ops::Deref for QLSyntaxException {
    type Target = QLException;

    /// Transparent access to the wrapped [`QLException`] diagnostics
    /// (`error_code()`, `line_no()`, `col_no()`, `reason()`, ...).
    fn deref(&self) -> &QLException {
        &self.inner
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::exception::error_codes;

    /// SOURCE_PARITY: Java 受保护构造器把 message 与 Diagnostic 原样交给
    /// `QLException`；Rust 包装必须保留相同的基类可观察状态与 Syntax 类型。
    #[test]
    fn wrapper_preserves_java_superclass_state() {
        let syntax = QLException::report_scanner_err(
            "a +",
            2,
            1,
            3,
            "<EOF>",
            error_codes::SYNTAX_ERROR,
            "unexpected eof",
        );
        assert_eq!(syntax.kind(), QLExceptionKind::Syntax);
        assert_eq!(syntax.inner().reason(), "unexpected eof");
        assert_eq!(syntax.error_code(), error_codes::SYNTAX_ERROR);
        assert_eq!(syntax.line_no(), 1);
        assert_eq!(syntax.col_no(), 3);

        let inner = syntax.into_exception();
        assert_eq!(inner.kind(), QLExceptionKind::Syntax);
        assert_eq!(inner.reason(), "unexpected eof");
    }
}
