//! Operator restriction strategies, mirroring Java `OperatorCheckStrategy`
//! and its `Default` / `White` / `Black` implementations.

use std::collections::HashSet;

/// How script validation decides whether an operator is allowed, mirroring
/// Java `OperatorCheckStrategy`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum OperatorCheckStrategy {
    /// Java `OperatorCheckStrategy.allowAll()`.
    AllowAll,
    /// Java `OperatorCheckStrategy.whitelist(...)`: only these operators
    /// (Java operator text, e.g. `"+"`, `"in"`) are allowed.
    Whitelist(HashSet<String>),
    /// Java `OperatorCheckStrategy.blacklist(...)`: these operators are
    /// forbidden.
    Blacklist(HashSet<String>),
}

impl OperatorCheckStrategy {
    /// Java `OperatorCheckStrategy.allowAll()`.
    pub fn allow_all() -> Self {
        OperatorCheckStrategy::AllowAll
    }

    /// Java `OperatorCheckStrategy.whitelist(Set<String>)`.
    pub fn whitelist(allowed_operators: HashSet<String>) -> Self {
        OperatorCheckStrategy::Whitelist(allowed_operators)
    }

    /// Java `OperatorCheckStrategy.blacklist(Set<String>)`.
    pub fn blacklist(forbidden_operators: HashSet<String>) -> Self {
        OperatorCheckStrategy::Blacklist(forbidden_operators)
    }

    /// Java `isAllowed(String)`.
    pub fn is_allowed(&self, operator: &str) -> bool {
        match self {
            OperatorCheckStrategy::AllowAll => true,
            OperatorCheckStrategy::Whitelist(allowed) => allowed.contains(operator),
            OperatorCheckStrategy::Blacklist(forbidden) => !forbidden.contains(operator),
        }
    }

    /// Java `getOperators()` — the configured operator set; empty for
    /// allow-all.
    pub fn operators(&self) -> &HashSet<String> {
        match self {
            OperatorCheckStrategy::AllowAll => DefaultOperatorCheckStrategy::empty_set(),
            OperatorCheckStrategy::Whitelist(ops) | OperatorCheckStrategy::Blacklist(ops) => ops,
        }
    }
}

impl Default for OperatorCheckStrategy {
    fn default() -> Self {
        OperatorCheckStrategy::AllowAll
    }
}

/// Java `DefaultOperatorCheckStrategy` (allow-all singleton).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct DefaultOperatorCheckStrategy;

impl DefaultOperatorCheckStrategy {
    pub fn instance() -> Self {
        DefaultOperatorCheckStrategy
    }

    pub fn is_allowed(&self, _operator: &str) -> bool {
        true
    }

    pub(crate) fn empty_set() -> &'static HashSet<String> {
        static EMPTY: std::sync::OnceLock<HashSet<String>> = std::sync::OnceLock::new();
        EMPTY.get_or_init(HashSet::new)
    }
}

/// Java `WhiteOperatorCheckStrategy`.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct WhiteOperatorCheckStrategy {
    allowed_operators: HashSet<String>,
}

impl WhiteOperatorCheckStrategy {
    pub fn new(allowed_operators: HashSet<String>) -> Self {
        WhiteOperatorCheckStrategy { allowed_operators }
    }

    pub fn is_allowed(&self, operator: &str) -> bool {
        self.allowed_operators.contains(operator)
    }

    pub fn operators(&self) -> &HashSet<String> {
        &self.allowed_operators
    }
}

/// Java `BlackOperatorCheckStrategy`.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct BlackOperatorCheckStrategy {
    forbidden_operators: HashSet<String>,
}

impl BlackOperatorCheckStrategy {
    pub fn new(forbidden_operators: HashSet<String>) -> Self {
        BlackOperatorCheckStrategy { forbidden_operators }
    }

    pub fn is_allowed(&self, operator: &str) -> bool {
        !self.forbidden_operators.contains(operator)
    }

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
    fn allow_all_allows_everything() {
        assert!(OperatorCheckStrategy::allow_all().is_allowed("+"));
        assert!(DefaultOperatorCheckStrategy::instance().is_allowed("="));
    }

    #[test]
    fn whitelist_allows_only_listed() {
        let strategy = OperatorCheckStrategy::whitelist(set(&["+", "*"]));
        assert!(strategy.is_allowed("+"));
        assert!(!strategy.is_allowed("="));
        assert_eq!(strategy.operators().len(), 2);
    }

    #[test]
    fn blacklist_forbids_listed() {
        let strategy = OperatorCheckStrategy::blacklist(set(&["="]));
        assert!(!strategy.is_allowed("="));
        assert!(strategy.is_allowed("+"));
        assert_eq!(BlackOperatorCheckStrategy::new(set(&["="])).operators().len(), 1);
    }
}
