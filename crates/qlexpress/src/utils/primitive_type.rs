//! Java 原语及包装类型的规范类别。

/// Java 原语类型及其包装形式。
///
/// 对应 Java: `com.alibaba.qlexpress4.utils.BasicUtil.primitiveMap`。
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum PrimitiveType {
    /// boolean/Boolean。
    Boolean,
    /// char/Character。
    Character,
    /// double/Double。
    Double,
    /// float/Float。
    Float,
    /// int/Integer。
    Int,
    /// long/Long。
    Long,
    /// byte/Byte。
    Byte,
    /// short/Short。
    Short,
}
