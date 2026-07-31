//! NativeRegistry 的派生类型注册扩展。

use crate::runtime::ql_express_native_type::QLExpressNativeType;

/// 将 `QLExpressNativeType` 元数据注册到当前原生注册表。
///
/// Rust 原生扩展，替代 Java `ReflectLoader` 的运行时反射发现。
/// 对应 Java：无（Rust 显式注册替代 `ReflectLoader` 运行时发现）。
pub trait QLExpressRegistryExt {
    /// 注册类型 `T` 的字段、方法和构造器元数据。
    fn register_qlexpress_type<T: QLExpressNativeType>(&mut self);
}
