//! 原生成员加载门面，对应 Java
//! `com.alibaba.qlexpress4.runtime.ReflectLoader`。
//!
//! Java 通过 JVM 反射发现构造器、方法和字段；Rust 没有运行时 JVM，
//! 因而把“发现”改为显式注册，但仍由本对象承担相同的加载、安全过滤和
//! 生命周期职责，实际索引存储在 [`NativeRegistry`]。

use std::rc::Rc;

use crate::exception::error_codes;
use crate::exception::error_reporter::ErrorReporter;
use crate::exception::QLException;
use crate::runtime::class_ref::ClassRef;
use crate::runtime::function::{as_native_method, ExtensionFunction};
use crate::runtime::native_registry::NativeRegistry;
use crate::runtime::native_type::{NativeConstructor, NativeMethod, NativeMethodCandidate};
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

    /// 注册一个成员扩展函数。
    ///
    /// 对应 Java：`ReflectLoader#addExtendFunction(ExtensionFunction)`。
    /// 扩展函数按声明类和方法名进入独立扩展表，并保持 Java
    /// `loadExtendFunction` 在成员安全策略之前解析的顺序。
    ///
    /// # 参数
    ///
    /// - `extension_function`：声明接收者类型、方法签名与调用实现的扩展。
    pub fn add_extend_function<F>(&self, extension_function: F)
    where
        F: ExtensionFunction + 'static,
    {
        let method_name = extension_function.name().to_string();
        let declaring_class = extension_function.declaring_class();
        let parameter_types = extension_function.parameter_types();
        let var_args = extension_function.is_var_args();
        self.registry.register_extension_candidate(
            declaring_class,
            method_name,
            NativeMethodCandidate::new(
                parameter_types,
                var_args,
                as_native_method(extension_function),
            ),
        );
    }

    /// 是否允许宿主注册非公开成员。对应 Java 字段 `allowPrivateAccess`。
    pub fn allow_private_access(&self) -> bool {
        self.allow_private_access
    }

    /// 按运行时实参加载最佳注册构造器。
    ///
    /// 对应 Java 方法 `loadConstructor(Class<?>, Class<?>[])`。Rust 直接接收
    /// 运行时值，由 [`NativeRegistry`] 计算与 Java `MemberResolver` 相同的
    /// 参数类型、重载优先级及必要转换。
    ///
    /// # 参数
    ///
    /// - `class_ref`：待实例化的 Java 规范类型。
    /// - `arguments`：本次构造调用的运行时实参。
    ///
    /// # 返回值
    ///
    /// 返回完成参数适配的最佳构造器；隔离策略拒绝或无匹配项时返回 `None`。
    pub fn load_constructor(
        &self,
        class_ref: &ClassRef,
        arguments: &[DataValue],
    ) -> Option<NativeConstructor> {
        self.registry
            .load_constructor_for_args(class_ref, arguments)
    }

    /// 加载对象字段。
    ///
    /// 对应 Java 方法
    /// `loadField(Object, String, boolean, ErrorReporter)`。Rust 原生成员闭包
    /// 直接返回结构化 [`QLException`]，因此无需把 `ErrorReporter` 作为解析
    /// 参数传递；`skip_security` 的可观察分支完整保留。
    ///
    /// # 参数
    ///
    /// - `bean`：字段接收者。
    /// - `field_name`：字段、属性或 Map 键名称。
    /// - `skip_security`：是否跳过成员安全策略；宿主门面使用 `true`，脚本
    ///   指令使用 `false`。
    pub fn load_field(
        &self,
        bean: &DataValue,
        field_name: &str,
        skip_security: bool,
    ) -> Option<QValue> {
        self.registry
            .load_field_with_security(bean, field_name, skip_security)
    }

    /// 按运行时实参加载对象方法。
    ///
    /// 对应 Java 方法 `loadMethod(Object, String, Class<?>[])`。扩展函数仍在
    /// 成员安全策略之前解析；注册方法随后按静态/实例、别名、重载优先级和
    /// 安全策略选择。
    ///
    /// # 参数
    ///
    /// - `bean`：实例接收者或 [`crate::runtime::meta_class::MetaClass`]。
    /// - `method_name`：脚本方法名或别名。
    /// - `arguments`：本次调用的运行时实参。
    pub fn load_method(
        &self,
        bean: &DataValue,
        method_name: &str,
        arguments: &[DataValue],
    ) -> Option<NativeMethod> {
        self.registry
            .resolve_method_for_args(bean, method_name, arguments)
    }

    /// 获取当前成员安全策略。对应 Java `securityStrategy` 字段。
    pub fn security_strategy(&self) -> QLSecurityStrategy {
        self.registry.security_strategy()
    }

    /// 将原生方法调用失败转换为带脚本位置的运行时异常。
    ///
    /// 对应 Java 静态方法
    /// `ReflectLoader#unwrapMethodInvokeEx(ErrorReporter, String, Exception)`。
    /// Rust 原生闭包不会产生 JVM `InvocationTargetException` 对象，但
    /// [`QLException`] 保留了等价来源标记：
    ///
    /// - 参数转换/分派错误码对应 Java `IllegalArgumentException`；
    /// - `host_origin` 对应被 `InvocationTargetException` 包装的调用目标错误；
    /// - 其余引擎/适配层错误对应 Java 未知反射错误。
    pub fn unwrap_method_invoke_ex(
        error_reporter: &dyn ErrorReporter,
        method_name: &str,
        error: QLException,
    ) -> QLException {
        if error.error_code() == error_codes::INVOKE_METHOD_WITH_WRONG_ARGUMENTS {
            return error_reporter.report_format(
                error_codes::INVOKE_METHOD_WITH_WRONG_ARGUMENTS,
                error_codes::error_msg(error_codes::INVOKE_METHOD_WITH_WRONG_ARGUMENTS),
                &[method_name.to_string()],
            );
        }
        let error_code = if error.is_host_origin() {
            error_codes::INVOKE_METHOD_INNER_ERROR
        } else {
            error_codes::INVOKE_METHOD_UNKNOWN_ERROR
        };
        error_reporter
            .report_format(
                error_code,
                error_codes::error_msg(error_code),
                &[method_name.to_string()],
            )
            .with_cause(error)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::exception::pure_err_reporter::PureErrReporter;
    use crate::exception::QLExceptionKind;
    use crate::runtime::meta_class::MetaClass;
    use crate::runtime::native_type::{NativeConstructorCandidate, NativeType};
    use crate::security::ql_security_strategy::QLSecurityStrategy;

    #[test]
    fn delegates_builtin_member_loading() {
        let loader = ReflectLoader::new(QLSecurityStrategy::open(), false);
        assert!(loader
            .load_method(&DataValue::string("abc"), "length", &[])
            .is_some());
        assert!(!loader.allow_private_access());
    }

    /// SOURCE_PARITY: `ReflectLoader#loadConstructor` 必须把调用现场类型交给
    /// `MemberResolver`，不能固定返回无签名构造器。
    #[test]
    fn constructor_loading_uses_runtime_argument_types() {
        let loader = ReflectLoader::new(QLSecurityStrategy::open(), false);
        let class_ref = ClassRef::from_name("test.OverloadedConstructor");
        let mut native_type = NativeType::named(class_ref.java_name());
        native_type.add_constructor_candidate(NativeConstructorCandidate::new(
            vec![ClassRef::from_name("java.lang.Integer")],
            false,
            Rc::new(|_| Ok(DataValue::string("integer"))),
        ));
        native_type.add_constructor_candidate(NativeConstructorCandidate::new(
            vec![ClassRef::from_name("java.lang.Long")],
            false,
            Rc::new(|_| Ok(DataValue::string("long"))),
        ));
        loader.registry().register_type(native_type);

        let arguments = [DataValue::Int(1)];
        let constructor = loader
            .load_constructor(&class_ref, &arguments)
            .expect("Integer exact-match constructor");
        assert_eq!(
            constructor(&arguments).expect("constructor invocation"),
            DataValue::string("integer")
        );
    }

    /// SOURCE_PARITY: `ReflectLoader#loadMethod` 以实参类型选择同名重载。
    #[test]
    fn method_loading_uses_runtime_argument_types() {
        let loader = ReflectLoader::new(QLSecurityStrategy::open(), false);
        let class_ref = ClassRef::from_name("test.OverloadedStatic");
        let mut native_type = NativeType::named(class_ref.java_name());
        native_type.add_static_method_candidate(
            "choose",
            NativeMethodCandidate::new(
                vec![ClassRef::from_name("java.lang.Integer")],
                false,
                Rc::new(|_, _| Ok(DataValue::string("integer"))),
            ),
        );
        native_type.add_static_method_candidate(
            "choose",
            NativeMethodCandidate::new(
                vec![ClassRef::from_name("java.lang.Long")],
                false,
                Rc::new(|_, _| Ok(DataValue::string("long"))),
            ),
        );
        loader.registry().register_type(native_type);
        let bean = MetaClass::new(class_ref).into_data_value();
        let arguments = [DataValue::Int(1)];

        let method = loader
            .load_method(&bean, "choose", &arguments)
            .expect("Integer exact-match method");
        assert_eq!(
            method(&bean, &arguments).expect("method invocation"),
            DataValue::string("integer")
        );
    }

    /// SOURCE_PARITY: Java 宿主 `Express4Runner#loadField` 传
    /// `skipSecurity=true`，而脚本访问传 `false`。
    #[test]
    fn field_loading_preserves_skip_security_branch() {
        let loader = ReflectLoader::new(QLSecurityStrategy::isolation(), false);
        let class_ref = ClassRef::from_name("test.Fields");
        let mut native_type = NativeType::named(class_ref.java_name());
        native_type
            .static_fields
            .insert("answer".to_string(), DataValue::Int(42));
        loader.registry().register_type(native_type);
        let bean = MetaClass::new(class_ref).into_data_value();

        assert!(loader.load_field(&bean, "answer", false).is_none());
        assert_eq!(
            loader
                .load_field(&bean, "answer", true)
                .expect("host field access bypasses script security")
                .get(),
            DataValue::Int(42)
        );
    }

    /// SOURCE_PARITY: Java `unwrapMethodInvokeEx` 对
    /// `IllegalArgumentException`、`InvocationTargetException` 和其他异常
    /// 使用三个不同错误码，并在后两支保留 cause。
    #[test]
    fn unwrap_method_errors_preserves_all_three_java_branches() {
        let reporter = PureErrReporter::INSTANCE;
        let wrong = ReflectLoader::unwrap_method_invoke_ex(
            &reporter,
            "run",
            QLException::for_test(
                QLExceptionKind::Runtime,
                "wrong",
                error_codes::INVOKE_METHOD_WITH_WRONG_ARGUMENTS,
            ),
        );
        assert_eq!(
            wrong.error_code(),
            error_codes::INVOKE_METHOD_WITH_WRONG_ARGUMENTS
        );
        assert!(wrong.cause().is_none());

        let inner = ReflectLoader::unwrap_method_invoke_ex(
            &reporter,
            "run",
            QLException::host_error(QLExceptionKind::Runtime, "target", "TARGET"),
        );
        assert_eq!(inner.error_code(), error_codes::INVOKE_METHOD_INNER_ERROR);
        assert_eq!(inner.cause().expect("target cause").error_code(), "TARGET");

        let unknown = ReflectLoader::unwrap_method_invoke_ex(
            &reporter,
            "run",
            QLException::for_test(QLExceptionKind::Runtime, "adapter", "ADAPTER"),
        );
        assert_eq!(
            unknown.error_code(),
            error_codes::INVOKE_METHOD_UNKNOWN_ERROR
        );
        assert_eq!(
            unknown.cause().expect("adapter cause").error_code(),
            "ADAPTER"
        );
    }
}
