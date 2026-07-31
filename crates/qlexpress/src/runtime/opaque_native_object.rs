//! 仅携带 Java 类型身份的宿主对象。
//!
//! Rust 无 JVM 对象模型；对无需暴露字段或方法、但必须保留运行时类型
//! 身份的 JDK 对象（例如异常、`HashSet`）使用本适配器。

use crate::exception::QLException;
use crate::exception::error_codes;
use crate::exception::ql_exception::QLExceptionKind;
use crate::runtime::native_object::NativeObject;
use crate::runtime::value::DataValue;

/// 只保存 Java 全限定类型名的原生对象。Rust 适配对象，Java 无同名类。
/// 对应 Java: 无（Rust 原生适配）。
pub struct OpaqueNativeObject {
    type_name: String,
}

impl OpaqueNativeObject {
    /// 以 Java 全限定类型名创建对象。Rust 适配入口，Java 无同名方法。
    /// 对应 Java: 无（Rust 原生适配）。
    pub fn new(type_name: impl Into<String>) -> Self {
        Self {
            type_name: type_name.into(),
        }
    }

    /// 包装成 QLExpress 栈值。Rust 适配入口，Java 无同名方法。
    /// 对应 Java: 无（Rust 原生适配）。
    pub fn into_data_value(self) -> DataValue {
        DataValue::Object(std::rc::Rc::new(std::cell::RefCell::new(self)))
    }
}

impl NativeObject for OpaqueNativeObject {
    fn get_field(&self, _name: &str) -> Option<DataValue> {
        None
    }

    fn call_method(&mut self, name: &str, _args: &[DataValue]) -> Result<DataValue, QLException> {
        Err(QLException::for_test(
            QLExceptionKind::Runtime,
            format!("method '{name}' not found on {}", self.type_name),
            error_codes::METHOD_NOT_FOUND,
        ))
    }

    fn native_type_name(&self) -> &str {
        &self.type_name
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}
