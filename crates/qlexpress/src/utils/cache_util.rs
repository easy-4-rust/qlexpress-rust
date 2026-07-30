//! Small memoization cache, mirroring Java `CacheUtil`.
//!
//! Java caches `Class -> isFunctionInterface` in a `ConcurrentHashMap`.
//! Rust has no runtime class objects; the same "compute once, cache by key"
//! semantics are offered generically with interior mutability so callers can
//! share a cache behind `&self`.

use std::cell::RefCell;
use std::collections::HashMap;
use std::hash::Hash;

/// 按键缓存一次性计算结果。
/// 对应 Java: `com.alibaba.qlexpress4.utils.CacheUtil`，承接 `computeIfAbsent` 缓存语义。
pub struct CacheUtil<K, V> {
    cache: RefCell<HashMap<K, V>>,
}

impl<K: Eq + Hash, V: Clone> CacheUtil<K, V> {
    /// 创建空缓存；后续相同键只执行一次计算。
    /// 对应 Java: `CacheUtil` 内部 `ConcurrentHashMap` 的初始化。
    pub fn new() -> Self {
        CacheUtil {
            cache: RefCell::new(HashMap::new()),
        }
    }

    /// 处理 compute if absent 对应的领域职责。
    /// 参数：`key`、`compute`；返回：`V) -> V`。
    /// 对应或承接 Java 源文件：`com/alibaba/qlexpress4/utils/CacheUtil.java`，方法 `computeIfAbsent`；Rust 侧按所有权与 `Result` 语义适配。
    /// Java `Map.computeIfAbsent` semantics.
    /// 对应 Java: com.alibaba.qlexpress4.utils.CacheUtil#computeIfAbsent。
    pub fn compute_if_absent(&self, key: K, compute: impl FnOnce(&K) -> V) -> V {
        if let Some(value) = self.cache.borrow().get(&key) {
            return value.clone();
        }
        let value = compute(&key);
        self.cache.borrow_mut().insert(key, value.clone());
        value
    }
}

impl<K: Eq + Hash, V: Clone> Default for CacheUtil<K, V> {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::Cell;

    #[test]
    fn computes_only_once_per_key() {
        let cache: CacheUtil<String, bool> = CacheUtil::new();
        let calls = Cell::new(0);
        let compute = |_: &String| {
            calls.set(calls.get() + 1);
            true
        };
        assert!(cache.compute_if_absent("java.util.function.Function".to_string(), compute));
        assert!(cache.compute_if_absent("java.util.function.Function".to_string(), compute));
        assert_eq!(calls.get(), 1);
    }
}
