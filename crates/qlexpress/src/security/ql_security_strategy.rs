//! Security strategy, mirroring Java `QLSecurityStrategy`.
//!
//! Java checks `java.lang.reflect.Member` objects; Rust native members are
//! registered explicitly (SPEC §4/§6), so members are identified by
//! `NativeMember` descriptors (`type_name.member_name`).

use std::cell::RefCell;
use std::collections::HashSet;
use std::fmt;
use std::rc::Rc;

pub use super::native_member::NativeMember;

/// 在宿主、安全策略和 runner 之间共享的原生成员集合。
pub type SharedNativeMembers = Rc<RefCell<HashSet<NativeMember>>>;

impl NativeMember {
    /// 创建用于安全策略匹配的“类型名 + 成员名”描述符。
    /// 对应 Java: `java.lang.reflect.Member` 提供给 `QLSecurityStrategy#check` 的身份信息。
    pub fn new(type_name: impl Into<String>, member_name: impl Into<String>) -> Self {
        NativeMember {
            type_name: type_name.into(),
            member_name: member_name.into(),
        }
    }
}

/// `QLSecurityStrategy` 枚举的 Rust 实现，保留对应对象的领域职责与公开契约。
/// 对应或承接 Java 源文件：`com/alibaba/qlexpress4/security/QLSecurityStrategy.java`；具体对象路径见 `docs/对象级对照表.md`。
/// Mirroring Java `QLSecurityStrategy`: decides whether a native member may
/// be accessed from a script.
#[derive(Clone, Default)]
/// 对应 Java: com.alibaba.qlexpress4.security.QLSecurityStrategy。
pub enum QLSecurityStrategy {
    /// Java `QLSecurityStrategy.open()`: allow everything.
    Open,
    /// Java `QLSecurityStrategy.isolation()`: allow nothing. (Java default.)
    #[default]
    Isolation,
    /// Java `QLSecurityStrategy.blackList(Set<Member>)`.
    BlackList(HashSet<NativeMember>),
    /// Java `QLSecurityStrategy.whiteList(Set<Member>)`.
    WhiteList(HashSet<NativeMember>),
    /// 保存调用方集合引用的 Java 黑名单语义。
    SharedBlackList(SharedNativeMembers),
    /// 保存调用方集合引用的 Java 白名单语义。
    SharedWhiteList(SharedNativeMembers),
    /// 业务宿主实现 Java `QLSecurityStrategy#check(Member)` 的 Rust 闭包适配。
    Custom(Rc<dyn Fn(&NativeMember) -> bool>),
}

impl fmt::Debug for QLSecurityStrategy {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Open => formatter.write_str("Open"),
            Self::Isolation => formatter.write_str("Isolation"),
            Self::BlackList(members) => formatter.debug_tuple("BlackList").field(members).finish(),
            Self::WhiteList(members) => formatter.debug_tuple("WhiteList").field(members).finish(),
            Self::SharedBlackList(members) => formatter
                .debug_tuple("SharedBlackList")
                .field(&members.borrow())
                .finish(),
            Self::SharedWhiteList(members) => formatter
                .debug_tuple("SharedWhiteList")
                .field(&members.borrow())
                .finish(),
            Self::Custom(_) => formatter.write_str("Custom(..)"),
        }
    }
}

impl PartialEq for QLSecurityStrategy {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Open, Self::Open) | (Self::Isolation, Self::Isolation) => true,
            (Self::BlackList(left), Self::BlackList(right))
            | (Self::WhiteList(left), Self::WhiteList(right)) => left == right,
            (Self::SharedBlackList(left), Self::SharedBlackList(right))
            | (Self::SharedWhiteList(left), Self::SharedWhiteList(right)) => {
                Rc::ptr_eq(left, right) || *left.borrow() == *right.borrow()
            }
            (Self::Custom(left), Self::Custom(right)) => Rc::ptr_eq(left, right),
            _ => false,
        }
    }
}

impl Eq for QLSecurityStrategy {}

impl QLSecurityStrategy {
    /// 创建允许访问全部已注册原生成员的开放策略。
    /// 无显式参数；返回：`Self`。
    /// 对应或承接 Java 源文件：`com/alibaba/qlexpress4/security/QLSecurityStrategy.java`，方法 `open`；Rust 侧按所有权与 `Result` 语义适配。
    /// Java `QLSecurityStrategy.open()`.
    /// 对应 Java: com.alibaba.qlexpress4.security.QLSecurityStrategy#open。
    pub fn open() -> Self {
        QLSecurityStrategy::Open
    }

    /// 创建拒绝全部原生成员访问的隔离策略。
    /// 无显式参数；返回：`Self`。
    /// 对应或承接 Java 源文件：`com/alibaba/qlexpress4/security/QLSecurityStrategy.java`，方法 `isolation`；Rust 侧按所有权与 `Result` 语义适配。
    /// Java `QLSecurityStrategy.isolation()`.
    /// 对应 Java: com.alibaba.qlexpress4.security.QLSecurityStrategy#isolation。
    pub fn isolation() -> Self {
        QLSecurityStrategy::Isolation
    }

    /// 创建拒绝指定原生成员的黑名单策略。
    /// 参数：`black_list`；返回：`Self`。
    /// 对应或承接 Java 源文件：`com/alibaba/qlexpress4/security/QLSecurityStrategy.java`，方法 `blackList`；Rust 侧按所有权与 `Result` 语义适配。
    /// Java `QLSecurityStrategy.blackList(...)`.
    /// 对应 Java: com.alibaba.qlexpress4.security.QLSecurityStrategy#blackList。
    pub fn black_list(black_list: HashSet<NativeMember>) -> Self {
        QLSecurityStrategy::BlackList(black_list)
    }

    /// 创建仅允许指定原生成员的白名单策略。
    /// 参数：`white_list`；返回：`Self`。
    /// 对应或承接 Java 源文件：`com/alibaba/qlexpress4/security/QLSecurityStrategy.java`，方法 `whiteList`；Rust 侧按所有权与 `Result` 语义适配。
    /// Java `QLSecurityStrategy.whiteList(...)`.
    /// 对应 Java: com.alibaba.qlexpress4.security.QLSecurityStrategy#whiteList。
    pub fn white_list(white_list: HashSet<NativeMember>) -> Self {
        QLSecurityStrategy::WhiteList(white_list)
    }

    /// 使用调用方保留的共享集合创建黑名单策略。
    ///
    /// Java `StrategyBlackList` 直接保存构造参数 `Set<Member>`；调用方在
    /// runner 创建后修改集合时，后续成员检查会立即观察到变化。
    ///
    /// # 参数
    ///
    /// - `black_list`：宿主与策略共享的成员集合。
    ///
    /// # 返回值
    ///
    /// 返回具有 Java 引用语义的黑名单策略。
    /// 对应 Java：`QLSecurityStrategy#blackList(Set<Member>)` 的集合引用语义。
    pub fn shared_black_list(black_list: SharedNativeMembers) -> Self {
        Self::SharedBlackList(black_list)
    }

    /// 使用调用方保留的共享集合创建白名单策略。
    ///
    /// # 参数
    ///
    /// - `white_list`：宿主与策略共享的成员集合。
    ///
    /// # 返回值
    ///
    /// 返回具有 Java 引用语义的白名单策略。
    /// 对应 Java：`QLSecurityStrategy#whiteList(Set<Member>)` 的集合引用语义。
    pub fn shared_white_list(white_list: SharedNativeMembers) -> Self {
        Self::SharedWhiteList(white_list)
    }

    /// 使用宿主闭包创建自定义成员安全策略。
    ///
    /// Java 的 `QLSecurityStrategy` 是公开接口，业务宿主可按成员、租户或
    /// 动态配置实现 `check(Member)`；本方法以共享闭包保留同一扩展能力。
    ///
    /// # 参数
    ///
    /// - `check`：接收成员描述符并返回是否放行的策略函数。
    ///
    /// # 返回值
    ///
    /// 返回可克隆且共享闭包状态的安全策略。
    /// 对应 Java：业务实现 `QLSecurityStrategy#check(Member)`。
    pub fn custom<F>(check: F) -> Self
    where
        F: Fn(&NativeMember) -> bool + 'static,
    {
        Self::Custom(Rc::new(check))
    }

    /// 依据当前配置执行校验。
    /// 参数：`member`；返回：`bool`。
    /// 对应或承接 Java 源文件：`com/alibaba/qlexpress4/security/QLSecurityStrategy.java`，方法 `check`；Rust 侧按所有权与 `Result` 语义适配。
    /// Java `check(Member)`: true when the member is secure to access.
    /// 对应 Java: com.alibaba.qlexpress4.security.QLSecurityStrategy#check。
    pub fn check(&self, member: &NativeMember) -> bool {
        match self {
            QLSecurityStrategy::Open => true,
            QLSecurityStrategy::Isolation => false,
            QLSecurityStrategy::BlackList(black_list) => !black_list.contains(member),
            QLSecurityStrategy::WhiteList(white_list) => white_list.contains(member),
            QLSecurityStrategy::SharedBlackList(black_list) => {
                !black_list.borrow().contains(member)
            }
            QLSecurityStrategy::SharedWhiteList(white_list) => white_list.borrow().contains(member),
            QLSecurityStrategy::Custom(check) => check(member),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strategy_semantics() {
        let member = NativeMember::new("java.lang.Runtime", "exec");
        assert!(QLSecurityStrategy::open().check(&member));
        assert!(!QLSecurityStrategy::isolation().check(&member));

        let black: HashSet<_> = [member.clone()].into_iter().collect();
        assert!(!QLSecurityStrategy::black_list(black).check(&member));
        assert!(!QLSecurityStrategy::white_list(HashSet::new()).check(&member));

        let white: HashSet<_> = [member.clone()].into_iter().collect();
        assert!(QLSecurityStrategy::white_list(white).check(&member));
    }

    #[test]
    fn custom_strategy_preserves_java_interface_extensibility_and_shared_state() {
        let allowed = Rc::new(std::cell::RefCell::new(HashSet::new()));
        let captured = Rc::clone(&allowed);
        let strategy = QLSecurityStrategy::custom(move |member| captured.borrow().contains(member));
        let member = NativeMember::new("com.example.Service", "run");

        assert!(!strategy.check(&member));
        allowed.borrow_mut().insert(member.clone());
        assert!(strategy.check(&member));
        assert_eq!(strategy.clone(), strategy);
    }

    #[test]
    fn shared_lists_observe_mutation_after_strategy_creation() {
        let members = Rc::new(RefCell::new(HashSet::new()));
        let strategy = QLSecurityStrategy::shared_white_list(Rc::clone(&members));
        let member = NativeMember::new("com.example.Service", "run");

        assert!(!strategy.check(&member));
        members.borrow_mut().insert(member.clone());
        assert!(strategy.check(&member));
        members.borrow_mut().remove(&member);
        assert!(!strategy.check(&member));
    }

    #[test]
    fn shared_blacklist_and_strategy_identity_cover_remaining_java_adapters() {
        let member = NativeMember::new("com.example.Service", "run");
        let members = Rc::new(RefCell::new(HashSet::new()));
        let blacklist = QLSecurityStrategy::shared_black_list(Rc::clone(&members));
        assert!(blacklist.check(&member));
        members.borrow_mut().insert(member.clone());
        assert!(!blacklist.check(&member));

        assert_ne!(QLSecurityStrategy::open(), QLSecurityStrategy::isolation());
        assert_ne!(
            QLSecurityStrategy::black_list(HashSet::new()),
            QLSecurityStrategy::white_list(HashSet::new())
        );
        assert_ne!(
            QLSecurityStrategy::custom(|_| true),
            QLSecurityStrategy::custom(|_| true)
        );
    }
}
