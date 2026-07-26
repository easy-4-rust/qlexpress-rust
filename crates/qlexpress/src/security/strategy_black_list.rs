//! 黑名单安全策略,对应 Java `com.alibaba.qlexpress4.security.StrategyBlackList`。
//! 职责:禁止访问黑名单中的 Java(Rust 本地)成员,其余放行。

use std::collections::HashSet;

use super::ql_security_strategy::{NativeMember, QLSecurityStrategy};

/// 黑名单安全策略。对应 Java: com.alibaba.qlexpress4.security.StrategyBlackList
/// (A security policy that prohibits access to Java members in the blacklist.)
///
/// 成员以 [`NativeMember`] 描述符(类型名 + 成员名)标识,对应 Java 的
/// `java.lang.reflect.Member`;匹配语义与 Java `Set<Member>.contains` 一致
/// (精确匹配 `type_name.member_name`)。
/// 接线说明:成员分派点(GetField/GetMethod/MethodInvoke 指令经
/// `runtime/member.rs` 的 NativeRegistry 解析出 `NativeMember` 后调用
/// 本策略 `check`)在 Express4Runner 阶段接线,本文件仅提供策略本体。
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct StrategyBlackList {
    /// 黑名单成员集合。对应 Java 字段 `blackList`(`Set<Member>`)。
    black_list: HashSet<NativeMember>,
}

impl StrategyBlackList {
    /// 构造黑名单策略。对应 Java 构造器 `StrategyBlackList(Set<Member> blackList)`。
    pub fn new(black_list: HashSet<NativeMember>) -> Self {
        StrategyBlackList { black_list }
    }

    /// 检查成员是否安全:不在黑名单即放行。
    /// 对应 Java 方法 `check(Member)`(实现 `return !blackList.contains(member)`)。
    pub fn check(&self, member: &NativeMember) -> bool {
        !self.black_list.contains(member)
    }

    /// 黑名单集合。对应 Java 字段访问。
    pub fn black_list(&self) -> &HashSet<NativeMember> {
        &self.black_list
    }
}

/// 转为等价的外观枚举 [`QLSecurityStrategy::BlackList`]。
impl From<StrategyBlackList> for QLSecurityStrategy {
    fn from(strategy: StrategyBlackList) -> Self {
        QLSecurityStrategy::BlackList(strategy.black_list)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn denies_listed_allows_others() {
        let dangerous = NativeMember::new("java.lang.Runtime", "exec");
        let black: HashSet<_> = [dangerous.clone()].into_iter().collect();
        let strategy = StrategyBlackList::new(black);
        // 黑名单内:拒绝
        assert!(!strategy.check(&dangerous));
        // 黑名单外:放行(同类型不同成员、不同类型均不受限)
        assert!(strategy.check(&NativeMember::new(
            "java.lang.Runtime",
            "availableProcessors"
        )));
        assert!(strategy.check(&NativeMember::new("java.lang.String", "length")));
        let as_enum: QLSecurityStrategy = strategy.into();
        assert!(!as_enum.check(&dangerous));
    }
}
