//! 对应 Java 类：com.alibaba.qlexpress4.operator.DefaultOperatorCheckStrategy
//!
//! 默认（允许全部）操作符检查策略，对应 Java 单例 `getInstance()`。
//! 在 QLExpress4 中 `OperatorCheckStrategy.allowAll()` 即返回该单例，
//! `isAllowed` 恒为 `true`、`getOperators` 返回空集。
//!
//! 与 Java 实现的差异：Rust 侧 `DefaultOperatorCheckStrategy` 为零成本单元结构体，
//! 通过 `instance()` 工厂返回 `Copy` 实例；同时由
//! [`crate::operator::operator_check_strategy::OperatorCheckStrategy::AllowAll`]
//! 枚举变体在 `OperatorCheckStrategy` 入口处对外暴露。

use std::collections::HashSet;

/// Java `DefaultOperatorCheckStrategy` —— allow-all 单例。
///
/// 对应 Java：
/// ```java
/// public static final DefaultOperatorCheckStrategy INSTANCE = new DefaultOperatorCheckStrategy();
/// public boolean isAllowed(String operator) { return true; }
/// public Set<String> getOperators() { return Collections.emptySet(); }
/// ```
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct DefaultOperatorCheckStrategy;

impl DefaultOperatorCheckStrategy {
    /// 处理 instance 对应的领域职责。
    /// 无显式参数；返回：`Self`。
    /// 对应或承接 Java 源文件：`com/alibaba/qlexpress4/operator/DefaultOperatorCheckStrategy.java`，方法 `instance`；Rust 侧按所有权与 `Result` 语义适配。
    /// Java `DefaultOperatorCheckStrategy.getInstance()`。
    pub fn instance() -> Self {
        DefaultOperatorCheckStrategy
    }

    /// Java `isAllowed(String)` —— 永远返回 `true`。
    pub fn is_allowed(&self, _operator: &str) -> bool {
        true
    }

    /// Java `getOperators()` —— 永远返回空集。
    pub fn operators(&self) -> &'static HashSet<String> {
        empty_set()
    }

    /// 供 [`crate::operator::operator_check_strategy::OperatorCheckStrategy::operators`]
    /// 在 `AllowAll` 分支共享同一空集。
    pub(crate) fn empty_set() -> &'static HashSet<String> {
        empty_set()
    }
}

fn empty_set() -> &'static HashSet<String> {
    static EMPTY: std::sync::OnceLock<HashSet<String>> = std::sync::OnceLock::new();
    EMPTY.get_or_init(HashSet::new)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_allows_everything_and_has_empty_set() {
        let strategy = DefaultOperatorCheckStrategy::instance();
        assert!(strategy.is_allowed("+"));
        assert!(strategy.is_allowed("="));
        assert!(strategy.is_allowed("in"));
        assert!(strategy.operators().is_empty());
    }

    #[test]
    fn instance_is_copy_singleton() {
        let a = DefaultOperatorCheckStrategy::instance();
        let b = a; // Copy
        assert_eq!(a, b);
    }
}
