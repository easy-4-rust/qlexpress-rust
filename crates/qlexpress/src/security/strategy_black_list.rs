//! 黑名单安全策略,对应 Java `com.alibaba.qlexpress4.security.StrategyBlackList`。
//! 职责:禁止访问黑名单中的 Java(Rust 本地)成员,其余放行。

use std::cell::{Ref, RefCell};
use std::collections::HashSet;
use std::rc::Rc;

use super::ql_security_strategy::{NativeMember, QLSecurityStrategy};
use crate::exception::{QLException, QLExceptionKind};

const NULL_POINTER_EXCEPTION: &str = "java.lang.NullPointerException";

/// 黑名单安全策略。对应 Java: com.alibaba.qlexpress4.security.StrategyBlackList
/// (A security policy that prohibits access to Java members in the blacklist.)
///
/// 成员以 [`NativeMember`] 描述符(类型名 + 成员名)标识,对应 Java 的
/// `java.lang.reflect.Member`;匹配语义与 Java `Set<Member>.contains` 一致
/// (精确匹配 `type_name.member_name`)。
/// 接线说明:成员分派点(GetField/GetMethod/MethodInvoke 指令经
/// `runtime/member.rs` 的 NativeRegistry 解析出 `NativeMember` 后调用
/// 本策略 `check`)在 Express4Runner 阶段接线,本文件仅提供策略本体。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StrategyBlackList {
    /// 黑名单成员集合。对应 Java 字段 `blackList`(`Set<Member>`)。
    black_list: Option<Rc<RefCell<HashSet<Option<NativeMember>>>>>,
}

impl StrategyBlackList {
    /// 构造黑名单策略。对应 Java 构造器 `StrategyBlackList(Set<Member> blackList)`。
    pub fn new(black_list: HashSet<NativeMember>) -> Self {
        StrategyBlackList {
            black_list: Some(Rc::new(RefCell::new(
                black_list.into_iter().map(Some).collect(),
            ))),
        }
    }

    /// 从 Java 可空、共享的 `Set<Member>` 引用创建黑名单。
    /// 对应 Java: com.alibaba.qlexpress4.security.StrategyBlackList#StrategyBlackList。
    pub fn from_shared(black_list: Option<Rc<RefCell<HashSet<Option<NativeMember>>>>>) -> Self {
        StrategyBlackList { black_list }
    }

    /// 检查成员是否安全:不在黑名单即放行。
    /// 对应 Java 方法 `check(Member)`(实现 `return !blackList.contains(member)`)。
    pub fn check(&self, member: Option<&NativeMember>) -> Result<bool, QLException> {
        let members = self.black_list.as_ref().ok_or_else(|| {
            QLException::host_error(
                QLExceptionKind::Runtime,
                NULL_POINTER_EXCEPTION,
                NULL_POINTER_EXCEPTION,
            )
        })?;
        Ok(!members
            .borrow()
            .iter()
            .any(|candidate| candidate.as_ref() == member))
    }

    /// 黑名单集合。对应 Java 字段访问。
    pub fn black_list(&self) -> Option<Ref<'_, HashSet<Option<NativeMember>>>> {
        self.black_list.as_ref().map(|members| members.borrow())
    }
}

impl Default for StrategyBlackList {
    fn default() -> Self {
        Self::new(HashSet::new())
    }
}

/// 转为等价的外观枚举 [`QLSecurityStrategy::BlackList`]。
impl From<StrategyBlackList> for QLSecurityStrategy {
    fn from(strategy: StrategyBlackList) -> Self {
        QLSecurityStrategy::Custom(Rc::new(move |member| {
            strategy
                .check(Some(member))
                .expect("StrategyBlackList.blackList is null")
        }))
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
        assert!(!strategy.check(Some(&dangerous)).expect("black list"));
        // 黑名单外:放行(同类型不同成员、不同类型均不受限)
        assert!(strategy
            .check(Some(&NativeMember::new(
                "java.lang.Runtime",
                "availableProcessors"
            )))
            .expect("black list"));
        assert!(strategy
            .check(Some(&NativeMember::new("java.lang.String", "length")))
            .expect("black list"));
        assert!(strategy.check(None).expect("black list"));
        let as_enum: QLSecurityStrategy = strategy.into();
        assert!(!as_enum.is_allowed(&dangerous));
    }

    #[test]
    fn preserves_shared_set_null_member_and_null_set_semantics() {
        let shared = Rc::new(RefCell::new(HashSet::new()));
        let strategy = StrategyBlackList::from_shared(Some(Rc::clone(&shared)));
        let member = NativeMember::new("java.lang.String", "length");
        assert!(strategy.check(Some(&member)).expect("black list"));
        shared.borrow_mut().insert(Some(member.clone()));
        shared.borrow_mut().insert(None);
        assert!(!strategy.check(Some(&member)).expect("black list"));
        assert!(!strategy.check(None).expect("black list"));

        let null_set = StrategyBlackList::from_shared(None);
        let error = null_set
            .check(Some(&member))
            .expect_err("null Java set must fail");
        assert_eq!(error.error_code(), NULL_POINTER_EXCEPTION);
    }
}
