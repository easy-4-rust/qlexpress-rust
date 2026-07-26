//! 编译缓存序列化异常,对应 Java `com.alibaba.qlexpress4.api.parsecache.SerializableParseCacheException`。
//! 职责:导出/导入编译缓存时的错误(携带脚本位置与 LSP Diagnostic)。

use crate::exception::ex_message_util::ExMessageUtil;
use crate::exception::lsp::{Diagnostic, Position, Range};
use crate::exception::ql_exception::{QLException, QLExceptionKind};

use super::serializable_source::SerializableSource;

/// 编译缓存序列化异常。对应 Java: com.alibaba.qlexpress4.api.parsecache.SerializableParseCacheException
/// (extends `QLException`)。
///
/// Rust 以「持有 [`QLException`] 的 newtype」对应 Java 的继承;
/// `Deref`/`into_ql_exception` 暴露基类全部行为。kind 固定为
/// [`QLExceptionKind::Runtime`](Rust 错误体系的最近对应,见 SPEC §3.4)。
#[derive(Clone, Debug)]
pub struct SerializableParseCacheException {
    inner: QLException,
}

impl SerializableParseCacheException {
    /// 构造异常。对应 Java 构造器
    /// `SerializableParseCacheException(String script, SerializableSource source,
    /// String errorCode, String reason)`。
    pub fn new(
        script: Option<&str>,
        source: Option<&SerializableSource>,
        error_code: &str,
        reason: &str,
    ) -> Self {
        let normalized = normalize_source(script, source);
        let script_str = script.unwrap_or("");
        let ex_message = ExMessageUtil::format(
            script_str,
            normalized.start,
            normalized.line,
            normalized.col + 1,
            normalized.lexeme.as_deref().unwrap_or(""),
            error_code,
            reason,
        );
        // Java buildDiagnostic:行为 0 基(line - 1),列保持 0 基
        let lexeme = normalized.lexeme.clone().unwrap_or_default();
        let lexeme_len = lexeme.chars().count() as i32;
        let start = Position::new(normalized.line - 1, normalized.col);
        let end = Position::new(normalized.line - 1, normalized.col + lexeme_len);
        let diagnostic = Diagnostic::new(
            normalized.start,
            Range::new(start, end),
            lexeme,
            error_code,
            reason,
            ex_message.snippet(),
        );
        SerializableParseCacheException {
            inner: QLException::new(
                QLExceptionKind::Runtime,
                ex_message.message(),
                diagnostic,
                None,
            ),
        }
    }

    /// 取出内部 [`QLException`](Java 多态持有的等价物)。
    pub fn into_ql_exception(self) -> QLException {
        self.inner
    }
}

/// 对应 Java 私有静态方法 `normalizeSource`:
/// start 截断到 `[0, script.length]`;line <= 0 归一为 1;col < 0 归一为 0;
/// lexeme 为 null 归一为空串。
fn normalize_source(
    script: Option<&str>,
    source: Option<&SerializableSource>,
) -> SerializableSource {
    let script_length = script.map(|s| s.chars().count() as i32).unwrap_or(0);
    let start = source.map(|s| s.start).unwrap_or(0);
    SerializableSource {
        start: start.clamp(0, script_length),
        line: match source {
            Some(s) if s.line > 0 => s.line,
            _ => 1,
        },
        col: source.map(|s| s.col.max(0)).unwrap_or(0),
        lexeme: source
            .and_then(|s| s.lexeme.clone())
            .or_else(|| Some(String::new())),
    }
}

impl std::ops::Deref for SerializableParseCacheException {
    type Target = QLException;

    fn deref(&self) -> &QLException {
        &self.inner
    }
}

impl std::fmt::Display for SerializableParseCacheException {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.inner.fmt(f)
    }
}

impl std::error::Error for SerializableParseCacheException {}
