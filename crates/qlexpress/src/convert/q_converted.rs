//! Java 风格类型转换的结果。

use crate::runtime::value::DataValue;

/// 转换是否可行以及转换后的脚本值。
///
/// 对应 Java:
/// `com.alibaba.qlexpress4.runtime.data.convert.ObjTypeConvertor.QConverted`。
#[derive(Clone, Debug, PartialEq)]
pub struct QConverted {
    pub(crate) convertible: bool,
    pub(crate) converted: DataValue,
}
