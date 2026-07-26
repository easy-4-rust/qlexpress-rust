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
