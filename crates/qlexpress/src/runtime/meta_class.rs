//! 类字面量包装,对应 Java `com.alibaba.qlexpress4.runtime.MetaClass`。

use std::cell::RefCell;
use std::rc::Rc;

use crate::exception::QLException;
use crate::exception::error_codes;
use crate::exception::ql_exception::QLExceptionKind;
use crate::runtime::class_ref::ClassRef;
use crate::runtime::native_object::NativeObject;
use crate::runtime::value::DataValue;

/// 操作数栈上的类字面量。对应 Java: com.alibaba.qlexpress4.runtime.MetaClass
/// (职责:把 `Class<?>` 作为脚本值传递,支持静态字段/静态方法访问与
/// `instanceof` 右操作数;存储于 `DataValue::Object` 内)。
///
/// `PartialEq/Eq/Hash` 全部仅按 `clz`，对应 Java `equals/hashCode`。
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct MetaClass {
    /// 被包装的类(Java `Class<?> clz`)。
    clz: ClassRef,
}

impl MetaClass {
    /// 对应 Java 构造器 `MetaClass(Class<?> clz)`。
    pub fn new(clz: ClassRef) -> Self {
        MetaClass { clz }
    }

    /// 对应 Java 方法 `getClz()`。
    pub fn clz(&self) -> &ClassRef {
        &self.clz
    }

    /// 包成栈值(Java `new DataValue(new MetaClass(clz))`)。
    /// 对应 Java：`new DataValue(new MetaClass(clz))`。
    pub fn into_data_value(self) -> DataValue {
        DataValue::Object(Rc::new(RefCell::new(self)))
    }
}

impl NativeObject for MetaClass {
    /// 对应 Java 反射字段读取:`MetaClass` 自身无实例字段暴露给脚本
    /// (静态字段由 `NativeRegistry.load_field` 的 MetaClass 分支处理)。
    fn get_field(&self, _name: &str) -> Option<DataValue> {
        None
    }

    /// 对应 Java 反射方法调用:静态方法分派在 `NativeRegistry` 完成,
    /// 直接调用 `MetaClass` 的方法恒报「方法不存在」。
    fn call_method(&mut self, name: &str, _args: &[DataValue]) -> Result<DataValue, QLException> {
        Err(QLException::for_test(
            QLExceptionKind::Runtime,
            format!("method '{name}' not found on MetaClass"),
            error_codes::METHOD_NOT_FOUND,
        ))
    }

    fn native_type_name(&self) -> &str {
        "com.alibaba.qlexpress4.runtime.MetaClass"
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

/// 从栈值中取出 [`MetaClass`] 的类引用。对应 Java 的
/// `target instanceof MetaClass` 判断 + 强转。
pub fn as_meta_class(value: &DataValue) -> Option<ClassRef> {
    if let DataValue::Object(obj) = value {
        let borrowed = obj.borrow();
        borrowed
            .as_any()
            .downcast_ref::<MetaClass>()
            .map(|meta| meta.clz.clone())
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    use super::*;

    /// `SOURCE_PARITY`：Java `MetaClass#equals/hashCode` 仅取决于 `clz`。
    #[test]
    fn equality_and_hash_code_are_class_based() {
        let first = MetaClass::new(ClassRef::from_name("java.lang.String"));
        let same = MetaClass::new(ClassRef::from_name("java.lang.String"));
        let different = MetaClass::new(ClassRef::from_name("java.lang.Integer"));
        assert_eq!(first, same);
        assert_ne!(first, different);

        let mut first_hash = DefaultHasher::new();
        first.hash(&mut first_hash);
        let mut same_hash = DefaultHasher::new();
        same.hash(&mut same_hash);
        assert_eq!(first_hash.finish(), same_hash.finish());
    }
}
