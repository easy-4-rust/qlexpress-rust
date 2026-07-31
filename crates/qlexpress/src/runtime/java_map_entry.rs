//! Java `java.util.Map.Entry` 的最小宿主适配。

use crate::exception::QLException;
use crate::exception::error_codes;
use crate::exception::ql_exception::QLExceptionKind;
use crate::runtime::native_object::NativeObject;
use crate::runtime::value::DataValue;

/// 保存 Map 的键值对并暴露 `getKey`/`getValue`。
/// 对应 Java: java.util.Map.Entry
pub struct JavaMapEntry {
    key: DataValue,
    value: DataValue,
}

impl JavaMapEntry {
    /// 创建条目。对应 Java Map 遍历产生的 `Map.Entry`。
    pub fn new(key: DataValue, value: DataValue) -> Self {
        Self { key, value }
    }

    /// 转换为脚本宿主对象。
    pub fn into_data_value(self) -> DataValue {
        DataValue::Object(std::rc::Rc::new(std::cell::RefCell::new(self)))
    }
}

impl NativeObject for JavaMapEntry {
    fn get_field(&self, _name: &str) -> Option<DataValue> {
        None
    }

    fn call_method(&mut self, name: &str, args: &[DataValue]) -> Result<DataValue, QLException> {
        if !args.is_empty() {
            return Err(QLException::for_test(
                QLExceptionKind::Runtime,
                format!("invoke method '{name}' with wrong arguments"),
                error_codes::INVOKE_METHOD_WITH_WRONG_ARGUMENTS,
            ));
        }
        match name {
            "getKey" => Ok(self.key.clone()),
            "getValue" => Ok(self.value.clone()),
            _ => Err(QLException::for_test(
                QLExceptionKind::Runtime,
                format!("method not found: {name}"),
                error_codes::METHOD_NOT_FOUND,
            )),
        }
    }

    fn native_type_name(&self) -> &str {
        "java.util.Map$Entry"
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}
