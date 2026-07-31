//! 可由派生宏生成的宿主 NativeType 契约。

use std::any::TypeId;
use std::rc::Rc;

use crate::runtime::native_object::NativeObject;
use crate::runtime::native_type::NativeType;

/// 声明宿主 Rust 类型如何构建可供脚本访问的 `NativeType`。
///
/// Rust 原生扩展，承接 Java 运行时反射注册职责。
/// 对应 Java：无（Rust 显式注册替代 `Class` 运行时反射）。
pub trait QLExpressNativeType: NativeObject + 'static {
    /// Java 风格规范类型名。
    const QL_TYPE_NAME: &'static str;

    /// 返回实现类型的 Rust `TypeId`，用于宿主对象向下转型。
    fn ql_type_id(&self) -> TypeId {
        TypeId::of::<Self>()
    }

    /// 构建注册到 [`crate::runtime::native_registry::NativeRegistry`] 的类型描述。
    fn build_native_type() -> NativeType;

    /// 把宿主对象包装为引擎持有的 `DataValue::Object`。
    fn into_data_value(self) -> crate::runtime::value::DataValue
    where
        Self: Sized,
    {
        use std::cell::RefCell;
        let cell: Rc<RefCell<dyn NativeObject>> = Rc::new(RefCell::new(self));
        crate::runtime::value::DataValue::Object(cell)
    }
}
