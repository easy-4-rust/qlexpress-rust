//! 宿主对象 trait,对应 Java 反射访问对象的能力(SPEC §4/§6 显式注册策略;
//! Rust 新增物,承担 Java 中 `Field.get`/`Method.invoke` 于宿主对象上的职责)。

use std::cmp::Ordering;
use std::fmt;

use crate::exception::QLException;
use crate::runtime::value::DataValue;

/// 宿主(原生)对象,存储于 [`DataValue::Object`],替代 Java 反射访问
/// (SPEC §4/§6)。
///
/// Java 版对任意 `Object` 通过反射读字段、调方法;Rust 要求宿主对象实现
/// 本 trait 显式暴露:
/// - [`NativeObject::get_field`] 对应 Java `Field.get`(含 getter 方法);
/// - [`NativeObject::set_field`] 对应 Java 可写 `Field` 的赋值;
/// - [`NativeObject::call_method`] 对应 Java `Method.invoke`;
/// - [`NativeObject::native_type_name`] 对应 Java `obj.getClass().getName()`,
///   用于错误信息中的类型名。
pub trait NativeObject: std::any::Any {
    /// 对应 Java 反射字段读取(`Field.get` / getter 方法)。
    fn get_field(&self, name: &str) -> Option<DataValue>;

    /// 尝试写入宿主字段，对应 Java `FieldValue#setInner`。
    ///
    /// 返回 `true` 表示字段存在、可写且赋值成功；返回 `false` 表示字段
    /// 不存在、只读或值类型不兼容。调用方随后仍会执行字段读取，以区分
    /// “字段不存在（忽略）”与“字段存在但不可赋值（INVALID_ASSIGNMENT）”。
    ///
    /// 默认实现保持手写宿主对象向后兼容；`#[derive(QLExpressType)]`
    /// 会为支持的非只读字段生成精确写入分派。
    fn set_field(&mut self, _name: &str, _value: &DataValue) -> bool {
        false
    }

    /// 对应 Java 反射方法调用(`Method.invoke`)。
    fn call_method(&mut self, name: &str, args: &[DataValue]) -> Result<DataValue, QLException>;

    /// 原生类型名,用于错误消息,对应 Java 类名。
    fn native_type_name(&self) -> &str;

    /// 是否实现 Java `Comparable`。Rust 宿主对象默认不可比较。
    fn is_comparable(&self) -> bool {
        false
    }

    /// 对应 Java `Comparable.compareTo(Object)`；不可比较或类型不兼容时
    /// 返回 `None`。
    fn compare_to(&self, _other: &dyn NativeObject) -> Option<Ordering> {
        None
    }

    /// 向下转型支持(如 `CastInstruction`/`as_meta_class` 识别 `MetaClass`
    /// 包装,对应 Java `instanceof MetaClass`)。
    fn as_any(&self) -> &dyn std::any::Any;
}

impl fmt::Debug for dyn NativeObject {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "NativeObject({})", self.native_type_name())
    }
}
