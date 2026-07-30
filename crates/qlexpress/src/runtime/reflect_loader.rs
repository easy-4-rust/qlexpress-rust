//! 原生成员加载门面，对应 Java
//! `com.alibaba.qlexpress4.runtime.ReflectLoader`。
//!
//! Java 通过 JVM 反射发现构造器、方法和字段；Rust 没有运行时 JVM，
//! 因而把“发现”改为显式注册，但仍由本对象承担相同的加载、安全过滤和
//! 生命周期职责，实际索引存储在 [`NativeRegistry`]。

use std::rc::Rc;

use crate::runtime::class_ref::ClassRef;
use crate::runtime::function::{as_native_method, ExtensionFunction};
use crate::runtime::native_registry::NativeRegistry;
use crate::runtime::native_type::{NativeConstructor, NativeMethod};
use crate::runtime::value::{DataValue, QValue};
use crate::security::ql_security_strategy::QLSecurityStrategy;

/// Rust 原生成员加载器。
///
/// 对应 Java: `com.alibaba.qlexpress4.runtime.ReflectLoader`。Java 的
/// `allowPrivateAccess` 在 Rust 中表示“宿主是否选择注册非公开成员”；
/// Rust 不会绕过语言可见性，因此该标志作为注册策略元数据保留。
///
/// Java 的下列内部键/缓存对象只服务 JVM 反射发现；Rust 把已经解析的字段、
/// 方法和扩展函数直接存入 [`NativeRegistry`] 的类型化 `HashMap`，从而保留
/// 重用语义而不保留 JVM `Class`/`Method` 身份：
///
/// - 对应 Java: `com.alibaba.qlexpress4.runtime.ReflectLoader.FieldReflectCache`
/// - 对应 Java: `com.alibaba.qlexpress4.runtime.ReflectLoader.ExtensionMapKey`
/// - 对应 Java: `com.alibaba.qlexpress4.runtime.ReflectLoader.MethodCacheKey`
pub struct ReflectLoader {
    registry: Rc<NativeRegistry>,
    allow_private_access: bool,
}

impl ReflectLoader {
    /// 使用安全策略和私有访问策略创建加载器。
    ///
    /// 对应 Java 构造器 `ReflectLoader(securityStrategy, allowPrivateAccess)`。
    pub fn new(security_strategy: QLSecurityStrategy, allow_private_access: bool) -> Self {
        let registry = NativeRegistry::with_builtins();
        registry.set_security_strategy(security_strategy);
        Self {
            registry: Rc::new(registry),
            allow_private_access,
        }
    }

    /// 以既有注册表创建加载器。Rust 宿主集成便捷入口，Java 无同名方法。
    /// 对应 Java: com.alibaba.qlexpress4.runtime.ReflectLoader#fromRegistry。
    pub fn from_registry(registry: Rc<NativeRegistry>, allow_private_access: bool) -> Self {
        Self {
            registry,
            allow_private_access,
        }
    }

    /// 获取底层原生注册表。
    /// 对应 Java: com.alibaba.qlexpress4.runtime.ReflectLoader#registry。
    pub fn registry(&self) -> &Rc<NativeRegistry> {
        &self.registry
    }

    /// 当注册表尚未被运行时共享时获取可变引用，供宿主注册成员。
    /// 对应 Java: com.alibaba.qlexpress4.runtime.ReflectLoader#registryMut。
    pub fn registry_mut(&mut self) -> Option<&mut NativeRegistry> {
        Rc::get_mut(&mut self.registry)
    }

    /// 注册一个成员扩展函数。
    ///
    /// 对应 Java：`ReflectLoader#addExtendFunction(ExtensionFunction)`。
    /// 扩展函数按声明类和方法名进入独立扩展表，并保持 Java
    /// `loadExtendFunction` 在成员安全策略之前解析的顺序。
    ///
    /// # 参数
    ///
    /// - `extension_function`：声明接收者类型、方法签名与调用实现的扩展。
    pub fn add_extend_function<F>(&mut self, extension_function: F)
    where
        F: ExtensionFunction + 'static,
    {
        let method_name = extension_function.name().to_string();
        let type_name = extension_function.declaring_class().java_name().to_string();
        self.registry_mut()
            .expect("ReflectLoader registry must be uniquely owned while registering extensions")
            .register_method(type_name, method_name, as_native_method(extension_function));
    }

    /// 是否允许宿主注册非公开成员。对应 Java 字段 `allowPrivateAccess`。
    pub fn allow_private_access(&self) -> bool {
        self.allow_private_access
    }

    /// 加载注册构造器。对应 Java 方法 `loadConstructor`。
    pub fn load_constructor(&self, class_ref: &ClassRef) -> Option<NativeConstructor> {
        self.registry.load_constructor(class_ref)
    }

    /// 加载对象字段。对应 Java 方法 `loadField`。
    pub fn load_field(&self, bean: &DataValue, field_name: &str) -> Option<QValue> {
        self.registry
            .load_field_with_security(bean, field_name, true)
    }

    /// 加载对象方法。对应 Java 方法 `loadMethod`。
    pub fn load_method(&self, bean: &DataValue, method_name: &str) -> Option<NativeMethod> {
        self.registry.resolve_method(bean, method_name)
    }

    /// 获取当前成员安全策略。对应 Java `securityStrategy` 字段。
    pub fn security_strategy(&self) -> QLSecurityStrategy {
        self.registry.security_strategy()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::security::ql_security_strategy::QLSecurityStrategy;

    #[test]
    fn delegates_builtin_member_loading() {
        let loader = ReflectLoader::new(QLSecurityStrategy::open(), false);
        assert!(loader
            .load_method(&DataValue::Str("abc".to_string()), "length")
            .is_some());
        assert!(!loader.allow_private_access());
    }
}
