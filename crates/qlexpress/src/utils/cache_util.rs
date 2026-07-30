//! 函数式接口判定缓存，对应 Java `com.alibaba.qlexpress4.utils.CacheUtil`。
//!
//! Java 以 `ConcurrentHashMap<Class<?>, Boolean>` 缓存
//! `clazz.isInterface() && MethodHandler.hasOnlyOneAbstractMethod(...)`。
//! Rust 没有 JVM `Class`，由 [`NativeType`] 显式携带接口及抽象方法元数据，
//! 缓存键包含完整元数据，避免不同注册表中同名类型互相污染。

use std::cell::RefCell;
use std::collections::HashMap;
use std::hash::Hash;

use crate::member::method_handler::MethodHandler;
use crate::runtime::native_type::NativeType;

/// Java `CacheUtil` 的 Rust 对应对象。
///
/// 每个 [`crate::runtime::member::NativeRegistry`] 拥有独立实例，等价于在
/// 一个 ClassLoader/宿主模型边界内缓存 `Class -> isFunctionInterface`。
#[derive(Default)]
pub struct CacheUtil {
    function_interface_cache: RefCell<HashMap<FunctionInterfaceKey, bool>>,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
struct FunctionInterfaceKey {
    name: String,
    is_interface: bool,
    abstract_methods: Vec<String>,
}

impl CacheUtil {
    /// 创建空的函数式接口判定缓存。
    ///
    /// 对应 Java：`FUNCTION_INTERFACE_CACHE` 的初始化。
    pub fn new() -> Self {
        Self::default()
    }

    /// 判断已注册类型是否是单抽象方法接口。
    ///
    /// 对应 Java：`CacheUtil#isFunctionInterface(Class<?>)`。首次看到相同
    /// 类型元数据时计算，后续直接返回缓存结果。
    ///
    /// # 参数
    ///
    /// - `native_type`：Rust 显式注册的 Java 类/接口元数据。
    ///
    /// # 返回值
    ///
    /// 仅当类型被标记为接口且恰有一个抽象方法时返回 `true`。
    pub fn is_function_interface(&self, native_type: &NativeType) -> bool {
        let key = FunctionInterfaceKey {
            name: native_type.name.clone(),
            is_interface: native_type.is_interface,
            abstract_methods: native_type.abstract_methods.clone(),
        };
        if let Some(value) = self.function_interface_cache.borrow().get(&key) {
            return *value;
        }
        let abstract_flags = vec![true; native_type.abstract_methods.len()];
        let value = native_type.is_interface
            && MethodHandler::has_only_one_abstract_method(&abstract_flags);
        self.function_interface_cache
            .borrow_mut()
            .insert(key, value);
        value
    }
}

/// 通用的按键一次性计算缓存。
///
/// Java `CacheUtil` 只负责函数式接口；此类型保留早期 Rust API 的通用
/// `computeIfAbsent` 能力，属于 `RUST_EXTENSION`。
pub struct MemoCache<K, V> {
    cache: RefCell<HashMap<K, V>>,
}

impl<K: Eq + Hash, V: Clone> MemoCache<K, V> {
    /// 创建空缓存。
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::Cell;

    /// SOURCE_PARITY: CacheUtil#isFunctionInterface。
    #[test]
    fn detects_and_caches_single_abstract_method_interface() {
        let cache = CacheUtil::new();
        let function = NativeType::interface("example.Function", ["apply"]);
        let ordinary_interface = NativeType::interface("example.Multi", ["a", "b"]);
        let mut class = NativeType::named("example.Class");
        class.abstract_methods.push("run".to_string());

        assert!(cache.is_function_interface(&function));
        assert!(cache.is_function_interface(&function));
        assert!(!cache.is_function_interface(&ordinary_interface));
        assert!(!cache.is_function_interface(&class));
    }

    /// VALUE_ADD: 通用 Rust memo cache 仍保持每键只计算一次。
    #[test]
    fn memo_cache_computes_only_once_per_key() {
        let cache: MemoCache<String, bool> = MemoCache::new();
        let calls = Cell::new(0);
        let compute = |_: &String| {
            calls.set(calls.get() + 1);
            true
        };
        assert!(cache.compute_if_absent("key".to_string(), compute));
        assert!(cache.compute_if_absent("key".to_string(), compute));
        assert_eq!(calls.get(), 1);
    }
}
