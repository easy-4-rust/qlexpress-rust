//! 实例方法扩展函数,对应 Java `com.alibaba.qlexpress4.runtime.function.ExtensionFunction`。

use crate::exception::QLException;
use crate::runtime::class_ref::ClassRef;
use crate::runtime::i_method::IMethod;
use crate::runtime::value::DataValue;

/// 扩展函数:为既有类型追加「实例方法」。
/// 对应 Java: com.alibaba.qlexpress4.runtime.function.ExtensionFunction
/// (抽象类,实现 `IMethod`;职责:以 `target + methodName` 的形式
/// 让脚本把函数当某类型的实例方法调用,如 `list.filter(...)`)。
///
/// Java 语义要点:Java 版固定 `isVarArgs() == false`、`isAccess() == true`、
/// `setAccessible` 空实现;Rust 以默认方法复现同一契约。
/// 声明类由 [`ExtensionFunction::declaring_class`] 给出(Java
/// `getDeclaringClass`),运行期成员解析据此把调用路由到扩展函数
/// (对应 Java `Express4Runner.addExtensionFunction` 的注册效果)。
pub trait ExtensionFunction {
    /// 对应 Java 方法 `getParameterTypes()`(来自 `IMethod`)。
    fn parameter_types(&self) -> Vec<ClassRef>;

    /// 对应 Java 方法 `getName()`(来自 `IMethod`):扩展出的方法名。
    fn name(&self) -> &str;

    /// 对应 Java 方法 `getDeclaringClass()`(来自 `IMethod`):
    /// 被扩展的目标类型(Java 中为 `List.class` 等)。
    fn declaring_class(&self) -> ClassRef;

    /// 对应 Java 方法 `invoke(Object obj, Object[] args)`(来自 `IMethod`):
    /// `obj` 即扩展目标实例(Java `target`),`args` 为调用参数。
    fn invoke(&self, obj: &DataValue, args: &[DataValue]) -> Result<DataValue, QLException>;

    /// 对应 Java `isVarArgs()`:固定 false(Java `@Override` 写死)。
    fn is_var_args(&self) -> bool {
        false
    }

    /// 对应 Java `isAccess()`:固定 true(Java `@Override` 写死)。
    fn is_access(&self) -> bool {
        true
    }

    /// 对应 Java `setAccessible(boolean)`:空实现(Java 方法体为空)。
    fn set_accessible(&self, flag: bool) {
        let _ = flag;
    }
}

/// 让任何 [`ExtensionFunction`] 自动成为 [`IMethod`]
/// (对应 Java `ExtensionFunction implements IMethod` 的继承关系)。
impl<T: ExtensionFunction> IMethod for T {
    fn parameter_types(&self) -> Vec<ClassRef> {
        ExtensionFunction::parameter_types(self)
    }

    fn is_var_args(&self) -> bool {
        ExtensionFunction::is_var_args(self)
    }

    fn is_access(&self) -> bool {
        ExtensionFunction::is_access(self)
    }

    fn set_accessible(&self, flag: bool) {
        ExtensionFunction::set_accessible(self, flag)
    }

    fn name(&self) -> &str {
        ExtensionFunction::name(self)
    }

    fn declaring_class(&self) -> ClassRef {
        ExtensionFunction::declaring_class(self)
    }

    fn invoke(&self, obj: &DataValue, args: &[DataValue]) -> Result<DataValue, QLException> {
        ExtensionFunction::invoke(self, obj, args)
    }
}

/// 把 [`ExtensionFunction`] 适配为可直接注册进 `NativeType`/`NativeRegistry`
/// 的原生方法闭包(Rust 适配辅助,Java 无对应物):
/// 闭包忽略注册表的接收者分派,直接转发 `invoke(target, args)`。
/// 对应 Java: com.alibaba.qlexpress4.runtime.function.ExtensionFunction#asNativeMethod。
pub fn as_native_method<F: ExtensionFunction + 'static>(
    extension: F,
) -> crate::runtime::native_type::NativeMethod {
    let extension = std::rc::Rc::new(extension);
    std::rc::Rc::new(move |bean, args| extension.invoke(bean, args))
}

/// 可共享的扩展函数句柄(Rust 适配辅助,等价于 Java 单例 `INSTANCE` 的共享语义)。
/// 对应 Java: com.alibaba.qlexpress4.runtime.function.ExtensionFunction。
pub struct SharedExtension<F: ExtensionFunction> {
    extension: F,
}

impl<F: ExtensionFunction> SharedExtension<F> {
    /// 创建共享扩展函数适配器。Rust 共享适配入口，Java 无同名方法。
    /// 对应 Java: com.alibaba.qlexpress4.runtime.function.ExtensionFunction#new。
    pub fn new(extension: F) -> Self {
        SharedExtension { extension }
    }

    /// 取内部扩展函数。
    /// 对应 Java: com.alibaba.qlexpress4.runtime.function.ExtensionFunction#inner。
    pub fn inner(&self) -> &F {
        &self.extension
    }
}
