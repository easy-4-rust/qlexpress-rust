//! 原生方法包装,对应 Java `com.alibaba.qlexpress4.runtime.JvmIMethod`。
//!
//! 适配说明(SPEC §4):Java `JvmIMethod` 持有 `java.lang.reflect.Method`,
//! 签名与调用全部来自反射;Rust 无反射,改为显式注册的
//! [`NativeMethod`] 闭包 + 手工登记的签名(方法名/声明类/参数类型/
//! 是否可变参数),对 `IMethod` 消费者提供与 Java 完全一致的行为。

use std::cell::Cell;
use std::rc::Rc;

use crate::exception::QLException;
use crate::runtime::class_ref::ClassRef;
use crate::runtime::i_method::IMethod;
use crate::runtime::native_type::NativeMethod;
use crate::runtime::value::DataValue;

/// 原生方法包装。对应 Java: com.alibaba.qlexpress4.runtime.JvmIMethod
/// (职责:把一个「方法」包装为 `IMethod`;Java 包装反射 `Method`,
/// Rust 包装注册闭包与显式签名)。
pub struct NativeIMethod {
    /// 方法名(Java `Method.getName()`)。
    name: String,
    /// 声明类(Java `Method.getDeclaringClass()`)。
    declaring_class: ClassRef,
    /// 参数类型列表(Java `Method.getParameterTypes()`)。
    parameter_types: Vec<ClassRef>,
    /// 是否可变参数(Java `Method.isVarArgs()`)。
    var_args: bool,
    /// 可访问标志(Java `Method.setAccessible` 的状态;
    /// Rust 无访问检查,仅记录以复现 `isAccess` 语义)。
    accessible: Cell<bool>,
    /// 实际调用体(Java `Method.invoke`;`fn(接收者, 参数) -> 结果`)。
    method: NativeMethod,
}

impl NativeIMethod {
    /// 对应 Java 构造器 `JvmIMethod(Method method)`;Rust 需显式给出签名。
    pub fn new(
        name: impl Into<String>,
        declaring_class: ClassRef,
        parameter_types: Vec<ClassRef>,
        var_args: bool,
        method: NativeMethod,
    ) -> Self {
        NativeIMethod {
            name: name.into(),
            declaring_class,
            parameter_types,
            var_args,
            accessible: Cell::new(true),
            method,
        }
    }

    /// 便捷构造:从 [`NativeMethod`] 包装(对应 `QMethodFunction` 把注册的
    /// Java 方法包成函数的场景)。
    pub fn from_native(
        name: impl Into<String>,
        declaring_class: ClassRef,
        parameter_types: Vec<ClassRef>,
        method: NativeMethod,
    ) -> Rc<dyn IMethod> {
        Rc::new(Self::new(name, declaring_class, parameter_types, false, method))
    }
}

impl IMethod for NativeIMethod {
    /// 对应 Java `getParameterTypes()`。
    fn parameter_types(&self) -> Vec<ClassRef> {
        self.parameter_types.clone()
    }

    /// 对应 Java `isVarArgs()`。
    fn is_var_args(&self) -> bool {
        self.var_args
    }

    /// 对应 Java `isAccess()`(Java 取声明类与方法修饰符的 public 判定;
    /// Rust 注册即视为 public,除非被 `set_accessible(false)` 标记)。
    fn is_access(&self) -> bool {
        self.accessible.get()
    }

    /// 对应 Java `setAccessible(boolean)`。
    fn set_accessible(&self, flag: bool) {
        self.accessible.set(flag);
    }

    /// 对应 Java `getName()`。
    fn name(&self) -> &str {
        &self.name
    }

    /// 对应 Java `getDeclaringClass()`。
    fn declaring_class(&self) -> ClassRef {
        self.declaring_class.clone()
    }

    /// 对应 Java `invoke(Object, Object[])`。
    fn invoke(&self, obj: &DataValue, args: &[DataValue]) -> Result<DataValue, QLException> {
        (self.method)(obj, args)
    }
}
