//! Assignable values, mirroring Java `LeftValue`.

use crate::exception::error_codes;
use crate::exception::error_reporter::ErrorReporter;
use crate::exception::QLException;
use crate::runtime::class_ref::ClassRef;
use crate::runtime::data::convert::obj_type_convertor::ObjTypeConvertor;
use crate::runtime::native_registry::NativeRegistry;
use crate::runtime::value::{DataValue, Value};

/// `LeftValue` 接口的 Rust 实现，保留对应对象的领域职责与公开契约。
/// 对应或承接 Java 源文件：`com/alibaba/qlexpress4/runtime/LeftValue.java`；具体对象路径见 `docs/对象级对照表.md`。
/// An assignable `Value`, mirroring Java `LeftValue`.
/// 对应 Java: com.alibaba.qlexpress4.runtime.LeftValue。
pub trait LeftValue: Value {
    /// 处理 defined type 对应的接口职责。
    /// 无显式参数；返回：`Option<ClassRef>`。
    /// 对应或承接 Java 源文件：`com/alibaba/qlexpress4/runtime/LeftValue.java`，方法 `definedType`。
    /// Java `getDefinedType`; `None` mirrors `null` (no declared type).
    fn defined_type(&self) -> Option<ClassRef>;

    /// 返回声明类型校验所使用的宿主类型注册表。
    ///
    /// Java 直接通过 `Class#isInstance` 检查具名引用；Rust 左值在需要时
    /// 保存定义现场的注册表以复现继承关系。
    fn type_registry(&self) -> Option<&NativeRegistry> {
        None
    }

    /// 更新 inner。
    /// 参数：`new_value`；返回：`Result<(), QLException>`。
    /// 对应或承接 Java 源文件：`com/alibaba/qlexpress4/runtime/LeftValue.java`，方法 `setInner`。
    /// Java `setInner`: assign without conversion.
    ///
    /// Returns an error when the underlying storage rejects the assignment
    /// (e.g. a host-registered setter returns `false`).
    fn set_inner(&mut self, new_value: DataValue) -> Result<(), QLException>;

    /// 处理 symbol name 对应的接口职责。
    /// 无显式参数；返回：`Option<&str>`。
    /// 对应或承接 Java 源文件：`com/alibaba/qlexpress4/runtime/LeftValue.java`，方法 `symbolName`。
    /// Java `getSymbolName`; `None` mirrors `null`.
    fn symbol_name(&self) -> Option<&str>;

    /// 处理 set 对应的接口职责。
    /// 参数：`new_value`、`error_reporter`；返回：`Result<(), QLException>`。
    /// 对应或承接 Java 源文件：`com/alibaba/qlexpress4/runtime/LeftValue.java`，方法 `set`。
    /// Java default `set(Object, ErrorReporter)`: convert to the declared
    /// type, reporting `INCOMPATIBLE_ASSIGNMENT_TYPE` when unconvertible.
    ///
    /// Note: the argument order (value type name, declared type name)
    /// faithfully reproduces the Java call, including its original order.
    fn set(
        &mut self,
        new_value: DataValue,
        error_reporter: &dyn ErrorReporter,
    ) -> Result<(), QLException> {
        let define_type = self.defined_type();
        let result =
            ObjTypeConvertor::cast_class(&new_value, define_type.as_ref(), self.type_registry());
        if !result.is_convertible() {
            let value_type = if new_value.is_null() {
                "null".to_string()
            } else {
                new_value.runtime_type_name()
            };
            let define_type_name = define_type
                .as_ref()
                .map(ClassRef::java_name)
                .unwrap_or("java.lang.Object")
                .to_string();
            return Err(error_reporter.report_format(
                error_codes::INCOMPATIBLE_ASSIGNMENT_TYPE,
                error_codes::error_msg(error_codes::INCOMPATIBLE_ASSIGNMENT_TYPE),
                &[value_type, define_type_name],
            ));
        }
        self.set_inner(result.into_converted())
    }
}

/// Convenience for `LeftValue` trait objects (Java uses `LeftValue` as a
/// normal interface).
impl dyn LeftValue {
    /// 返回用于调试显示的当前左值内容。
    /// 无显式参数；返回：`String`。
    /// 对应或承接 Java 源文件：`com/alibaba/qlexpress4/runtime/LeftValue.java`，方法 `debugValue`；Rust 侧按所有权与 `Result` 语义适配。
    /// Helper to format this value for error messages.
    /// 对应 Java: com.alibaba.qlexpress4.runtime.LeftValue#debugValue。
    pub fn debug_value(&self) -> String {
        format!("{:?}", self.get())
    }
}
