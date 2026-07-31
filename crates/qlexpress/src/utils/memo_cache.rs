//! Rust 原生的按键一次性计算缓存。

use std::cell::RefCell;
use std::collections::HashMap;
use std::hash::Hash;

/// 通用的按键一次性计算缓存。
///
/// Java `CacheUtil` 只负责函数式接口；此类型保留早期 Rust API 的通用
/// `computeIfAbsent` 能力，属于 `RUST_EXTENSION`。
pub struct MemoCache<K, V> {
    cache: RefCell<HashMap<K, V>>,
}

impl<K: Eq + Hash, V: Clone> MemoCache<K, V> {
    /// 创建空缓存。
    ///
    /// # 返回值
    ///
    /// 返回不含任何条目的缓存。
    pub fn new() -> Self {
        Self {
            cache: RefCell::new(HashMap::new()),
        }
    }

    /// 缺少键时计算、缓存并返回新值。
    ///
    /// # 参数
    ///
    /// - `key`：缓存键。
    /// - `compute`：仅在首次缺少键时执行的计算。
    ///
    /// # 返回值
    ///
    /// 返回已有缓存值或首次计算并写入的值。
    pub fn compute_if_absent(&self, key: K, compute: impl FnOnce(&K) -> V) -> V {
        if let Some(value) = self.cache.borrow().get(&key) {
            return value.clone();
        }
        let value = compute(&key);
        self.cache.borrow_mut().insert(key, value.clone());
        value
    }
}

impl<K: Eq + Hash, V: Clone> Default for MemoCache<K, V> {
    fn default() -> Self {
        Self::new()
    }
}
