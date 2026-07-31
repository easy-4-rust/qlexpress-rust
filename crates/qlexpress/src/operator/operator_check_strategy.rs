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

use std::cell::RefCell;
use std::collections::HashSet;
use std::fmt;
use std::rc::Rc;

pub use super::black_operator_check_strategy::BlackOperatorCheckStrategy;
pub use super::default_operator_check_strategy::DefaultOperatorCheckStrategy;
pub use super::white_operator_check_strategy::WhiteOperatorCheckStrategy;

/// 在宿主、检查选项和静态检查器之间共享的操作符集合。
pub type SharedOperators = Rc<RefCell<HashSet<String>>>;

/// 枚举形态的策略入口，对齐 Java `OperatorCheckStrategy` 接口的三个静态工厂。
///
/// 三个变体分别对齐 Java：
/// - `AllowAll` ↔ `OperatorCheckStrategy.allowAll()`
/// - `Whitelist(set)` ↔ `OperatorCheckStrategy.whitelist(set)`
/// - `Blacklist(set)` ↔ `OperatorCheckStrategy.blacklist(set)`
///
/// 下游消费者（`CheckOptions` 等）直接持有该枚举做 `Clone`/`PartialEq`。
#[derive(Clone)]
/// 对应 Java: com.alibaba.qlexpress4.operator.OperatorCheckStrategy。
pub enum OperatorCheckStrategy {
    /// Java `OperatorCheckStrategy.allowAll()`。
    AllowAll,
    /// Java `OperatorCheckStrategy.whitelist(Set<String>)`：仅这些操作符被允许。
    Whitelist(HashSet<String>),
    /// Java `OperatorCheckStrategy.blacklist(Set<String>)`：这些操作符被禁止。
    Blacklist(HashSet<String>),
    /// 保存调用方集合引用的 Java 白名单语义。
    SharedWhitelist(SharedOperators),
    /// 保存调用方集合引用的 Java 黑名单语义。
    SharedBlacklist(SharedOperators),
    /// 业务宿主实现 Java `OperatorCheckStrategy` 的 Rust 闭包适配。
    Custom {
        /// 判断指定操作符是否允许使用的宿主回调。
        check: Rc<dyn Fn(&str) -> bool>,
        /// 策略公开的操作符集合快照。
        operators: HashSet<String>,
    },
}

impl Default for OperatorCheckStrategy {
    fn default() -> Self {
        Self::AllowAll
    }
}

impl fmt::Debug for OperatorCheckStrategy {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::AllowAll => formatter.write_str("AllowAll"),
            Self::Whitelist(operators) => {
                formatter.debug_tuple("Whitelist").field(operators).finish()
            }
            Self::Blacklist(operators) => {
                formatter.debug_tuple("Blacklist").field(operators).finish()
            }
            Self::SharedWhitelist(operators) => formatter
                .debug_tuple("SharedWhitelist")
                .field(&operators.borrow())
                .finish(),
            Self::SharedBlacklist(operators) => formatter
                .debug_tuple("SharedBlacklist")
                .field(&operators.borrow())
                .finish(),
            Self::Custom { operators, .. } => formatter
                .debug_struct("Custom")
                .field("operators", operators)
                .finish_non_exhaustive(),
        }
    }
}

impl PartialEq for OperatorCheckStrategy {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::AllowAll, Self::AllowAll) => true,
            (Self::Whitelist(left), Self::Whitelist(right))
            | (Self::Blacklist(left), Self::Blacklist(right)) => left == right,
            (Self::SharedWhitelist(left), Self::SharedWhitelist(right))
            | (Self::SharedBlacklist(left), Self::SharedBlacklist(right)) => {
                Rc::ptr_eq(left, right) || *left.borrow() == *right.borrow()
            }
            (
                Self::Custom {
                    check: left_check,
                    operators: left_operators,
                },
                Self::Custom {
                    check: right_check,
                    operators: right_operators,
                },
            ) => Rc::ptr_eq(left_check, right_check) && left_operators == right_operators,
            _ => false,
        }
    }
}

impl Eq for OperatorCheckStrategy {}

impl OperatorCheckStrategy {
    /// 创建允许全部操作符的默认检查策略。
    /// 无显式参数；返回：`Self`。
    /// 对应或承接 Java 源文件：`com/alibaba/qlexpress4/operator/OperatorCheckStrategy.java`，方法 `allowAll`；Rust 侧按所有权与 `Result` 语义适配。
    /// Java `OperatorCheckStrategy.allowAll()`。
    /// 对应 Java: com.alibaba.qlexpress4.operator.OperatorCheckStrategy#allowAll。
    pub fn allow_all() -> Self {
        OperatorCheckStrategy::AllowAll
    }

    /// 创建仅允许指定操作符的白名单策略。
    /// 参数：`allowed_operators`；返回：`Self`。
    /// 对应或承接 Java 源文件：`com/alibaba/qlexpress4/operator/OperatorCheckStrategy.java`，方法 `whitelist`；Rust 侧按所有权与 `Result` 语义适配。
    /// Java `OperatorCheckStrategy.whitelist(Set<String>)`。
    /// 对应 Java: com.alibaba.qlexpress4.operator.OperatorCheckStrategy#whitelist。
    pub fn whitelist(allowed_operators: HashSet<String>) -> Self {
        OperatorCheckStrategy::Whitelist(allowed_operators)
    }

    /// 创建拒绝指定操作符的黑名单策略。
    /// 参数：`forbidden_operators`；返回：`Self`。
    /// 对应或承接 Java 源文件：`com/alibaba/qlexpress4/operator/OperatorCheckStrategy.java`，方法 `blacklist`；Rust 侧按所有权与 `Result` 语义适配。
    /// Java `OperatorCheckStrategy.blacklist(Set<String>)`。
    /// 对应 Java: com.alibaba.qlexpress4.operator.OperatorCheckStrategy#blacklist。
    pub fn blacklist(forbidden_operators: HashSet<String>) -> Self {
        OperatorCheckStrategy::Blacklist(forbidden_operators)
    }

    /// 使用调用方保留的共享集合创建白名单策略。
    ///
    /// Java `WhiteOperatorCheckStrategy` 保存
    /// `Collections.unmodifiableSet(allowedOperators)` 视图，因此调用方对
    /// 原集合的后续修改仍然可见。
    ///
    /// # 参数
    ///
    /// - `allowed_operators`：宿主与检查策略共享的操作符集合。
    ///
    /// # 返回值
    ///
    /// 返回具有 Java backing-set 引用语义的白名单策略。
    /// 对应 Java：`OperatorCheckStrategy#whitelist(Set<String>)` 的 backing-set 语义。
    pub fn shared_whitelist(allowed_operators: SharedOperators) -> Self {
        Self::SharedWhitelist(allowed_operators)
    }

    /// 使用调用方保留的共享集合创建黑名单策略。
    ///
    /// # 参数
    ///
    /// - `forbidden_operators`：宿主与检查策略共享的操作符集合。
    ///
    /// # 返回值
    ///
    /// 返回具有 Java backing-set 引用语义的黑名单策略。
    /// 对应 Java：`OperatorCheckStrategy#blacklist(Set<String>)` 的 backing-set 语义。
    pub fn shared_blacklist(forbidden_operators: SharedOperators) -> Self {
        Self::SharedBlacklist(forbidden_operators)
    }

    /// 使用宿主闭包创建自定义操作符检查策略。
    ///
    /// Java `OperatorCheckStrategy` 是公开接口，宿主可以从动态配置决定每个
    /// 操作符是否允许；闭包捕获的共享状态会被已构造的 [`CheckOptions`]
    /// 持续观察。
    ///
    /// # 参数
    ///
    /// - `check`：接收操作符文本并返回是否允许。
    /// - `operators`：用于诊断和策略展示的相关操作符集合。
    ///
    /// # 返回值
    ///
    /// 返回共享自定义检查逻辑的策略。
    /// 对应 Java：业务实现 `OperatorCheckStrategy#isAllowed(String)`。
    pub fn custom<F>(check: F, operators: HashSet<String>) -> Self
    where
        F: Fn(&str) -> bool + 'static,
    {
        Self::Custom {
            check: Rc::new(check),
            operators,
        }
    }

    /// Java `isAllowed(String)` —— 判断操作符是否被允许。
    /// 对应 Java: com.alibaba.qlexpress4.operator.OperatorCheckStrategy#isAllowed。
    pub fn is_allowed(&self, operator: &str) -> bool {
        match self {
            OperatorCheckStrategy::AllowAll => true,
            OperatorCheckStrategy::Whitelist(allowed) => allowed.contains(operator),
            OperatorCheckStrategy::Blacklist(forbidden) => !forbidden.contains(operator),
            OperatorCheckStrategy::SharedWhitelist(allowed) => allowed.borrow().contains(operator),
            OperatorCheckStrategy::SharedBlacklist(forbidden) => {
                !forbidden.borrow().contains(operator)
            }
            OperatorCheckStrategy::Custom { check, .. } => check(operator),
        }
    }

    /// Java `getOperators()`：返回配置集合；`AllowAll` 返回空集。
    ///
    /// Rust 返回当前快照，避免把 `RefCell` 的动态借用暴露给调用方；
    /// 每次调用仍会读取 Java backing set 的最新内容。
    /// 对应 Java: com.alibaba.qlexpress4.operator.OperatorCheckStrategy#operators。
    pub fn operators(&self) -> HashSet<String> {
        match self {
            OperatorCheckStrategy::AllowAll => HashSet::new(),
            OperatorCheckStrategy::Whitelist(ops)
            | OperatorCheckStrategy::Blacklist(ops)
            | OperatorCheckStrategy::Custom { operators: ops, .. } => ops.clone(),
            OperatorCheckStrategy::SharedWhitelist(ops)
            | OperatorCheckStrategy::SharedBlacklist(ops) => ops.borrow().clone(),
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

    #[test]
    fn custom_strategy_preserves_java_interface_extensibility() {
        let allow_plus = Rc::new(std::cell::Cell::new(false));
        let captured = Rc::clone(&allow_plus);
        let strategy = OperatorCheckStrategy::custom(
            move |operator| operator != "+" || captured.get(),
            set(&["+"]),
        );

        assert!(!strategy.is_allowed("+"));
        allow_plus.set(true);
        assert!(strategy.is_allowed("+"));
        assert_eq!(strategy.operators(), set(&["+"]));
        assert_eq!(strategy.clone(), strategy);
    }

    #[test]
    fn shared_strategy_observes_backing_set_mutation() {
        let operators = Rc::new(RefCell::new(set(&["+"])));
        let strategy = OperatorCheckStrategy::shared_whitelist(Rc::clone(&operators));

        assert!(strategy.is_allowed("+"));
        assert!(!strategy.is_allowed("*"));
        operators.borrow_mut().insert("*".to_string());
        assert!(strategy.is_allowed("*"));
        assert_eq!(strategy.operators(), set(&["+", "*"]));
    }
}
