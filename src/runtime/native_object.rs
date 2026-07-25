//! 宿主对象 trait,对应 Java 反射访问对象的能力(SPEC §4/§6 显式注册策略;
//! Rust 新增物,承担 Java 中 `Field.get`/`Method.invoke` 于宿主对象上的职责)。

use std::fmt;

use crate::exception::QLException;
use crate::runtime::value::DataValue;

/// 宿主(原生)对象,存储于 [`DataValue::Object`],替代 Java 反射访问
/// (SPEC §4/§6)。
///
/// Java 版对任意 `Object` 通过反射读字段、调方法;Rust 要求宿主对象实现
/// 本 trait 显式暴露:
/// - [`NativeObject::get_field`] 对应 Java `Field.get`(含 getter 方法);
/// - [`NativeObject::call_method`] 对应 Java `Method.invoke`;
/// - [`NativeObject::native_type_name`] 对应 Java `obj.getClass().getName()`,
///   用于错误信息中的类型名。
pub trait NativeObject: std::any::Any {
    /// 对应 Java 反射字段读取(`Field.get` / getter 方法)。
    fn get_field(&self, name: &str) -> Option<DataValue>;

    /// 对应 Java 反射方法调用(`Method.invoke`)。
    fn call_method(&mut self, name: &str, args: &[DataValue]) -> Result<DataValue, QLException>;

    /// 原生类型名,用于错误消息,对应 Java 类名。
    fn native_type_name(&self) -> &str;

    /// 向下转型支持(如 `CastInstruction`/`as_meta_class` 识别 `MetaClass`
    /// 包装,对应 Java `instanceof MetaClass`)。
    fn as_any(&self) -> &dyn std::any::Any;
}

impl fmt::Debug for dyn NativeObject {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "NativeObject({})", self.native_type_name())
    }
}
