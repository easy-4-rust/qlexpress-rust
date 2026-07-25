//! 隔离安全策略,对应 Java `com.alibaba.qlexpress4.security.StrategyIsolation`。
//! 职责:将 QLExpress 脚本与 JVM(Rust 宿主)完全隔离,禁止一切成员访问。

use super::ql_security_strategy::{NativeMember, QLSecurityStrategy};

/// 隔离安全策略。对应 Java: com.alibaba.qlexpress4.security.StrategyIsolation
/// (A security policy that isolates qlexpress script with jvm.)
///
/// 语义要点:Java 版 `check(Member)` 直接 `throw new IllegalStateException()`
/// ——隔离策略下根本不允许走到成员检查这一步(在更上层就已拒绝),
/// 一旦走到即属引擎内部错误。Rust 版以 `panic!` 对应 Java 的
/// `IllegalStateException`(同为「不应发生」的运行时错误)。
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
    /// Java 语义:无条件 `throw new IllegalStateException()`;Rust 以 panic 对应。
    pub fn check(&self, _member: &NativeMember) -> bool {
        // Java: throw new IllegalStateException();
        panic!("StrategyIsolation forbids any member access check (java.lang.IllegalStateException)")
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
    #[should_panic(expected = "StrategyIsolation")]
    fn check_panics_like_java_illegal_state() {
        // Java: check 抛 IllegalStateException
        StrategyIsolation::instance().check(&NativeMember::new("java.lang.String", "length"));
    }

    #[test]
    fn facade_enum_isolation_denies() {
        let as_enum: QLSecurityStrategy = StrategyIsolation::instance().into();
        assert_eq!(as_enum, QLSecurityStrategy::isolation());
        // 外观枚举语义:默认拒绝(不 panic)
        assert!(!as_enum.check(&NativeMember::new("java.lang.String", "length")));
    }
}
