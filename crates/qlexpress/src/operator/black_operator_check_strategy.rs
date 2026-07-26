//! 对应 Java 类：com.alibaba.qlexpress4.operator.BlackOperatorCheckStrategy
//!
//! 黑名单操作符检查策略，禁止指定集合内的操作符；
//! 若 `forbiddenOperators` 为空，则**所有**操作符都被允许。
//!
//! 与 Java 实现的差异：Rust 用 [`HashSet<String>`] 替代 `Set<String>`，
//! 内部存所有权字符串；构造时通过 [`new`](BlackOperatorCheckStrategy::new)
//! 转移所有权，对外提供 `operators()` 借用。

use std::collections::HashSet;

/// Java `BlackOperatorCheckStrategy` —— 黑名单策略。
///
/// 对应 Java：
/// ```java
/// public BlackOperatorCheckStrategy(Set<String> blackOperators) { ... }
/// public boolean isAllowed(String operator) { return !blackOperators.contains(operator); }
/// public Set<String> getOperators() { return blackOperators; }
/// ```
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct BlackOperatorCheckStrategy {
    forbidden_operators: HashSet<String>,
}

impl BlackOperatorCheckStrategy {
    /// Java 构造器 `BlackOperatorCheckStrategy(Set<String>)`。
    pub fn new(forbidden_operators: HashSet<String>) -> Self {
        BlackOperatorCheckStrategy {
            forbidden_operators,
        }
    }

    /// Java `isAllowed(String)` —— 操作符不在黑名单中即允许。
    pub fn is_allowed(&self, operator: &str) -> bool {
        !self.forbidden_operators.contains(operator)
    }

    /// Java `getOperators()` —— 返回黑名单本身。
    pub fn operators(&self) -> &HashSet<String> {
        &self.forbidden_operators
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn set(items: &[&str]) -> HashSet<String> {
        items.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn blacklist_forbids_only_listed() {
        let s = BlackOperatorCheckStrategy::new(set(&["="]));
        assert!(!s.is_allowed("="));
        assert!(s.is_allowed("+"));
        assert_eq!(s.operators().len(), 1);
    }

    #[test]
    fn empty_blacklist_allows_all() {
        let s = BlackOperatorCheckStrategy::new(HashSet::new());
        assert!(s.is_allowed("+"));
        assert!(s.is_allowed("="));
    }
}
