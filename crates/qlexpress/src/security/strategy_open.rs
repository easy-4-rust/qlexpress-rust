//! 全放行安全策略,对应 Java `com.alibaba.qlexpress4.security.StrategyOpen`。
//! 职责:允许脚本访问应用内所有 Java(Rust 本地)成员。

use super::ql_security_strategy::{NativeMember, QLSecurityStrategy};

/// 全放行安全策略。对应 Java: com.alibaba.qlexpress4.security.StrategyOpen
/// (A security policy that allows access to all Java classes within the application.)
///
/// Java 版为单例(`StrategyOpen.getInstance()`);Rust 版为无状态单元结构体,
/// 提供 [`StrategyOpen::instance`] 对应 Java 的 `getInstance()`。
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct StrategyOpen;

impl StrategyOpen {
    /// 单例实例。对应 Java 静态方法 `StrategyOpen.getInstance()`。
    pub fn instance() -> Self {
        StrategyOpen
    }

    /// 检查成员是否安全:恒为 `true`(全部放行)。
    /// 对应 Java 方法 `check(Member)`(实现 `return true`)。
    pub fn check(&self, _member: Option<&NativeMember>) -> bool {
        true
    }
}

/// 转为等价的外观枚举 [`QLSecurityStrategy::Open`]。
/// (Java 中 `StrategyOpen` 直接实现 `QLSecurityStrategy` 接口。)
impl From<StrategyOpen> for QLSecurityStrategy {
    fn from(_strategy: StrategyOpen) -> Self {
        QLSecurityStrategy::Open
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn allows_everything() {
        let strategy = StrategyOpen::instance();
        // 任意成员(含危险成员)均放行
        let runtime_exec = NativeMember::new("java.lang.Runtime", "exec");
        let system_exit = NativeMember::new("java.lang.System", "exit");
        assert!(strategy.check(Some(&runtime_exec)));
        assert!(strategy.check(Some(&system_exit)));
        assert!(strategy.check(None));
        let as_enum: QLSecurityStrategy = strategy.into();
        assert_eq!(as_enum, QLSecurityStrategy::open());
    }
}
