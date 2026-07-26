//! 脚本值接口。对应 Java `com.alibaba.qlexpress4.runtime.Value`。

pub use crate::runtime::data::data_value::DataValue;
pub use crate::runtime::native_object::NativeObject;
pub use crate::runtime::q_value::QValue;

/// 脚本世界中的值接口。
///
/// 对应 Java: `com.alibaba.qlexpress4.runtime.Value`。
pub trait Value {
    /// 取得内部数据。对应 Java 方法 `Value#get()`。
    fn get(&self) -> DataValue;

    /// 取得 Java 风格类型名。对应 Java 方法 `Value#getTypeName()`。
    fn type_name(&self) -> &'static str;
}
