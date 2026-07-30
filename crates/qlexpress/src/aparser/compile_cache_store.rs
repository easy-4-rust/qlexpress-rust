//! Runner 内部的按租户有界 LRU 编译缓存。

use std::collections::{HashMap, VecDeque};
use std::rc::Rc;

use crate::api::parsecache::LoadedCompileCache;
use crate::security::CacheStats;

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
struct CacheKey {
    tenant_id: String,
    script: String,
}

/// 按租户隔离、容量有界并提供统计的 LRU 编译缓存。
///
/// Java `Express4Runner.compileCache` 是无界并发 Map；Rust 安全增强使用
/// 此对象替代无界表。淘汰只影响性能，不改变脚本语义。
pub struct CompileCacheStore {
    entries: HashMap<CacheKey, Rc<LoadedCompileCache>>,
    lru: VecDeque<CacheKey>,
    hits: u64,
    misses: u64,
    evictions: u64,
}

impl CompileCacheStore {
    /// 创建空缓存。
    pub fn new() -> Self {
        Self {
            entries: HashMap::new(),
            lru: VecDeque::new(),
            hits: 0,
            misses: 0,
            evictions: 0,
        }
    }

    /// 查询并提升一个租户脚本为最近使用项。
    pub fn get(&mut self, tenant_id: &str, script: &str) -> Option<Rc<LoadedCompileCache>> {
        let lookup = CacheKey {
            tenant_id: tenant_id.to_string(),
            script: script.to_string(),
        };
        let value = self.entries.get(&lookup).map(Rc::clone);
        match value {
            Some(value) => {
                self.hits = self.hits.saturating_add(1);
                self.touch(&lookup);
                Some(value)
            }
            None => {
                self.misses = self.misses.saturating_add(1);
                None
            }
        }
    }

    /// 插入编译产物，并同时执行单租户和全局 LRU 淘汰。
    pub fn insert(
        &mut self,
        tenant_id: &str,
        script: String,
        value: Rc<LoadedCompileCache>,
        max_entries: usize,
        max_entries_per_tenant: usize,
    ) {
        if max_entries == 0 || max_entries_per_tenant == 0 {
            return;
        }
        let key = CacheKey {
            tenant_id: tenant_id.to_string(),
            script,
        };
        self.entries.insert(key.clone(), value);
        self.touch(&key);

        while self.tenant_entry_count(tenant_id) > max_entries_per_tenant {
            if !self.evict_oldest_for_tenant(tenant_id) {
                break;
            }
        }
        while self.entries.len() > max_entries {
            if !self.evict_oldest() {
                break;
            }
        }
    }

    /// 清空所有租户缓存，不清零累计命中统计。
    pub fn clear(&mut self) {
        self.entries.clear();
        self.lru.clear();
    }

    /// 返回当前条目数及累计命中、未命中、淘汰统计。
    pub fn stats(&self) -> CacheStats {
        CacheStats {
            entries: self.entries.len(),
            hits: self.hits,
            misses: self.misses,
            evictions: self.evictions,
        }
    }

    fn touch(&mut self, key: &CacheKey) {
        if let Some(index) = self.lru.iter().position(|candidate| candidate == key) {
            self.lru.remove(index);
        }
        self.lru.push_back(key.clone());
    }

    fn tenant_entry_count(&self, tenant_id: &str) -> usize {
        self.entries
            .keys()
            .filter(|key| key.tenant_id == tenant_id)
            .count()
    }

    fn evict_oldest_for_tenant(&mut self, tenant_id: &str) -> bool {
        let Some(index) = self
            .lru
            .iter()
            .position(|candidate| candidate.tenant_id == tenant_id)
        else {
            return false;
        };
        let Some(key) = self.lru.remove(index) else {
            return false;
        };
        if self.entries.remove(&key).is_some() {
            self.evictions = self.evictions.saturating_add(1);
        }
        true
    }

    fn evict_oldest(&mut self) -> bool {
        let Some(key) = self.lru.pop_front() else {
            return false;
        };
        if self.entries.remove(&key).is_some() {
            self.evictions = self.evictions.saturating_add(1);
        }
        true
    }
}

impl Default for CompileCacheStore {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runtime::qlambda_definition_empty::QLambdaDefinitionEmpty;
    use crate::runtime::trace::TracePointTree;

    fn value() -> Rc<LoadedCompileCache> {
        Rc::new(crate::aparser::compile_cache::QCompileCache::new(
            Rc::new(QLambdaDefinitionEmpty),
            Vec::<TracePointTree>::new(),
        ))
    }

    #[test]
    fn evicts_lru_and_isolates_tenants() {
        let mut cache = CompileCacheStore::new();
        cache.insert("a", "1".into(), value(), 3, 2);
        cache.insert("a", "2".into(), value(), 3, 2);
        assert!(cache.get("a", "1").is_some());
        cache.insert("a", "3".into(), value(), 3, 2);
        assert!(cache.get("a", "2").is_none());
        cache.insert("b", "1".into(), value(), 3, 2);
        assert_eq!(cache.stats().entries, 3);
        assert!(cache.get("b", "1").is_some());
        assert_eq!(cache.stats().evictions, 1);
    }
}
