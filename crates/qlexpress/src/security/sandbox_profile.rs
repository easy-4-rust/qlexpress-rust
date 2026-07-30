//! 安全执行配置聚合。

use crate::check_options::CheckOptions;

use super::cancellation_token::CancellationToken;
use super::capability_policy::CapabilityPolicy;
use super::compile_cache_policy::CompileCachePolicy;
use super::resource_limits::ResourceLimits;

/// `execute_checked` 使用的完整沙箱配置。
///
/// 默认配置是有限预算、默认拒绝宿主 capability、启用有界租户缓存。
/// 它与 Java 兼容的 `QLOptions::default()` 相互独立。
#[derive(Clone, Debug)]
pub struct SandboxProfile {
    /// 各阶段资源预算。
    pub limits: ResourceLimits,
    /// 操作符和函数调用的静态检查规则。
    pub check_options: CheckOptions,
    /// 宿主扩展能力白名单。
    pub capability_policy: CapabilityPolicy,
    /// 安全编译缓存策略。
    pub compile_cache: CompileCachePolicy,
    /// 缓存隔离租户；空字符串无效。
    pub tenant_id: String,
    /// 外部协作式取消令牌。
    pub cancellation_token: CancellationToken,
}

impl Default for SandboxProfile {
    fn default() -> Self {
        Self {
            limits: ResourceLimits::default(),
            check_options: CheckOptions::default(),
            capability_policy: CapabilityPolicy::deny_all(),
            compile_cache: CompileCachePolicy::default(),
            tenant_id: "default".to_string(),
            cancellation_token: CancellationToken::new(),
        }
    }
}

impl SandboxProfile {
    /// 创建默认安全配置。
    pub fn secure() -> Self {
        Self::default()
    }

    /// 校验配置可用于安全执行。
    pub fn validate(&self) -> Result<(), &'static str> {
        self.limits.validate()?;
        self.compile_cache.validate()?;
        if self.tenant_id.trim().is_empty() {
            return Err("sandbox tenant_id must not be empty");
        }
        Ok(())
    }
}
