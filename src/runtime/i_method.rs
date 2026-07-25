//! 可调用方法抽象,对应 Java `com.alibaba.qlexpress4.runtime.IMethod`。

use crate::exception::QLException;
use crate::runtime::class_ref::ClassRef;
use crate::runtime::value::DataValue;

/// 方法抽象。对应 Java: com.alibaba.qlexpress4.runtime.IMethod
/// (职责:屏蔽「JVM 反射方法」与「扩展函数」等不同方法来源,
/// 向 `MemberResolver`/`MethodInvokeUtils` 提供统一的签名与调用面)。
///
/// Rust 无反射,实现者包括:
/// - [`crate::runtime::jvm_i_method::NativeIMethod`](对应 `JvmIMethod`,
///   包装注册进来的原生闭包);
/// - [`crate::runtime::function::ExtensionFunction`](脚本扩展函数,
///   通过 blanket impl 自动获得 `IMethod`)。
pub trait IMethod {
    /// 对应 Java 方法 `getParameterTypes()`。
    fn parameter_types(&self) -> Vec<ClassRef>;

    /// 对应 Java 方法 `isVarArgs()`。
    fn is_var_args(&self) -> bool;

    /// 对应 Java 方法 `isAccess()`(方法是否已可访问,无需 `setAccessible`)。
    fn is_access(&self) -> bool;

    /// 对应 Java 方法 `setAccessible(boolean)`。
    /// Rust 无访问检查,实现通常记录标志或为空操作(与 Java 中
    /// `ExtensionFunction.setAccessible` 的空实现一致)。
    fn set_accessible(&self, flag: bool);

    /// 对应 Java 方法 `getName()`。
    fn name(&self) -> &str;

    /// 对应 Java 方法 `getDeclaringClass()`。
    fn declaring_class(&self) -> ClassRef;

    /// 对应 Java 方法 `invoke(Object obj, Object[] args)`。
    /// `obj` 为接收者(静态方法时 Java 传 `null`,Rust 传 `DataValue::Null`)。
    fn invoke(&self, obj: &DataValue, args: &[DataValue]) -> Result<DataValue, QLException>;
}
