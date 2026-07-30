//! 脚本值接口。对应 Java `com.alibaba.qlexpress4.runtime.Value`。

pub use crate::runtime::data::data_value::DataValue;
pub use crate::runtime::native_object::NativeObject;
pub use crate::runtime::q_value::QValue;
use crate::runtime::class_ref::ClassRef;

/// 脚本世界中的值接口。
///
/// 对应 Java: `com.alibaba.qlexpress4.runtime.Value`。
pub trait Value {
    /// 取得内部数据。对应 Java 方法 `Value#get()`。
    fn get(&self) -> DataValue;

    /// 取得运行时 Java 类型。
    ///
    /// 对应 Java：`Value#getType()`；Java `null` 返回 `Nothing.class`，
    /// Rust 返回对应 [`ClassRef`]。宿主对象使用显式注册的类型名。
    ///
    /// # 返回值
    ///
    /// 返回当前内部值的 Java 类型引用。
    fn get_type(&self) -> ClassRef {
        crate::utils::basic_util::BasicUtil::type_of_value(&self.get())
    }

    /// 取得 Java 风格类型名。对应 Java 方法 `Value#getTypeName()`。
    fn type_name(&self) -> &'static str;
}
