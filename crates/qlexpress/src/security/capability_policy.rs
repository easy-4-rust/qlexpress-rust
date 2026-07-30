//! 统一宿主能力白名单。

use std::collections::HashSet;

use super::capability::Capability;

/// 安全执行的 capability allowlist。
///
/// 默认不允许任何宿主扩展能力。语言内建语法和纯 QVM 操作不属于宿主
/// capability；显式注册的函数、宏、扩展方法、操作符和 Native 成员必须授权。
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct CapabilityPolicy {
    allowed: HashSet<Capability>,
}

impl CapabilityPolicy {
    /// 创建默认拒绝全部宿主能力的策略。
    pub fn deny_all() -> Self {
        Self::default()
    }

    /// 创建只允许给定能力集合的策略。
    pub fn allow_only(capabilities: impl IntoIterator<Item = Capability>) -> Self {
        Self {
            allowed: capabilities.into_iter().collect(),
        }
    }

    /// 将一个能力加入白名单。
    pub fn allow(mut self, capability: Capability) -> Self {
        self.allowed.insert(capability);
        self
    }

    /// 判断能力是否被显式允许。
    pub fn is_allowed(&self, capability: &Capability) -> bool {
        self.allowed.contains(capability)
    }

    /// 返回白名单的只读视图。
    pub fn allowed(&self) -> &HashSet<Capability> {
        &self.allowed
    }
}
