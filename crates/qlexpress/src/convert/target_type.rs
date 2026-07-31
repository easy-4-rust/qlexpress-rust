//! Java `Class<?>` 转换目标的显式 Rust 表示。

/// `ObjTypeConvertor.cast` 支持的目标类型。
/// 对应 Java：`ObjTypeConvertor#cast(Object,Class<?>)` 的 `Class<?>` 参数。
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum TargetType {
    /// Boolean。
    Boolean,
    /// Byte。
    Byte,
    /// Short。
    Short,
    /// Integer。
    Int,
    /// Long。
    Long,
    /// Float。
    Float,
    /// Double。
    Double,
    /// BigInteger。
    BigInteger,
    /// BigDecimal。
    BigDecimal,
    /// Character。
    Character,
    /// Object，保持原值。
    Any,
}
