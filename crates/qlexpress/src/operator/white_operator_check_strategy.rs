//! 对应 Java 类：com.alibaba.qlexpress4.operator.WhiteOperatorCheckStrategy
//!
//! 白名单操作符检查策略，仅允许指定集合内的操作符；
//! 若 `allowedOperators` 为空，则**所有**操作符都被拒绝。
//!
//! 与 Java 实现的差异：Rust 用 [`HashSet<String>`] 替代 `Set<String>`，
//! 内部存所有权字符串；构造时通过 [`new`](WhiteOperatorCheckStrategy::new)
//! 转移所有权，对外提供 `operators()` 借用。

use std::collections::HashSet;

/// Java `WhiteOperatorCheckStrategy` —— 白名单策略。
///
/// 对应 Java：
/// ```java
/// public WhiteOperatorCheckStrategy(Set<String> allowedOperators) { ... }
/// public boolean isAllowed(String operator) { return allowedOperators.contains(operator); }
/// public Set<String> getOperators() { return allowedOperators; }
/// ```
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct WhiteOperatorCheckStrategy {
    allowed_operators: HashSet<String>,
}

impl WhiteOperatorCheckStrategy {
    /// Java 构造器 `WhiteOperatorCheckStrategy(Set<String>)`。
    ///
    /// 与 Java 一致：传入 `null` 时退化为空集；Rust 侧以 `Default` 表达。
    /// 对应 Java: com.alibaba.qlexpress4.operator.WhiteOperatorCheckStrategy#new。
    pub fn new(allowed_operators: HashSet<String>) -> Self {
        WhiteOperatorCheckStrategy { allowed_operators }
    }

    /// Java `isAllowed(String)` —— 操作符必须在白名单中才允许。
    /// 对应 Java: com.alibaba.qlexpress4.operator.WhiteOperatorCheckStrategy#isAllowed。
    pub fn is_allowed(&self, operator: &str) -> bool {
        self.allowed_operators.contains(operator)
    }

    /// Java `getOperators()` —— 返回白名单本身。
    /// 对应 Java: com.alibaba.qlexpress4.operator.WhiteOperatorCheckStrategy#operators。
    pub fn operators(&self) -> &HashSet<String> {
        &self.allowed_operators
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn set(items: &[&str]) -> HashSet<String> {
        items.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn whitelist_allows_only_listed() {
        let s = WhiteOperatorCheckStrategy::new(set(&["+", "*"]));
        assert!(s.is_allowed("+"));
        assert!(s.is_allowed("*"));
        assert!(!s.is_allowed("="));
    }

    #[test]
    fn empty_whitelist_rejects_all() {
        let s = WhiteOperatorCheckStrategy::new(HashSet::new());
        assert!(!s.is_allowed("+"));
        assert!(s.operators().is_empty());
    }
}
