//! Java `java.util.stream.Stream` 的顺序语义宿主适配。

use crate::exception::error_codes;
use crate::exception::ql_exception::QLExceptionKind;
use crate::exception::QLException;
use crate::runtime::java_collector::JavaCollector;
use crate::runtime::native_object::NativeObject;
use crate::runtime::value::DataValue;

/// 保存流元素并实现 QLExpress 测试所需的 `filter`/`map`/`collect`。
/// 对应 Java: java.util.stream.Stream
///
/// `parallelStream` 在 Rust 适配层保持 Java 的结果与 encounter order；
/// 实际并行调度不属于表达式可观察语义，由并发模型验收单独覆盖。
pub struct JavaStream {
    items: Vec<DataValue>,
}

impl JavaStream {
    /// 从集合元素创建流。
    /// 对应 Java：`java.util.Collection#stream()` / `parallelStream()`。
    pub fn new(items: Vec<DataValue>) -> Self {
        Self { items }
    }

    /// 转换为脚本宿主对象。
    /// 对应 Java: 无（Rust 原生适配）。
    pub fn into_data_value(self) -> DataValue {
        DataValue::Object(std::rc::Rc::new(std::cell::RefCell::new(self)))
    }
}

impl NativeObject for JavaStream {
    fn get_field(&self, _name: &str) -> Option<DataValue> {
        None
    }

    fn call_method(&mut self, name: &str, args: &[DataValue]) -> Result<DataValue, QLException> {
        match (name, args) {
            ("filter", [DataValue::Lambda(predicate)]) => {
                let mut filtered = Vec::with_capacity(self.items.len());
                for item in &self.items {
                    match predicate
                        .call(std::slice::from_ref(item))?
                        .value()
                        .as_bool()
                    {
                        Some(true) => filtered.push(item.clone()),
                        Some(false) => {}
                        None => {
                            return Err(QLException::for_test(
                                QLExceptionKind::Runtime,
                                "stream filter predicate must return boolean",
                                error_codes::INVOKE_LAMBDA_ERROR,
                            ));
                        }
                    }
                }
                Ok(JavaStream::new(filtered).into_data_value())
            }
            ("map", [DataValue::Lambda(lambda)]) => {
                let mut mapped = Vec::with_capacity(self.items.len());
                for item in &self.items {
                    mapped.push(lambda.call(std::slice::from_ref(item))?.value());
                }
                Ok(JavaStream::new(mapped).into_data_value())
            }
            ("collect", [DataValue::Object(collector)])
                if collector.borrow().as_any().is::<JavaCollector>() =>
            {
                Ok(DataValue::list(self.items.clone()))
            }
            _ => Err(QLException::for_test(
                QLExceptionKind::Runtime,
                format!("invoke method '{name}' with wrong arguments"),
                error_codes::INVOKE_METHOD_WITH_WRONG_ARGUMENTS,
            )),
        }
    }

    fn native_type_name(&self) -> &str {
        "java.util.stream.Stream"
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}
