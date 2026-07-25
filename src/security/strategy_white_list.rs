//! 白名单安全策略,对应 Java `com.alibaba.qlexpress4.security.StrategyWhiteList`。
//! 职责:仅放行白名单中的 Java(Rust 本地)成员,其余一律拒绝。

use std::collections::HashSet;

use super::ql_security_strategy::{NativeMember, QLSecurityStrategy};

/// 白名单安全策略。对应 Java: com.alibaba.qlexpress4.security.StrategyWhiteList
/// (A security policy that only permits access to Java members in the whitelist.)
///
/// 成员以 [`NativeMember`] 描述符(类型名 + 成员名)标识,对应 Java 的
/// `java.lang.reflect.Member`;匹配语义与 Java `Set<Member>.contains` 一致
/// (精确匹配 `type_name.member_name`)。
/// 接线说明:成员分派点(见 strategy_black_list.rs 注释)在 Express4Runner
/// 阶段接线,本文件仅提供策略本体。
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct StrategyWhiteList {
    /// 白名单成员集合。对应 Java 字段 `whiteList`(`Set<Member>`)。
    white_list: HashSet<NativeMember>,
}

impl StrategyWhiteList {
    /// 构造白名单策略。对应 Java 构造器 `StrategyWhiteList(Set<Member> whiteList)`。
    pub fn new(white_list: HashSet<NativeMember>) -> Self {
        StrategyWhiteList { white_list }
    }

    /// 检查成员是否安全:在白名单内才放行。
    /// 对应 Java 方法 `check(Member)`(实现 `return whiteList.contains(member)`)。
    pub fn check(&self, member: &NativeMember) -> bool {
        self.white_list.contains(member)
    }

    /// 白名单集合。对应 Java 字段访问。
    pub fn white_list(&self) -> &HashSet<NativeMember> {
        &self.white_list
    }
}

/// 转为等价的外观枚举 [`QLSecurityStrategy::WhiteList`]。
impl From<StrategyWhiteList> for QLSecurityStrategy {
    fn from(strategy: StrategyWhiteList) -> Self {
        QLSecurityStrategy::WhiteList(strategy.white_list)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn allows_listed_denies_others() {
        let allowed = NativeMember::new("java.lang.String", "length");
        let white: HashSet<_> = [allowed.clone()].into_iter().collect();
        let strategy = StrategyWhiteList::new(white);
        // 白名单内:放行
        assert!(strategy.check(&allowed));
        // 白名单外:拒绝(空名单即全拒)
        assert!(!strategy.check(&NativeMember::new("java.lang.String", "substring")));
        assert!(!StrategyWhiteList::default().check(&allowed));
        let as_enum: QLSecurityStrategy = strategy.into();
        assert!(as_enum.check(&allowed));
    }
}
