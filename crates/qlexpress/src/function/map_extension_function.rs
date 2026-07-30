//! List.map 扩展函数,对应 Java `com.alibaba.qlexpress4.runtime.function.MapExtensionFunction`。

use crate::exception::error_codes;
use crate::exception::ql_exception::QLExceptionKind;
use crate::exception::QLException;
use crate::runtime::class_ref::ClassRef;
use crate::runtime::function::extension_function::ExtensionFunction;
use crate::runtime::value::DataValue;

/// `map` 扩展函数。对应 Java: com.alibaba.qlexpress4.runtime.function.MapExtensionFunction
/// (职责:为 `List` 扩展 `map(Function)` 实例方法;
/// Java 为私有构造的单例 `INSTANCE`)。
///
/// Java 语义要点:`invoke` 中 `obj instanceof List` 不成立时返回 `null`;
/// 成立时 `list.stream().map(function).collect(toList())`。
/// Rust 中 `Function` 实参即脚本 Lambda(`DataValue::Lambda`)。
pub struct MapExtensionFunction;

impl MapExtensionFunction {
    /// 对应 Java 静态字段 `INSTANCE`(私有构造的单例)。
    pub fn instance() -> Self {
        MapExtensionFunction
    }
}

impl ExtensionFunction for MapExtensionFunction {
    /// 对应 Java `getParameterTypes()`:`new Class[] {Function.class}`。
    fn parameter_types(&self) -> Vec<ClassRef> {
        vec![ClassRef::Named("java.util.function.Function".to_string())]
    }

    /// 对应 Java `getName()`:`"map"`。
    fn name(&self) -> &str {
        "map"
    }

    /// 对应 Java `getDeclaringClass()`:`List.class`。
    fn declaring_class(&self) -> ClassRef {
        ClassRef::Named("java.util.List".to_string())
    }

    /// 对应 Java `invoke(Object obj, Object[] args)`:
    /// 非 List 返回 null;否则按 Function 逐项映射。
    fn invoke(&self, obj: &DataValue, args: &[DataValue]) -> Result<DataValue, QLException> {
        // Java: if (!(obj instanceof List)) return null;
        let DataValue::List(list) = obj else {
            return Ok(DataValue::Null);
        };
        let Some(DataValue::Lambda(function)) = args.first() else {
            return Err(QLException::for_test(
                QLExceptionKind::Runtime,
                "map expects a lambda (java.util.function.Function) argument",
                error_codes::INVOKE_METHOD_WITH_WRONG_ARGUMENTS,
            ));
        };
        let mut mapped = Vec::with_capacity(list.borrow().len());
        for item in list.borrow().iter() {
            // Java: function.apply(item)。
            mapped.push(function.call(std::slice::from_ref(item))?.value());
        }
        Ok(DataValue::list(mapped))
    }
}
