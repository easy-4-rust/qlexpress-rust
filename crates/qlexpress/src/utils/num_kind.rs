//! Java 数值提升矩阵使用的数值类别。

/// Java 数值包装类型对应的提升类别。
///
/// 对应 Java: `com.alibaba.qlexpress4.utils.BasicUtil` 的
/// `numberPromoteLevel` 支持集合。
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum NumKind {
    /// Byte。
    Byte,
    /// Short。
    Short,
    /// Integer。
    Int,
    /// Long。
    Long,
    /// BigInteger。
    BigInteger,
    /// Float。
    Float,
    /// Double。
    Double,
    /// BigDecimal。
    BigDecimal,
}
