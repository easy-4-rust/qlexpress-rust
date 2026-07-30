//! `@QLFunction` 宿主扫描入口的 Rust 适配契约。

use super::ql_function_method::QLFunctionMethod;

/// 提供实例方法与静态方法的 `@QLFunction` 扫描描述。
///
/// 对应 Java `Express4Runner#addObjFunction(Object)` /
/// `addStaticFunction(Class<?>)` 对 `getDeclaredMethods()` 的遍历。Rust 无法
/// 在运行时枚举任意类型的 impl 方法，因此宿主显式返回完整的方法清单；
/// 清单必须包含未标注和非公开方法，才能保留 Java 的失败归集语义。
pub trait QLFunctionProvider {
    /// 返回当前实例的全部声明方法描述。
    ///
    /// 每个调用实现应已绑定 `self`；默认空列表便于仅提供静态函数的类型。
    fn ql_object_function_methods(&self) -> Vec<QLFunctionMethod> {
        Vec::new()
    }

    /// 返回类型的全部声明方法描述。
    ///
    /// 默认空列表便于仅提供实例函数的类型。
    fn ql_static_function_methods() -> Vec<QLFunctionMethod>
    where
        Self: Sized,
    {
        Vec::new()
    }
}
