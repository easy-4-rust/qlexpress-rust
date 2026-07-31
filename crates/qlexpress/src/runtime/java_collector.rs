//! Java `java.util.stream.Collector` 的 `toList` 标记对象。

use crate::exception::QLException;
use crate::exception::error_codes;
use crate::exception::ql_exception::QLExceptionKind;
use crate::runtime::native_object::NativeObject;
use crate::runtime::value::DataValue;

/// `Collectors.toList()` 返回的收集器标记。
/// 对应 Java: java.util.stream.Collector
pub struct JavaCollector;

impl JavaCollector {
    /// 转换为脚本宿主对象。
    /// 对应 Java：`java.util.stream.Collectors#toList()` 返回的 `Collector` 对象。
    pub fn into_data_value(self) -> DataValue {
        DataValue::Object(std::rc::Rc::new(std::cell::RefCell::new(self)))
    }
}

impl NativeObject for JavaCollector {
    fn get_field(&self, _name: &str) -> Option<DataValue> {
        None
    }

    fn call_method(&mut self, name: &str, _args: &[DataValue]) -> Result<DataValue, QLException> {
        Err(QLException::for_test(
            QLExceptionKind::Runtime,
            format!("method not found: {name}"),
            error_codes::METHOD_NOT_FOUND,
        ))
    }

    fn native_type_name(&self) -> &str {
        "java.util.stream.Collector"
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}
