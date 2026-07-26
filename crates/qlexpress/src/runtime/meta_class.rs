//! 类字面量包装,对应 Java `com.alibaba.qlexpress4.runtime.MetaClass`。

use std::cell::RefCell;
use std::rc::Rc;

use crate::exception::error_codes;
use crate::exception::ql_exception::QLExceptionKind;
use crate::exception::QLException;
use crate::runtime::class_ref::ClassRef;
use crate::runtime::native_object::NativeObject;
use crate::runtime::value::DataValue;

/// 操作数栈上的类字面量。对应 Java: com.alibaba.qlexpress4.runtime.MetaClass
/// (职责:把 `Class<?>` 作为脚本值传递,支持静态字段/静态方法访问与
/// `instanceof` 右操作数;存储于 `DataValue::Object` 内)。
///
/// Java 版还实现了 `equals`/`hashCode`(按 `clz` 比较);Rust 侧脚本相等
/// 语义由操作符层处理,这里通过 [`as_meta_class`] 取回 `ClassRef` 后比较。
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
