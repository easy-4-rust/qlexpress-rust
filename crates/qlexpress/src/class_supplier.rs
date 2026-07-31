//! 类型供应契约,对应 Java `com.alibaba.qlexpress4.ClassSupplier`
//! (`@FunctionalInterface`,`Class<?> loadCls(String clsQualifiedName)`)。
//!
//! Rust 无可执行类路径:可执行注册表(构造器/方法/字段,供
//! GetField/GetMethod/NewInstance/Cast 使用)见
//! [`crate::runtime::native_registry::NativeRegistry`],在此 re-export
//! 以便宿主集成有单一入口;默认实现见
//! [`crate::default_class_supplier::DefaultClassSupplier`]。

pub use crate::default_class_supplier::DefaultClassSupplier;
pub use crate::runtime::member::{
    ClassRef, MetaClass, NativeConstructor, NativeFieldGetter, NativeMethod, NativeRegistry,
    NativeType,
};

/// 脚本侧类型名解析契约。对应 Java: com.alibaba.qlexpress4.ClassSupplier
/// (`loadCls` 返回加载到的 `Class`,未找到返回 `null`)。
pub trait ClassSupplier {
    /// Java `loadCls(String)`:类型已知时返回规范名,否则 `None`。
    fn load_cls(&self, qualified_name: &str) -> Option<String>;
}

/// 让 Rust 闭包直接实现 Java `@FunctionalInterface ClassSupplier`。
impl<F> ClassSupplier for F
where
    F: Fn(&str) -> Option<String>,
{
    fn load_cls(&self, qualified_name: &str) -> Option<String> {
        (self)(qualified_name)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// SOURCE_PARITY: Java `ClassSupplier#loadCls` 是单抽象方法契约；
    /// Rust 闭包必须收到完整类名，并能以 None 表达 Java null。
    #[test]
    fn closure_preserves_functional_interface_contract() {
        let supplier = |name: &str| (name == "example.Host").then(|| name.to_string());
        assert_eq!(
            supplier.load_cls("example.Host"),
            Some("example.Host".to_string())
        );
        assert_eq!(supplier.load_cls("example.Missing"), None);
    }
}
