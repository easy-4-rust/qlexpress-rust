//! Security strategy, mirroring Java `QLSecurityStrategy`.
//!
//! Java checks `java.lang.reflect.Member` objects; Rust native members are
//! registered explicitly (SPEC §4/§6), so members are identified by
//! `NativeMember` descriptors (`type_name.member_name`).

use std::collections::HashSet;
use std::fmt;
use std::rc::Rc;

pub use super::native_member::NativeMember;

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
#[derive(Clone)]
/// 对应 Java: com.alibaba.qlexpress4.security.QLSecurityStrategy。
pub enum QLSecurityStrategy {
    /// Java `QLSecurityStrategy.open()`: allow everything.
    Open,
    /// Java `QLSecurityStrategy.isolation()`: allow nothing. (Java default.)
    Isolation,
    /// Java `QLSecurityStrategy.blackList(Set<Member>)`.
    BlackList(HashSet<NativeMember>),
    /// Java `QLSecurityStrategy.whiteList(Set<Member>)`.
    WhiteList(HashSet<NativeMember>),
    /// 业务宿主实现 Java `QLSecurityStrategy#check(Member)` 的 Rust 闭包适配。
    Custom(Rc<dyn Fn(&NativeMember) -> bool>),
}

impl Default for QLSecurityStrategy {
    fn default() -> Self {
        Self::Isolation
    }
}

impl fmt::Debug for QLSecurityStrategy {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Open => formatter.write_str("Open"),
            Self::Isolation => formatter.write_str("Isolation"),
            Self::BlackList(members) => formatter
                .debug_tuple("BlackList")
                .field(members)
                .finish(),
            Self::WhiteList(members) => formatter
                .debug_tuple("WhiteList")
                .field(members)
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
        let strategy =
            QLSecurityStrategy::custom(move |member| captured.borrow().contains(member));
        let member = NativeMember::new("com.example.Service", "run");

        assert!(!strategy.check(&member));
        allowed.borrow_mut().insert(member.clone());
        assert!(strategy.check(&member));
        assert_eq!(strategy.clone(), strategy);
    }
}
