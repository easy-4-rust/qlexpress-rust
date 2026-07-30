//! Long 数值域实现。
//!
//! 对应 Java: com.alibaba.qlexpress4.runtime.operator.number.LongMath
//! (reference groovy source code)。语义要点同 IntegerMath:溢出回绕、
//! 除法委托 BigDecimalMath、移位距离按 63 掩码。

use super::big_decimal_math::BigDecimalMath;
use super::number_math;
use crate::exception::QLException;
use crate::runtime::data::convert;
use crate::runtime::value::DataValue;

/// 对应 Java: LongMath(单例,Rust 用零大小类型 + 关联函数)。
pub struct LongMath;

/// Java `Number.longValue()`。
fn long_value(v: &DataValue) -> i64 {
    convert::to_i64(v)
}

impl LongMath {
    /// 处理 abs impl 对应的领域职责。
    /// 参数：`number`；返回：`Result<DataValue, QLException>`。
    /// 对应或承接 Java 源文件：`com/alibaba/qlexpress4/runtime/operator/number/LongMath.java`，方法 `absImpl`；Rust 侧按所有权与 `Result` 语义适配。
    /// Java `absImpl`。
    /// 对应 Java: com.alibaba.qlexpress4.runtime.operator.number.LongMath#absImpl。
    pub fn abs_impl(number: &DataValue) -> Result<DataValue, QLException> {
        Ok(DataValue::Long(long_value(number).wrapping_abs()))
    }

    /// Java `addImpl`(long 溢出回绕)。
    /// 对应 Java: com.alibaba.qlexpress4.runtime.operator.number.LongMath#addImpl。
    pub fn add_impl(left: &DataValue, right: &DataValue) -> Result<DataValue, QLException> {
        Ok(DataValue::Long(
            long_value(left).wrapping_add(long_value(right)),
        ))
    }

    /// 处理 subtract impl 对应的领域职责。
    /// 参数：`left`、`right`；返回：`Result<DataValue, QLException>`。
    /// 对应或承接 Java 源文件：`com/alibaba/qlexpress4/runtime/operator/number/LongMath.java`，方法 `subtractImpl`；Rust 侧按所有权与 `Result` 语义适配。
    /// Java `subtractImpl`。
    /// 对应 Java: com.alibaba.qlexpress4.runtime.operator.number.LongMath#subtractImpl。
    pub fn subtract_impl(left: &DataValue, right: &DataValue) -> Result<DataValue, QLException> {
        Ok(DataValue::Long(
            long_value(left).wrapping_sub(long_value(right)),
        ))
    }

    /// 处理 multiply impl 对应的领域职责。
    /// 参数：`left`、`right`；返回：`Result<DataValue, QLException>`。
    /// 对应或承接 Java 源文件：`com/alibaba/qlexpress4/runtime/operator/number/LongMath.java`，方法 `multiplyImpl`；Rust 侧按所有权与 `Result` 语义适配。
    /// Java `multiplyImpl`。
    /// 对应 Java: com.alibaba.qlexpress4.runtime.operator.number.LongMath#multiplyImpl。
    pub fn multiply_impl(left: &DataValue, right: &DataValue) -> Result<DataValue, QLException> {
        Ok(DataValue::Long(
            long_value(left).wrapping_mul(long_value(right)),
        ))
    }

    /// Java `divideImpl`:委托 BigDecimalMath。
    /// 对应 Java: com.alibaba.qlexpress4.runtime.operator.number.LongMath#divideImpl。
    pub fn divide_impl(left: &DataValue, right: &DataValue) -> Result<DataValue, QLException> {
        BigDecimalMath::divide_impl(left, right)
    }

    /// 处理 compare to impl 对应的领域职责。
    /// 参数：`left`、`right`；返回：`i32`。
    /// 对应或承接 Java 源文件：`com/alibaba/qlexpress4/runtime/operator/number/LongMath.java`，方法 `compareToImpl`；Rust 侧按所有权与 `Result` 语义适配。
    /// Java `compareToImpl`。
    /// 对应 Java: com.alibaba.qlexpress4.runtime.operator.number.LongMath#compareToImpl。
    pub fn compare_to_impl(left: &DataValue, right: &DataValue) -> i32 {
        match long_value(left).cmp(&long_value(right)) {
            std::cmp::Ordering::Less => -1,
            std::cmp::Ordering::Equal => 0,
            std::cmp::Ordering::Greater => 1,
        }
    }

    /// Java `intDivImpl`(除零抛 ArithmeticException "/ by zero")。
    /// 对应 Java: com.alibaba.qlexpress4.runtime.operator.number.LongMath#intDivImpl。
    pub fn int_div_impl(left: &DataValue, right: &DataValue) -> Result<DataValue, QLException> {
        let dividend = long_value(left);
        let divisor = long_value(right);
        if divisor == 0 {
            Err(number_math::arithmetic_exception("/ by zero"))
        } else {
            // Java 对 MIN_VALUE / -1 不抛溢出异常，而是按补码回绕为 MIN_VALUE。
            Ok(DataValue::Long(dividend.wrapping_div(divisor)))
        }
    }

    /// 处理 remainder impl 对应的领域职责。
    /// 参数：`left`、`right`；返回：`Result<DataValue, QLException>`。
    /// 对应或承接 Java 源文件：`com/alibaba/qlexpress4/runtime/operator/number/LongMath.java`，方法 `remainderImpl`；Rust 侧按所有权与 `Result` 语义适配。
    /// Java `remainderImpl`。
    /// 对应 Java: com.alibaba.qlexpress4.runtime.operator.number.LongMath#remainderImpl。
    pub fn remainder_impl(left: &DataValue, right: &DataValue) -> Result<DataValue, QLException> {
        let dividend = long_value(left);
        let divisor = long_value(right);
        if divisor == 0 {
            Err(number_math::arithmetic_exception("/ by zero"))
        } else {
            // Java 对 MIN_VALUE % -1 返回 0。
            Ok(DataValue::Long(dividend.wrapping_rem(divisor)))
        }
    }

    /// Java `modImpl`:BigInteger.mod 恒非负。
    /// 对应 Java: com.alibaba.qlexpress4.runtime.operator.number.LongMath#modImpl。
    pub fn mod_impl(left: &DataValue, right: &DataValue) -> Result<DataValue, QLException> {
        let modulus = convert::to_i128(right);
        if modulus <= 0 {
            return Err(number_math::arithmetic_exception(
                "BigInteger: modulus not positive",
            ));
        }
        Ok(DataValue::Long(
            convert::to_i128(left).rem_euclid(modulus) as i64
        ))
    }

    /// 处理 unary minus impl 对应的领域职责。
    /// 参数：`left`；返回：`Result<DataValue, QLException>`。
    /// 对应或承接 Java 源文件：`com/alibaba/qlexpress4/runtime/operator/number/LongMath.java`，方法 `unaryMinusImpl`；Rust 侧按所有权与 `Result` 语义适配。
    /// Java `unaryMinusImpl`。
    /// 对应 Java: com.alibaba.qlexpress4.runtime.operator.number.LongMath#unaryMinusImpl。
    pub fn unary_minus_impl(left: &DataValue) -> Result<DataValue, QLException> {
        Ok(DataValue::Long(long_value(left).wrapping_neg()))
    }

    /// 处理 unary plus impl 对应的领域职责。
    /// 参数：`left`；返回：`Result<DataValue, QLException>`。
    /// 对应或承接 Java 源文件：`com/alibaba/qlexpress4/runtime/operator/number/LongMath.java`，方法 `unaryPlusImpl`；Rust 侧按所有权与 `Result` 语义适配。
    /// Java `unaryPlusImpl`。
    /// 对应 Java: com.alibaba.qlexpress4.runtime.operator.number.LongMath#unaryPlusImpl。
    pub fn unary_plus_impl(left: &DataValue) -> Result<DataValue, QLException> {
        Ok(DataValue::Long(long_value(left)))
    }

    /// 处理 bitwise negate impl 对应的领域职责。
    /// 参数：`left`；返回：`Result<DataValue, QLException>`。
    /// 对应或承接 Java 源文件：`com/alibaba/qlexpress4/runtime/operator/number/LongMath.java`，方法 `bitwiseNegateImpl`；Rust 侧按所有权与 `Result` 语义适配。
    /// Java `bitwiseNegateImpl`。
    /// 对应 Java: com.alibaba.qlexpress4.runtime.operator.number.LongMath#bitwiseNegateImpl。
    pub fn bitwise_negate_impl(left: &DataValue) -> Result<DataValue, QLException> {
        Ok(DataValue::Long(!long_value(left)))
    }

    /// 处理 or impl 对应的领域职责。
    /// 参数：`left`、`right`；返回：`Result<DataValue, QLException>`。
    /// 对应或承接 Java 源文件：`com/alibaba/qlexpress4/runtime/operator/number/LongMath.java`，方法 `orImpl`；Rust 侧按所有权与 `Result` 语义适配。
    /// Java `orImpl`。
    /// 对应 Java: com.alibaba.qlexpress4.runtime.operator.number.LongMath#orImpl。
    pub fn or_impl(left: &DataValue, right: &DataValue) -> Result<DataValue, QLException> {
        Ok(DataValue::Long(long_value(left) | long_value(right)))
    }

    /// 处理 and impl 对应的领域职责。
    /// 参数：`left`、`right`；返回：`Result<DataValue, QLException>`。
    /// 对应或承接 Java 源文件：`com/alibaba/qlexpress4/runtime/operator/number/LongMath.java`，方法 `andImpl`；Rust 侧按所有权与 `Result` 语义适配。
    /// Java `andImpl`。
    /// 对应 Java: com.alibaba.qlexpress4.runtime.operator.number.LongMath#andImpl。
    pub fn and_impl(left: &DataValue, right: &DataValue) -> Result<DataValue, QLException> {
        Ok(DataValue::Long(long_value(left) & long_value(right)))
    }

    /// 处理 xor impl 对应的领域职责。
    /// 参数：`left`、`right`；返回：`Result<DataValue, QLException>`。
    /// 对应或承接 Java 源文件：`com/alibaba/qlexpress4/runtime/operator/number/LongMath.java`，方法 `xorImpl`；Rust 侧按所有权与 `Result` 语义适配。
    /// Java `xorImpl`。
    /// 对应 Java: com.alibaba.qlexpress4.runtime.operator.number.LongMath#xorImpl。
    pub fn xor_impl(left: &DataValue, right: &DataValue) -> Result<DataValue, QLException> {
        Ok(DataValue::Long(long_value(left) ^ long_value(right)))
    }

    /// Java `leftShiftImpl`(距离按 63 掩码)。
    /// 对应 Java: com.alibaba.qlexpress4.runtime.operator.number.LongMath#leftShiftImpl。
    pub fn left_shift_impl(left: &DataValue, right: &DataValue) -> Result<DataValue, QLException> {
        Ok(DataValue::Long(
            long_value(left).wrapping_shl(long_value(right) as u32),
        ))
    }

    /// Java `rightShiftImpl`(算术右移,符号扩展)。
    /// 对应 Java: com.alibaba.qlexpress4.runtime.operator.number.LongMath#rightShiftImpl。
    pub fn right_shift_impl(left: &DataValue, right: &DataValue) -> Result<DataValue, QLException> {
        Ok(DataValue::Long(
            long_value(left).wrapping_shr(long_value(right) as u32),
        ))
    }

    /// Java `rightShiftUnsignedImpl`(逻辑右移,高位补零)。
    /// 对应 Java: com.alibaba.qlexpress4.runtime.operator.number.LongMath#rightShiftUnsignedImpl。
    pub fn right_shift_unsigned_impl(
        left: &DataValue,
        right: &DataValue,
    ) -> Result<DataValue, QLException> {
        let shift = (long_value(right) as u32) & 63;
        Ok(DataValue::Long(((long_value(left) as u64) >> shift) as i64))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn long_wrapping_and_shift_mask() {
        assert_eq!(
            LongMath::add_impl(&DataValue::Long(i64::MAX), &DataValue::Long(1)).unwrap(),
            DataValue::Long(i64::MIN)
        );
        // 距离 65 掩成 1:1L << 65 == 2。
        assert_eq!(
            LongMath::left_shift_impl(&DataValue::Long(1), &DataValue::Int(65)).unwrap(),
            DataValue::Long(2)
        );
        // -1L >>> 1 = 0x7FFF...F。
        assert_eq!(
            LongMath::right_shift_unsigned_impl(&DataValue::Long(-1), &DataValue::Int(1)).unwrap(),
            DataValue::Long(i64::MAX)
        );
    }

    #[test]
    fn long_remainder_by_zero() {
        let err = LongMath::remainder_impl(&DataValue::Long(1), &DataValue::Long(0)).unwrap_err();
        assert_eq!(err.reason(), "/ by zero");
    }

    #[test]
    fn java_long_domain_operation_matrix() {
        assert_eq!(
            LongMath::abs_impl(&DataValue::Long(-7)).unwrap(),
            DataValue::Long(7)
        );
        assert_eq!(
            LongMath::subtract_impl(&DataValue::Long(7), &DataValue::Int(9)).unwrap(),
            DataValue::Long(-2)
        );
        assert_eq!(
            LongMath::multiply_impl(&DataValue::Long(7), &DataValue::Int(9)).unwrap(),
            DataValue::Long(63)
        );
        assert_eq!(
            LongMath::divide_impl(&DataValue::Long(7), &DataValue::Int(2)).unwrap(),
            DataValue::BigDec("3.5".into())
        );
        assert_eq!(
            LongMath::compare_to_impl(&DataValue::Long(7), &DataValue::Long(9)),
            -1
        );
        assert_eq!(
            LongMath::compare_to_impl(&DataValue::Long(9), &DataValue::Long(9)),
            0
        );
        assert_eq!(
            LongMath::compare_to_impl(&DataValue::Long(10), &DataValue::Long(9)),
            1
        );
        assert_eq!(
            LongMath::int_div_impl(&DataValue::Long(-7), &DataValue::Long(3)).unwrap(),
            DataValue::Long(-2)
        );
        assert_eq!(
            LongMath::remainder_impl(&DataValue::Long(-7), &DataValue::Long(3)).unwrap(),
            DataValue::Long(-1)
        );
        assert_eq!(
            LongMath::mod_impl(&DataValue::Long(-7), &DataValue::Long(3)).unwrap(),
            DataValue::Long(2)
        );
        assert_eq!(
            LongMath::unary_minus_impl(&DataValue::Long(7)).unwrap(),
            DataValue::Long(-7)
        );
        assert_eq!(
            LongMath::unary_plus_impl(&DataValue::Long(-7)).unwrap(),
            DataValue::Long(-7)
        );
        assert_eq!(
            LongMath::bitwise_negate_impl(&DataValue::Long(0)).unwrap(),
            DataValue::Long(-1)
        );
        assert_eq!(
            LongMath::or_impl(&DataValue::Long(0b1010), &DataValue::Long(0b0101)).unwrap(),
            DataValue::Long(0b1111)
        );
        assert_eq!(
            LongMath::and_impl(&DataValue::Long(0b1010), &DataValue::Long(0b0110)).unwrap(),
            DataValue::Long(0b0010)
        );
        assert_eq!(
            LongMath::xor_impl(&DataValue::Long(0b1010), &DataValue::Long(0b0110)).unwrap(),
            DataValue::Long(0b1100)
        );
        assert_eq!(
            LongMath::right_shift_impl(&DataValue::Long(-8), &DataValue::Int(2)).unwrap(),
            DataValue::Long(-2)
        );
    }

    #[test]
    fn java_long_min_value_division_and_error_edges() {
        assert_eq!(
            LongMath::int_div_impl(&DataValue::Long(i64::MIN), &DataValue::Long(-1)).unwrap(),
            DataValue::Long(i64::MIN)
        );
        assert_eq!(
            LongMath::remainder_impl(&DataValue::Long(i64::MIN), &DataValue::Long(-1)).unwrap(),
            DataValue::Long(0)
        );
        assert_eq!(
            LongMath::int_div_impl(&DataValue::Long(1), &DataValue::Long(0))
                .unwrap_err()
                .reason(),
            "/ by zero"
        );
        assert_eq!(
            LongMath::mod_impl(&DataValue::Long(1), &DataValue::Long(0))
                .unwrap_err()
                .reason(),
            "BigInteger: modulus not positive"
        );
    }
}
