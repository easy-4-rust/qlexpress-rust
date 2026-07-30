//! List.filter 扩展函数,对应 Java `com.alibaba.qlexpress4.runtime.function.FilterExtensionFunction`。

use crate::exception::error_codes;
use crate::exception::ql_exception::QLExceptionKind;
use crate::exception::QLException;
use crate::runtime::class_ref::ClassRef;
use crate::runtime::function::extension_function::ExtensionFunction;
use crate::runtime::value::DataValue;

/// `filter` 扩展函数。对应 Java: com.alibaba.qlexpress4.runtime.function.FilterExtensionFunction
/// (职责:为 `List` 扩展 `filter(Predicate)` 实例方法;
/// Java 为私有构造的单例 `INSTANCE`)。
///
/// Java 语义要点:`invoke` 中 `obj instanceof List` 不成立时返回 `null`;
/// 成立时 `list.stream().filter(predicate).collect(toList())`。
/// Rust 中 `Predicate` 实参即脚本 Lambda(`DataValue::Lambda`),
/// 逐项调用并以其布尔结果决定是否保留。
pub struct FilterExtensionFunction;

impl FilterExtensionFunction {
    /// 对应 Java 静态字段 `INSTANCE`(私有构造的单例)。
    pub fn instance() -> Self {
        FilterExtensionFunction
    }
}

impl ExtensionFunction for FilterExtensionFunction {
    /// 对应 Java `getParameterTypes()`:`new Class[] {Predicate.class}`。
    fn parameter_types(&self) -> Vec<ClassRef> {
        vec![ClassRef::Named("java.util.function.Predicate".to_string())]
    }

    /// 对应 Java `getName()`:`"filter"`。
    fn name(&self) -> &str {
        "filter"
    }

    /// 对应 Java `getDeclaringClass()`:`List.class`。
    fn declaring_class(&self) -> ClassRef {
        ClassRef::Named("java.util.List".to_string())
    }

    /// 对应 Java `invoke(Object obj, Object[] args)`:
    /// 非 List 返回 null;否则按 Predicate 过滤。
    fn invoke(&self, obj: &DataValue, args: &[DataValue]) -> Result<DataValue, QLException> {
        // Java: if (!(obj instanceof List)) return null;
        let DataValue::List(list) = obj else {
            return Ok(DataValue::Null);
        };
        let Some(DataValue::Lambda(predicate)) = args.first() else {
            // Java 侧反射会先做参数转换;此处对应「参数类型不符」的运行期失败。
            return Err(QLException::for_test(
                QLExceptionKind::Runtime,
                "filter expects a lambda (java.util.function.Predicate) argument",
                error_codes::INVOKE_METHOD_WITH_WRONG_ARGUMENTS,
            ));
        };
        let mut filtered = Vec::new();
        for item in list.borrow().iter() {
            // Java: predicate.test(item),结果按 Boolean 拆箱。
            let kept = predicate.call(std::slice::from_ref(item))?.value();
            match kept.as_bool() {
                Some(true) => filtered.push(item.clone()),
                Some(false) => {}
                // Java:非 Boolean 结果会 ClassCastException。
                None => {
                    return Err(QLException::for_test(
                        QLExceptionKind::Runtime,
                        "filter predicate must return boolean",
                        error_codes::INVOKE_LAMBDA_ERROR,
                    ));
                }
            }
        }
        Ok(DataValue::list(filtered))
    }
}
