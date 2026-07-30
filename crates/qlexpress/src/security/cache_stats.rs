//! 编译缓存统计快照。

/// 安全编译缓存的累计统计。
/// 对应 Java: 无（Rust 安全增强）。
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct CacheStats {
    /// 当前缓存条目。
    pub entries: usize,
    /// 命中次数。
    pub hits: u64,
    /// 未命中次数。
    pub misses: u64,
    /// LRU 淘汰次数。
    pub evictions: u64,
}
