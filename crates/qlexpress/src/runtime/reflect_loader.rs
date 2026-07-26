//! 原生成员加载门面，对应 Java
//! `com.alibaba.qlexpress4.runtime.ReflectLoader`。
//!
//! Java 通过 JVM 反射发现构造器、方法和字段；Rust 没有运行时 JVM，
//! 因而把“发现”改为显式注册，但仍由本对象承担相同的加载、安全过滤和
//! 生命周期职责，实际索引存储在 [`NativeRegistry`]。

use std::rc::Rc;

use crate::runtime::class_ref::ClassRef;
use crate::runtime::native_registry::NativeRegistry;
use crate::runtime::native_type::{NativeConstructor, NativeMethod};
use crate::runtime::value::{DataValue, QValue};
use crate::security::ql_security_strategy::QLSecurityStrategy;

/// Rust 原生成员加载器。
///
/// 对应 Java: `com.alibaba.qlexpress4.runtime.ReflectLoader`。Java 的
/// `allowPrivateAccess` 在 Rust 中表示“宿主是否选择注册非公开成员”；
/// Rust 不会绕过语言可见性，因此该标志作为注册策略元数据保留。
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
    pub fn from_registry(registry: Rc<NativeRegistry>, allow_private_access: bool) -> Self {
        Self {
            registry,
            allow_private_access,
        }
    }

    /// 获取底层原生注册表。
    pub fn registry(&self) -> &Rc<NativeRegistry> {
        &self.registry
    }

    /// 当注册表尚未被运行时共享时获取可变引用，供宿主注册成员。
    pub fn registry_mut(&mut self) -> Option<&mut NativeRegistry> {
        Rc::get_mut(&mut self.registry)
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
