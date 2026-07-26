//! 对应 Java 类：com.alibaba.qlexpress4.operator.OperatorCheckStrategy
//!
//! 操作符限制策略接口，定义"某个操作符是否被允许"的契约。
//!
//! Java 是一个 `interface` 加三个实现（`Default`/`White`/`Black`）：
//! - `OperatorCheckStrategy.allowAll()` → `DefaultOperatorCheckStrategy`
//! - `OperatorCheckStrategy.whitelist(Set)` → `WhiteOperatorCheckStrategy`
//! - `OperatorCheckStrategy.blacklist(Set)` → `BlackOperatorCheckStrategy`
//!
//! Rust 侧同时提供：
//! - **enum 形态的 [`OperatorCheckStrategy`]**（一次性承载三种策略，
//!   供 `CheckOptions` 直接 `Clone`/`PartialEq`，与历史 API 兼容）；
//! - **三个具体实现**（一文件一对象，对齐 SPEC §2）：
//!   - [`crate::operator::DefaultOperatorCheckStrategy`]（`default_operator_check_strategy.rs`）
//!   - [`crate::operator::WhiteOperatorCheckStrategy`]（`white_operator_check_strategy.rs`）
//!   - [`crate::operator::BlackOperatorCheckStrategy`]（`black_operator_check_strategy.rs`）
//!
//! Java 接口注释列出的支持操作符分类（arithmetic / assign / bit / collection /
//! compare / logic / string / unary / root instanceof）在 Rust 侧由
//! `runtime/operator/` 子包逐类落地。

use std::collections::HashSet;

pub use super::black_operator_check_strategy::BlackOperatorCheckStrategy;
pub use super::default_operator_check_strategy::DefaultOperatorCheckStrategy;
pub use super::white_operator_check_strategy::WhiteOperatorCheckStrategy;

/// 枚举形态的策略入口，对齐 Java `OperatorCheckStrategy` 接口的三个静态工厂。
///
/// 三个变体分别对齐 Java：
/// - `AllowAll` ↔ `OperatorCheckStrategy.allowAll()`
/// - `Whitelist(set)` ↔ `OperatorCheckStrategy.whitelist(set)`
/// - `Blacklist(set)` ↔ `OperatorCheckStrategy.blacklist(set)`
///
/// 下游消费者（`CheckOptions` 等）直接持有该枚举做 `Clone`/`PartialEq`。
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub enum OperatorCheckStrategy {
    /// Java `OperatorCheckStrategy.allowAll()`。
    #[default]
    AllowAll,
    /// Java `OperatorCheckStrategy.whitelist(Set<String>)`：仅这些操作符被允许。
    Whitelist(HashSet<String>),
    /// Java `OperatorCheckStrategy.blacklist(Set<String>)`：这些操作符被禁止。
    Blacklist(HashSet<String>),
}

impl OperatorCheckStrategy {
    /// 处理 allow all 对应的领域职责。
    /// 无显式参数；返回：`Self`。
    /// 对应或承接 Java 源文件：`com/alibaba/qlexpress4/operator/OperatorCheckStrategy.java`，方法 `allowAll`；Rust 侧按所有权与 `Result` 语义适配。
    /// Java `OperatorCheckStrategy.allowAll()`。
    pub fn allow_all() -> Self {
        OperatorCheckStrategy::AllowAll
    }

    /// 处理 whitelist 对应的领域职责。
    /// 参数：`allowed_operators`；返回：`Self`。
    /// 对应或承接 Java 源文件：`com/alibaba/qlexpress4/operator/OperatorCheckStrategy.java`，方法 `whitelist`；Rust 侧按所有权与 `Result` 语义适配。
    /// Java `OperatorCheckStrategy.whitelist(Set<String>)`。
    pub fn whitelist(allowed_operators: HashSet<String>) -> Self {
        OperatorCheckStrategy::Whitelist(allowed_operators)
    }

    /// 处理 blacklist 对应的领域职责。
    /// 参数：`forbidden_operators`；返回：`Self`。
    /// 对应或承接 Java 源文件：`com/alibaba/qlexpress4/operator/OperatorCheckStrategy.java`，方法 `blacklist`；Rust 侧按所有权与 `Result` 语义适配。
    /// Java `OperatorCheckStrategy.blacklist(Set<String>)`。
    pub fn blacklist(forbidden_operators: HashSet<String>) -> Self {
        OperatorCheckStrategy::Blacklist(forbidden_operators)
    }

    /// Java `isAllowed(String)` —— 判断操作符是否被允许。
    pub fn is_allowed(&self, operator: &str) -> bool {
        match self {
            OperatorCheckStrategy::AllowAll => true,
            OperatorCheckStrategy::Whitelist(allowed) => allowed.contains(operator),
            OperatorCheckStrategy::Blacklist(forbidden) => !forbidden.contains(operator),
        }
    }

    /// Java `getOperators()`：返回配置集合；`AllowAll` 返回空集。
    pub fn operators(&self) -> &HashSet<String> {
        match self {
            OperatorCheckStrategy::AllowAll => DefaultOperatorCheckStrategy::empty_set(),
            OperatorCheckStrategy::Whitelist(ops) | OperatorCheckStrategy::Blacklist(ops) => ops,
        }
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
        assert_eq!(
            BlackOperatorCheckStrategy::new(set(&["="]))
                .operators()
                .len(),
            1
        );
    }

    #[test]
    fn enum_and_concrete_implementations_agree() {
        // AllowAll enum ↔ DefaultOperatorCheckStrategy
        let default_struct = DefaultOperatorCheckStrategy::instance();
        let default_enum = OperatorCheckStrategy::allow_all();
        for op in ["+", "-", "*", "/", "=", "in", "&&", "!", "<<"] {
            assert_eq!(
                default_enum.is_allowed(op),
                default_struct.is_allowed(op),
                "op={op}"
            );
        }

        // Whitelist enum ↔ WhiteOperatorCheckStrategy
        let white_struct = WhiteOperatorCheckStrategy::new(set(&["+", "*"]));
        let white_enum = OperatorCheckStrategy::whitelist(set(&["+", "*"]));
        for op in ["+", "*", "=", "-"] {
            assert_eq!(
                white_enum.is_allowed(op),
                white_struct.is_allowed(op),
                "op={op}"
            );
        }

        // Blacklist enum ↔ BlackOperatorCheckStrategy
        let black_struct = BlackOperatorCheckStrategy::new(set(&["="]));
        let black_enum = OperatorCheckStrategy::blacklist(set(&["="]));
        for op in ["=", "+", "-", "in"] {
            assert_eq!(
                black_enum.is_allowed(op),
                black_struct.is_allowed(op),
                "op={op}"
            );
        }
    }
}
