//! 默认类型供应器,对应 Java `com.alibaba.qlexpress4.DefaultClassSupplier`。
//!
//! Java 版以 `Class.forName` 探测类路径并缓存结果;Rust 无类路径,
//! 按 SPEC §4 改为显式注册类型名集合(SPEC §6 关键决策)。

use std::collections::HashSet;

use crate::class_supplier::ClassSupplier;

/// 默认类型供应器。对应 Java: com.alibaba.qlexpress4.DefaultClassSupplier
/// (单例 `getInstance()`;`loadCls` 命中缓存的已知类型名)。
///
/// Rust 以显式注册的类型名集合替代 Java 的 `Class.forName` 探测;
/// `register` 对应「让该类对脚本可见」的宿主动作。
#[derive(Clone, Debug, Default)]
pub struct DefaultClassSupplier {
    /// 已知类型名集合(Java 的 `cache` 中命中项)。
    registered: HashSet<String>,
}

impl DefaultClassSupplier {
    /// Java `DefaultClassSupplier.getInstance()` — 一个空供应器。
    pub fn instance() -> Self {
        DefaultClassSupplier::default()
    }

    /// 注册一个可加载类型名(Rust 替代类路径可见性)。
    pub fn register(&mut self, qualified_name: impl Into<String>) {
        self.registered.insert(qualified_name.into());
    }
}

impl ClassSupplier for DefaultClassSupplier {
    /// Java `loadCls(String)`:已知返回规范名,否则 `None`
    /// (Java 返回 `null`)。
    fn load_cls(&self, qualified_name: &str) -> Option<String> {
        self.registered.get(qualified_name).cloned()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn registered_names_load() {
        let mut supplier = DefaultClassSupplier::instance();
        assert_eq!(supplier.load_cls("java.lang.String"), None);
        supplier.register("java.lang.String");
        assert_eq!(
            supplier.load_cls("java.lang.String"),
            Some("java.lang.String".to_string())
        );
    }
}
