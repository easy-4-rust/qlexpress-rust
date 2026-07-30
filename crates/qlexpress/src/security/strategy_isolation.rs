//! 隔离安全策略,对应 Java `com.alibaba.qlexpress4.security.StrategyIsolation`。
//! 职责:将 QLExpress 脚本与 JVM(Rust 宿主)完全隔离,禁止一切成员访问。

use super::ql_security_strategy::{NativeMember, QLSecurityStrategy};
use crate::exception::{QLException, QLExceptionKind};

const ILLEGAL_STATE_EXCEPTION: &str = "java.lang.IllegalStateException";

/// 隔离安全策略。对应 Java: com.alibaba.qlexpress4.security.StrategyIsolation
/// (A security policy that isolates qlexpress script with jvm.)
///
/// 语义要点:Java 版 `check(Member)` 直接 `throw new IllegalStateException()`
/// ——隔离策略下根本不允许走到成员检查这一步(在更上层就已拒绝),
/// 一旦走到即属引擎内部错误。Rust 版将 Java 的
/// `IllegalStateException` 映射为 [`QLException`]，避免公开 API 因宿主
/// 误用而终止进程。
/// 需要「默认拒绝、返回 false」的可组合语义时,请使用外观枚举
/// [`QLSecurityStrategy::Isolation`] 的 `check`(返回 `false`)。
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct StrategyIsolation;

impl StrategyIsolation {
    /// 单例实例。对应 Java 静态方法 `StrategyIsolation.getInstance()`。
    pub fn instance() -> Self {
        StrategyIsolation
    }

    /// 检查成员是否安全。对应 Java 方法 `check(Member)`。
    ///
    /// # 参数
    ///
    /// - `member`：待检查的宿主成员；Java 实现不会读取该参数。
    ///
    /// # 返回值
    ///
    /// Java 实现不会正常返回。
    ///
    /// # 错误
    ///
    /// 始终返回 `java.lang.IllegalStateException`，对应 Java 方法无条件抛出
    /// 同类异常；该错误表示隔离策略被错误地用于成员解析路径。
    pub fn check(&self, _member: &NativeMember) -> Result<bool, QLException> {
        // Java: throw new IllegalStateException();
        Err(QLException::host_error(
            QLExceptionKind::Runtime,
            ILLEGAL_STATE_EXCEPTION,
            ILLEGAL_STATE_EXCEPTION,
        ))
    }
}

/// 转为等价的外观枚举 [`QLSecurityStrategy::Isolation`]。
impl From<StrategyIsolation> for QLSecurityStrategy {
    fn from(_strategy: StrategyIsolation) -> Self {
        QLSecurityStrategy::Isolation
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn check_returns_java_illegal_state_exception() {
        // Java: check 抛 IllegalStateException；Rust 以 Result 保留异常类别。
        let error = StrategyIsolation::instance()
            .check(&NativeMember::new("java.lang.String", "length"))
            .expect_err("isolation strategy must reject direct member checks");
        assert_eq!(error.kind(), QLExceptionKind::Runtime);
        assert_eq!(error.error_code(), ILLEGAL_STATE_EXCEPTION);
        assert_eq!(error.reason(), ILLEGAL_STATE_EXCEPTION);
    }

    #[test]
    fn facade_enum_isolation_denies() {
        let as_enum: QLSecurityStrategy = StrategyIsolation::instance().into();
        assert_eq!(as_enum, QLSecurityStrategy::isolation());
        // 外观枚举语义:默认拒绝(不 panic)
        assert!(!as_enum.check(&NativeMember::new("java.lang.String", "length")));
    }
}
