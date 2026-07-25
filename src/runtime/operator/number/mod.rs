//! 数值数学域,对应 Java `com.alibaba.qlexpress4.runtime.operator.number` 包。
//! 按 SPEC §5.5 一类一文件:NumberMath 门面 + 五个数值域实现
//! (Integer/Long/FloatingPoint/BigInteger/BigDecimal),参考 Groovy 的
//! 类型提升矩阵(见 number_math.rs 注释)。

pub mod big_decimal_math;
pub mod big_integer_math;
pub mod floating_point_math;
pub mod integer_math;
pub mod long_math;
pub mod number_math;

pub use big_decimal_math::BigDecimalMath;
pub use big_integer_math::BigIntegerMath;
pub use floating_point_math::FloatingPointMath;
pub use integer_math::IntegerMath;
pub use long_math::LongMath;
pub use number_math::NumberMath;
