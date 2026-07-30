//! Rust 多线程执行模型使用的线程安全编译缓存。

use std::collections::HashMap;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Mutex;

use crate::api::parsecache::SerializableParseCache;
use crate::exception::QLException;

/// 跨线程共享的可序列化编译缓存。
///
/// Java `Express4Runner` 直接共享 `ConcurrentHashMap<String,
/// Future<QCompileCache>>`；Rust Runner 内部含 `Rc/RefCell`，因此采用
/// “线程本地 Runner + 共享纯数据缓存”模型。锁覆盖首次编译，保证相同
/// 脚本在并发冷启动时也只执行一次 `compile`。
#[derive(Default)]
/// 对应 Java: 无（Rust 原生适配）。
pub struct ConcurrentParseCache {
    caches: Mutex<HashMap<String, SerializableParseCache>>,
    compile_count: AtomicUsize,
}

impl ConcurrentParseCache {
    /// 创建空缓存。
    /// 对应 Java: 无（Rust 原生适配）。
    pub fn new() -> Self {
        Self::default()
    }

    /// 返回已有缓存，或在互斥区内执行一次编译并保存结果。
    ///
    /// `compile` 对应线程本地 [`crate::Express4Runner::export_parse_cache`]。
    /// 对应 Java: 无（Rust 原生适配）。
    pub fn get_or_compile<F>(
        &self,
        script: &str,
        compile: F,
    ) -> Result<SerializableParseCache, QLException>
    where
        F: FnOnce() -> Result<SerializableParseCache, QLException>,
    {
        let mut caches = self
            .caches
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
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
        self.caches
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .len()
    }

    /// 缓存是否为空。
    /// 对应 Java: 无（Rust 原生适配）。
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// 实际执行首次编译的次数，供并发验收和监控使用。
    /// 对应 Java: 无（Rust 原生适配）。
    pub fn compile_count(&self) -> usize {
        self.compile_count.load(Ordering::Relaxed)
    }

    /// 清空缓存。正在执行的线程已取得的克隆不受影响。
    /// 对应 Java: 无（Rust 原生适配）。
    pub fn clear(&self) {
        self.caches
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .clear();
    }
}
