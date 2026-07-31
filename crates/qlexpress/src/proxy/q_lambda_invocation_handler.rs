//! QLambda 动态代理调用处理器,对应 Java
//! `com.alibaba.qlexpress4.proxy.QLambdaInvocationHandler`。
//!
//! Java 语义说明(动态代理 → Rust 替代方案):
//! Java 版实现 `java.lang.reflect.InvocationHandler`,配合
//! `Proxy.newProxyInstance` 把脚本 Lambda 包装成**任意接口**的运行时实现:
//! - 抽象方法 → 转发给 `QLambda.call(args)`;
//! - `toString()` → 固定返回 `"QLambdaProxy"`;
//! - 其他默认方法 → 原样调用。
//!
//! Rust 没有运行时接口代理,等价物是**显式闭包/trait 适配器**:宿主为
//! 目标 trait 手写一个适配器,方法体调用本处理器的
//! [`QLambdaInvocationHandler::invoke_abstract`]。本类型保留 Java 的
//! 分派语义(抽象方法转发 / `to_string` 固定值),供适配器复用。

use std::rc::Rc;

use crate::exception::QLException;
use crate::runtime::qlambda::QLambda;
use crate::runtime::value::DataValue;

/// Java 代理上 `toString()` 的固定返回值。
pub const Q_LAMBDA_PROXY_TO_STRING: &str = "QLambdaProxy";

/// 脚本 Lambda 的调用处理器。对应 Java:
/// com.alibaba.qlexpress4.proxy.QLambdaInvocationHandler
/// (`implements InvocationHandler`,持有被代理的 `QLambda`)。
pub struct QLambdaInvocationHandler {
    /// 被代理的脚本 Lambda。对应 Java 字段 `qLambda`。
    q_lambda: Rc<QLambda>,
}

impl QLambdaInvocationHandler {
    /// 对应 Java 构造器 `QLambdaInvocationHandler(QLambda)`。
    pub fn new(q_lambda: Rc<QLambda>) -> Self {
        QLambdaInvocationHandler { q_lambda }
    }

    /// 抽象方法分派。对应 Java `invoke` 中
    /// `Modifier.isAbstract(method.getModifiers())` 为 true 的分支:
    /// 转发 `qLambda.call(args)` 并取结果值
    /// (Java `.getResult().get()`)。
    pub fn invoke_abstract(&self, args: &[DataValue]) -> Result<DataValue, QLException> {
        Ok(self.q_lambda.call(args)?.value())
    }

    /// `toString` 分派。对应 Java `invoke` 中
    /// `method.getReturnType() == String.class && "toString".equals(...)`
    /// 分支:固定返回 `"QLambdaProxy"`。
    pub fn invoke_to_string(&self) -> String {
        Q_LAMBDA_PROXY_TO_STRING.to_string()
    }

    /// 便捷适配:把处理器转成可多次调用的闭包(Java 代理实例的 Rust
    /// 等价物——宿主把该闭包塞进自己的 trait 适配器)。
    /// 对应 Java：`Proxy.newProxyInstance(..., QLambdaInvocationHandler)` 的调用适配。
    pub fn into_fn(self) -> impl Fn(&[DataValue]) -> Result<DataValue, QLException> {
        move |args| self.invoke_abstract(args)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runtime::data::lambda::QLambdaMethod;
    use crate::runtime::native_registry::NativeRegistry;
    use crate::runtime::qlambda_empty::QLambdaEmpty;

    fn method_handler(method_name: &str, bean: DataValue) -> QLambdaInvocationHandler {
        QLambdaInvocationHandler::new(Rc::new(QLambda::Method(QLambdaMethod::new(
            method_name,
            Rc::new(NativeRegistry::with_builtins()),
            bean,
        ))))
    }

    #[test]
    fn to_string_is_fixed_proxy_name() {
        let handler = QLambdaInvocationHandler::new(Rc::new(QLambda::Empty(QLambdaEmpty)));
        assert_eq!(handler.invoke_to_string(), "QLambdaProxy");
    }

    /// `SOURCE_PARITY`：Java `invoke` 的抽象方法分支把全部参数原序转发给
    /// `QLambda.call(args)`，并返回 `QResult` 中的值。
    #[test]
    fn abstract_dispatch_forwards_arguments_and_result() {
        let handler = method_handler("equals", DataValue::string("same"));
        assert_eq!(
            handler
                .invoke_abstract(&[DataValue::string("same")])
                .expect("abstract dispatch"),
            DataValue::Bool(true)
        );

        let callable = method_handler("substring", DataValue::string("value")).into_fn();
        assert_eq!(
            callable(&[DataValue::Int(1)]).expect("closure adapter"),
            DataValue::string("alue")
        );
    }

    /// `SOURCE_PARITY`：Java 非抽象接口默认方法不转发到脚本 Lambda；
    /// Rust trait 的默认实现同样在适配器自身执行，只有抽象方法调用处理器。
    #[test]
    fn rust_trait_adapter_keeps_default_methods_outside_handler() {
        trait HostFunction {
            fn apply(&self, argument: DataValue) -> Result<DataValue, QLException>;

            fn description(&self) -> &'static str {
                "host-default"
            }
        }

        struct Adapter {
            handler: QLambdaInvocationHandler,
        }

        impl HostFunction for Adapter {
            fn apply(&self, argument: DataValue) -> Result<DataValue, QLException> {
                self.handler.invoke_abstract(&[argument])
            }
        }

        let adapter = Adapter {
            handler: method_handler("equals", DataValue::string("same")),
        };
        assert_eq!(adapter.description(), "host-default");
        assert_eq!(
            adapter
                .apply(DataValue::string("same"))
                .expect("abstract method"),
            DataValue::Bool(true)
        );
    }
}
