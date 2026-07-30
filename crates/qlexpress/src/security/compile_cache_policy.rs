//! 安全编译缓存策略。

/// 按租户隔离的有界编译缓存配置。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CompileCachePolicy {
    /// 是否允许安全执行入口使用编译缓存。
    pub enabled: bool,
    /// 全 runner 最大条目数；达到容量时按 LRU 淘汰。
    pub max_entries: usize,
    /// 单租户最大条目数；防止一个租户污染全部缓存。
    pub max_entries_per_tenant: usize,
}

impl Default for CompileCachePolicy {
    fn default() -> Self {
        Self {
            enabled: true,
            max_entries: 256,
            max_entries_per_tenant: 64,
        }
    }
}

impl CompileCachePolicy {
    /// 校验启用缓存时容量配置有效。
    pub fn validate(&self) -> Result<(), &'static str> {
        if self.enabled
            && (self.max_entries == 0
                || self.max_entries_per_tenant == 0
                || self.max_entries_per_tenant > self.max_entries)
        {
            return Err(
                "cache limits must be positive and per-tenant capacity must not exceed total",
            );
        }
        Ok(())
    }
}
