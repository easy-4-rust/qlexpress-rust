//! 统一宿主能力白名单。

use std::collections::HashSet;

use super::capability::Capability;

/// 安全执行的 capability allowlist。
///
/// 默认不允许任何宿主扩展能力。语言内建语法和纯 QVM 操作不属于宿主
/// capability；显式注册的函数、宏、扩展方法、操作符和 Native 成员必须授权。
/// 对应 Java: 无（Rust 安全增强，将宿主扩展统一建模为显式能力）。
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct CapabilityPolicy {
    allowed: HashSet<Capability>,
}

impl CapabilityPolicy {
    /// 创建默认拒绝全部宿主能力的策略。
    /// 对应 Java：无（Rust 安全增强的统一 capability 白名单）。
    pub fn deny_all() -> Self {
        Self::default()
    }

    /// 创建只允许给定能力集合的策略。
    ///
    /// # Arguments
    ///
    /// * `capabilities` - 本次安全执行可以使用的完整宿主能力集合。
    ///
    /// # Returns
    ///
    /// 返回默认拒绝集合之外所有能力的白名单策略。
    /// 对应 Java：无（Rust 安全增强的统一 capability 白名单）。
    pub fn allow_only(capabilities: impl IntoIterator<Item = Capability>) -> Self {
        Self {
            allowed: capabilities.into_iter().collect(),
        }
    }

    /// 将一个能力加入白名单。
    ///
    /// # Arguments
    ///
    /// * `capability` - 要授权的函数、宏、操作符、扩展方法或 Native 成员。
    ///
    /// # Returns
    ///
    /// 返回包含新能力的策略，便于链式构造。
    /// 对应 Java：无（Rust 安全增强的显式能力授权）。
    pub fn allow(mut self, capability: Capability) -> Self {
        self.allowed.insert(capability);
        self
    }

    /// 判断能力是否被显式允许。
    ///
    /// # Arguments
    ///
    /// * `capability` - 待检查的规范化宿主能力。
    ///
    /// # Returns
    ///
    /// 仅当能力出现在白名单中时返回 `true`。
    /// 对应 Java：无（Rust 安全增强的统一 capability 校验）。
    pub fn is_allowed(&self, capability: &Capability) -> bool {
        self.allowed.contains(capability)
    }

    /// 返回白名单的只读视图。
    ///
    /// # Returns
    ///
    /// 返回当前显式授权能力集合，不包含任何隐式权限。
    /// 对应 Java：无（Rust 安全增强的能力策略可观测性）。
    pub fn allowed(&self) -> &HashSet<Capability> {
        &self.allowed
    }

    /// 判断安全运行时能否调用接收者上的方法。
    ///
    /// Native 成员按运行时类型精确匹配；扩展函数允许其声明类型精确匹配。
    /// Java `List` 扩展在 Rust 中的运行时值类型为 `ArrayList`，因此显式
    /// 接受 `java.util.List` 作为该内建接口的声明类型。
    ///
    /// # Arguments
    ///
    /// * `runtime_type` - 接收者运行时的 Java 规范类型名。
    /// * `method_name` - 脚本准备调用的方法名。
    ///
    /// # Returns
    ///
    /// Native 成员或扩展方法获得明确授权时返回 `true`。
    /// 对应 Java：`QLSecurityStrategy#check(Member)`（Rust 扩展到 Native 与扩展方法能力）。
    pub fn is_method_allowed(&self, runtime_type: &str, method_name: &str) -> bool {
        self.allowed.iter().any(|capability| match capability {
            Capability::NativeMember {
                type_name,
                member_name,
            } => type_name == runtime_type && member_name == method_name,
            Capability::ExtensionMethod {
                type_name,
                method_name: allowed_method,
            } => {
                allowed_method == method_name
                    && (type_name == runtime_type
                        || (type_name == "java.util.List" && runtime_type == "java.util.ArrayList"))
            }
            _ => false,
        })
    }
}
