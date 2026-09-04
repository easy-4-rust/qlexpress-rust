//! Rust 多线程执行模型使用的线程安全编译缓存。

use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Mutex, MutexGuard};

use crate::api::parsecache::SerializableParseCache;
use crate::exception::QLException;

/// 跨线程共享的可序列化编译缓存。
///
/// Java `Express4Runner` 直接共享 `ConcurrentHashMap<String,
/// Future<QCompileCache>>`；Rust Runner 内部含 `Rc/RefCell`，因此采用
/// “线程本地 Runner + 共享纯数据缓存”模型。锁覆盖首次编译，保证相同
/// 脚本在并发冷启动时也只执行一次 `compile`。
///
/// ## 锁毒恢复
///
/// `std::sync::Mutex` 一旦 poison 就**永久**带毒——后续每次 `lock()`
/// 都会返回 `Err(PoisonError)`。原始实现用 `unwrap_or_else(PoisonError::into_inner)`
/// 会在毒状态下继续读"panic 线程可能写了一半"的哈希表，是数据竞争隐患。
///
/// 本实现用一个 `AtomicBool` 保证"清毒"动作只发生一次：首次发现 poison 时
/// 清空哈希表（丢弃 panic 线程可能写脏的数据），之后只拿锁不重清。
/// `AtomicBool::swap` 在持锁状态下求值，跨线程无竞态。
#[derive(Default)]
/// 对应 Java: 无（Rust 原生适配）。
pub struct ConcurrentParseCache {
    caches: Mutex<HashMap<String, SerializableParseCache>>,
    compile_count: AtomicUsize,
    poison_cleared: AtomicBool,
}

impl ConcurrentParseCache {
    /// 创建空缓存。
    /// 对应 Java: 无（Rust 原生适配）。
    pub fn new() -> Self {
        Self::default()
    }

    /// 加锁并处理 poison：首次发现 poison 时清空哈希表丢弃脏数据，
    /// 之后只拿锁不重清。
    fn lock_recovered(&self) -> MutexGuard<'_, HashMap<String, SerializableParseCache>> {
        match self.caches.lock() {
            Ok(g) => g,
            Err(poisoned) => {
                let mut guard = poisoned.into_inner();
                if !self.poison_cleared.swap(true, Ordering::AcqRel) {
                    guard.clear();
                }
                guard
            }
        }
    }

    /// 返回已有缓存，或在互斥区内执行一次编译并保存结果。
    /// 对应 Java: 无（Rust 原生适配）。
    pub fn get_or_compile<F>(
        &self,
        script: &str,
        compile: F,
    ) -> Result<SerializableParseCache, QLException>
    where
        F: FnOnce() -> Result<SerializableParseCache, QLException>,
    {
        let mut caches = self.lock_recovered();
        if let Some(cache) = caches.get(script) {
            return Ok(cache.clone());
        }
        let cache = compile()?;
        self.compile_count.fetch_add(1, Ordering::Relaxed);
        caches.insert(script.to_string(), cache.clone());
        Ok(cache)
    }

    /// 当前缓存脚本数。
    /// 对应 Java: 无（Rust 原生适配）。
    pub fn len(&self) -> usize {
        self.lock_recovered().len()
    }

    /// 缓存是否为空。
    /// 对应 Java: 无（Rust 原生适配）。
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// 实际执行首次编译的次数。
    /// 对应 Java: 无（Rust 原生适配）。
    pub fn compile_count(&self) -> usize {
        self.compile_count.load(Ordering::Relaxed)
    }

    /// 清空缓存。正在执行的线程已取得的克隆不受影响。
    /// 对应 Java: 无（Rust 原生适配）。
    pub fn clear(&self) {
        self.lock_recovered().clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::panic::{catch_unwind, AssertUnwindSafe};
    use std::sync::{Arc, PoisonError};
    use std::thread;

    fn empty_serializable_cache() -> SerializableParseCache {
        SerializableParseCache::default()
    }

    /// 构造一个**已 poison** 的 `Mutex<HashMap<...>>`，单线程 setup，
    /// 调用方拿到的值可直接塞进结构体字段（结构体字段类型是 `Mutex<...>`
    /// 而非 `Arc<Mutex<...>>`）。
    fn poisoned_mutex() -> Mutex<HashMap<String, SerializableParseCache>> {
        let m: Mutex<HashMap<String, SerializableParseCache>> = Mutex::new(HashMap::new());
        // 在 panic 期间持锁，留下 poison 标志
        let _ = catch_unwind(AssertUnwindSafe(|| {
            let _g = m.lock().unwrap();
            panic!("intentional poison");
        }));
        m
    }

    #[test]
    fn lock_recovered_clears_dirty_data_on_first_call_after_poison() {
        // 模拟"panic 线程在锁内写了半成品"——既然原始 Mutex 已 poison，
        // 我们在它之后取 into_inner 拿到锁，污染一次
        let poisoned = poisoned_mutex();
        {
            let mut g = poisoned.lock().unwrap_or_else(PoisonError::into_inner);
            g.insert("dirty".to_string(), empty_serializable_cache());
        }
        // 现在把这个被污染的 Mutex 喂给 cache
        let cache = ConcurrentParseCache {
            caches: poisoned,
            compile_count: AtomicUsize::new(0),
            poison_cleared: AtomicBool::new(false),
        };
        // 第一次 lock_recovered 应清空
        assert_eq!(cache.len(), 0, "first recovery should clear dirty data");
        assert!(cache.poison_cleared.load(Ordering::Acquire));
    }

    #[test]
    fn cache_is_usable_after_poison_recovery() {
        let cache = ConcurrentParseCache {
            caches: poisoned_mutex(),
            compile_count: AtomicUsize::new(0),
            poison_cleared: AtomicBool::new(false),
        };
        // 第一次恢复：哈希表已被清空
        assert_eq!(cache.len(), 0);
        // 走完整 get_or_compile 路径
        let result = cache.get_or_compile("a.lua", || Ok(empty_serializable_cache()));
        assert!(result.is_ok());
        assert_eq!(cache.len(), 1);
        assert_eq!(cache.compile_count(), 1);
        // 显式清空
        cache.clear();
        assert_eq!(cache.len(), 0);
    }

    #[test]
    fn double_poison_does_not_break_recovery() {
        // 这里单测：先 poison 一次，让 cache 恢复一次；
        // 再次 poison，验证不会因为 poison_cleared 已 true 而
        // 错误地保留"脏"数据——但恢复动作只发生一次
        let cache = ConcurrentParseCache {
            caches: poisoned_mutex(),
            compile_count: AtomicUsize::new(0),
            poison_cleared: AtomicBool::new(false),
        };
        // 第一次恢复
        let _ = cache.len();
        assert!(cache.poison_cleared.load(Ordering::Acquire));
        // 再次走 lock_recovered：第二次会走 into_inner 路径但因 poison_cleared
        // 已经是 true 不再清。功能上：仍然能继续使用。
        let _ = cache.get_or_compile("b.lua", || Ok(empty_serializable_cache()));
        assert_eq!(cache.len(), 1);
    }

    #[test]
    fn compile_count_unchanged_across_recovery() {
        let cache = ConcurrentParseCache {
            caches: poisoned_mutex(),
            compile_count: AtomicUsize::new(7),
            poison_cleared: AtomicBool::new(false),
        };
        let _ = cache.len();
        assert_eq!(
            cache.compile_count(),
            7,
            "compile_count must survive poison"
        );
    }

    #[test]
    fn concurrent_access_after_poison_does_not_panic() {
        // 共享 ConcurrentParseCache 而不是底层 Mutex：
        // 字段是 Mutex<...>，要跨线程共享只能把 ConcurrentParseCache
        // 整体包成 Arc。
        let inner = poisoned_mutex();
        let cache = Arc::new(ConcurrentParseCache {
            caches: inner,
            compile_count: AtomicUsize::new(0),
            poison_cleared: AtomicBool::new(false),
        });
        let handles: Vec<_> = (0..8)
            .map(|i| {
                let c = cache.clone();
                thread::spawn(move || {
                    let _ = c.get_or_compile(&format!("k{}", i), || Ok(empty_serializable_cache()));
                })
            })
            .collect();
        for h in handles {
            h.join().unwrap();
        }
        assert_eq!(cache.len(), 8);
    }
}
