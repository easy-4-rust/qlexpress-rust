//! 白名单安全策略,对应 Java `com.alibaba.qlexpress4.security.StrategyWhiteList`。
//! 职责:仅放行白名单中的 Java(Rust 本地)成员,其余一律拒绝。

use std::cell::{Ref, RefCell};
use std::collections::HashSet;
use std::rc::Rc;

use super::ql_security_strategy::{NativeMember, QLSecurityStrategy};
use crate::exception::{QLException, QLExceptionKind};

const NULL_POINTER_EXCEPTION: &str = "java.lang.NullPointerException";

/// 白名单安全策略。对应 Java: com.alibaba.qlexpress4.security.StrategyWhiteList
/// (A security policy that only permits access to Java members in the whitelist.)
///
/// 成员以 [`NativeMember`] 描述符(类型名 + 成员名)标识,对应 Java 的
/// `java.lang.reflect.Member`;匹配语义与 Java `Set<Member>.contains` 一致
/// (精确匹配 `type_name.member_name`)。
/// 接线说明:成员分派点(见 strategy_black_list.rs 注释)在 Express4Runner
/// 阶段接线,本文件仅提供策略本体。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StrategyWhiteList {
    /// 白名单成员集合。对应 Java 字段 `whiteList`(`Set<Member>`)。
    white_list: Option<Rc<RefCell<HashSet<Option<NativeMember>>>>>,
}

impl StrategyWhiteList {
    /// 构造白名单策略。对应 Java 构造器 `StrategyWhiteList(Set<Member> whiteList)`。
    pub fn new(white_list: HashSet<NativeMember>) -> Self {
        StrategyWhiteList {
            white_list: Some(Rc::new(RefCell::new(
                white_list.into_iter().map(Some).collect(),
            ))),
        }
    }

    /// 从 Java 可空、共享的 `Set<Member>` 引用创建白名单。
    /// 对应 Java: com.alibaba.qlexpress4.security.StrategyWhiteList#StrategyWhiteList。
    pub fn from_shared(white_list: Option<Rc<RefCell<HashSet<Option<NativeMember>>>>>) -> Self {
        StrategyWhiteList { white_list }
    }

    /// 检查成员是否安全:在白名单内才放行。
    /// 对应 Java 方法 `check(Member)`(实现 `return whiteList.contains(member)`)。
    pub fn check(&self, member: Option<&NativeMember>) -> Result<bool, QLException> {
        let members = self.white_list.as_ref().ok_or_else(|| {
            QLException::host_error(
                QLExceptionKind::Runtime,
                NULL_POINTER_EXCEPTION,
                NULL_POINTER_EXCEPTION,
            )
        })?;
        Ok(members
            .borrow()
            .iter()
            .any(|candidate| candidate.as_ref() == member))
    }

    /// 白名单集合。对应 Java 字段访问。
    pub fn white_list(&self) -> Option<Ref<'_, HashSet<Option<NativeMember>>>> {
        self.white_list.as_ref().map(|members| members.borrow())
    }
}

impl Default for StrategyWhiteList {
    fn default() -> Self {
        Self::new(HashSet::new())
    }
}

/// 转为等价的外观枚举 [`QLSecurityStrategy::WhiteList`]。
impl From<StrategyWhiteList> for QLSecurityStrategy {
    fn from(strategy: StrategyWhiteList) -> Self {
        QLSecurityStrategy::Custom(Rc::new(move |member| {
            strategy
                .check(Some(member))
                .expect("StrategyWhiteList.whiteList is null")
        }))
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
        assert!(strategy.check(Some(&allowed)).expect("white list"));
        // 白名单外:拒绝(空名单即全拒)
        assert!(!strategy
            .check(Some(&NativeMember::new("java.lang.String", "substring")))
            .expect("white list"));
        assert!(!StrategyWhiteList::default()
            .check(Some(&allowed))
            .expect("white list"));
        assert!(!strategy.check(None).expect("white list"));
        let as_enum: QLSecurityStrategy = strategy.into();
        assert!(as_enum.is_allowed(&allowed));
    }

    #[test]
    fn preserves_shared_set_null_member_and_null_set_semantics() {
        let shared = Rc::new(RefCell::new(HashSet::new()));
        let strategy = StrategyWhiteList::from_shared(Some(Rc::clone(&shared)));
        let member = NativeMember::new("java.lang.String", "length");
        assert!(!strategy.check(Some(&member)).expect("white list"));
        shared.borrow_mut().insert(Some(member.clone()));
        shared.borrow_mut().insert(None);
        assert!(strategy.check(Some(&member)).expect("white list"));
        assert!(strategy.check(None).expect("white list"));

        let null_set = StrategyWhiteList::from_shared(None);
        let error = null_set
            .check(Some(&member))
            .expect_err("null Java set must fail");
        assert_eq!(error.error_code(), NULL_POINTER_EXCEPTION);
    }
}
