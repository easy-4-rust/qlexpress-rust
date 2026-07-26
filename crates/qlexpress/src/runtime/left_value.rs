//! Assignable values, mirroring Java `LeftValue`.

use crate::exception::error_codes;
use crate::exception::error_reporter::ErrorReporter;
use crate::exception::QLException;
use crate::runtime::data::convert::obj_type_convertor::{ObjTypeConvertor, TargetType};
use crate::runtime::value::{DataValue, Value};

/// `LeftValue` 接口的 Rust 实现，保留对应对象的领域职责与公开契约。
/// 对应或承接 Java 源文件：`com/alibaba/qlexpress4/runtime/LeftValue.java`；具体对象路径见 `docs/对象级对照表.md`。
/// An assignable `Value`, mirroring Java `LeftValue`.
pub trait LeftValue: Value {
    /// 处理 defined type 对应的接口职责。
    /// 无显式参数；返回：`Option<TargetType>`。
    /// 对应或承接 Java 源文件：`com/alibaba/qlexpress4/runtime/LeftValue.java`，方法 `definedType`。
    /// Java `getDefinedType`; `None` mirrors `null` (no declared type).
    fn defined_type(&self) -> Option<TargetType>;

    /// 更新 inner。
    /// 参数：`new_value`；返回：无。
    /// 对应或承接 Java 源文件：`com/alibaba/qlexpress4/runtime/LeftValue.java`，方法 `setInner`。
    /// Java `setInner`: assign without conversion.
    fn set_inner(&mut self, new_value: DataValue);

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
        let result = ObjTypeConvertor::cast_opt(&new_value, define_type);
        if !result.is_convertible() {
            let value_type = if new_value.is_null() {
                "null".to_string()
            } else {
                new_value.data_type_name().to_string()
            };
            let define_type_name = define_type
                .map(TargetType::java_name)
                .unwrap_or("java.lang.Object")
                .to_string();
            return Err(error_reporter.report_format(
                error_codes::INCOMPATIBLE_ASSIGNMENT_TYPE,
                error_codes::error_msg(error_codes::INCOMPATIBLE_ASSIGNMENT_TYPE),
                &[value_type, define_type_name],
            ));
        }
        self.set_inner(result.into_converted());
        Ok(())
    }
}

/// Convenience for `LeftValue` trait objects (Java uses `LeftValue` as a
/// normal interface).
impl dyn LeftValue {
    /// 处理 debug value 对应的领域职责。
    /// 无显式参数；返回：`String`。
    /// 对应或承接 Java 源文件：`com/alibaba/qlexpress4/runtime/LeftValue.java`，方法 `debugValue`；Rust 侧按所有权与 `Result` 语义适配。
    /// Helper to format this value for error messages.
    pub fn debug_value(&self) -> String {
        format!("{:?}", self.get())
    }
}
