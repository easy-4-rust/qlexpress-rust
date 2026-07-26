//! BigInteger 数值域实现。
//!
//! 对应 Java: com.alibaba.qlexpress4.runtime.operator.number.BigIntegerMath
//! (reference groovy source code)。SPEC §3.1 决策:无外部依赖,用 i128
//! 近似 BigInteger —— 溢出行为以 i128 回绕代替 Java 的无限精度,属于
//! SPEC 认可的近似偏差;位运算按补码语义,i128 与 BigInteger 的非负
//! 数范围一致,负数补码在固定位宽下与 Java 结果的低 128 位一致。

use super::big_decimal_math::BigDecimalMath;
use super::number_math;
use crate::exception::QLException;
use crate::runtime::data::convert;
use crate::runtime::value::DataValue;

/// 对应 Java: BigIntegerMath(单例,Rust 用零大小类型 + 关联函数)。
pub struct BigIntegerMath;

/// Java `NumberMath.toBigInteger(n)`(截断小数)。
fn big_value(v: &DataValue) -> i128 {
    convert::to_i128(v)
}

impl BigIntegerMath {
    /// Java `absImpl`。
    pub fn abs_impl(number: &DataValue) -> Result<DataValue, QLException> {
        Ok(DataValue::BigInt(big_value(number).wrapping_abs()))
    }

    /// Java `addImpl`。
    pub fn add_impl(left: &DataValue, right: &DataValue) -> Result<DataValue, QLException> {
        Ok(DataValue::BigInt(
            big_value(left).wrapping_add(big_value(right)),
        ))
    }

    /// Java `subtractImpl`。
    pub fn subtract_impl(left: &DataValue, right: &DataValue) -> Result<DataValue, QLException> {
        Ok(DataValue::BigInt(
            big_value(left).wrapping_sub(big_value(right)),
        ))
    }

    /// Java `multiplyImpl`。
    pub fn multiply_impl(left: &DataValue, right: &DataValue) -> Result<DataValue, QLException> {
        Ok(DataValue::BigInt(
            big_value(left).wrapping_mul(big_value(right)),
        ))
    }

    /// Java `divideImpl`:委托 BigDecimalMath。
    pub fn divide_impl(left: &DataValue, right: &DataValue) -> Result<DataValue, QLException> {
        BigDecimalMath::divide_impl(left, right)
    }

    /// Java `compareToImpl`。
    pub fn compare_to_impl(left: &DataValue, right: &DataValue) -> i32 {
        match big_value(left).cmp(&big_value(right)) {
            std::cmp::Ordering::Less => -1,
            std::cmp::Ordering::Equal => 0,
            std::cmp::Ordering::Greater => 1,
        }
    }

    /// Java `intDivImpl`(截断向零;除零抛 "/ by zero")。
    pub fn int_div_impl(left: &DataValue, right: &DataValue) -> Result<DataValue, QLException> {
        match big_value(left).checked_div(big_value(right)) {
            Some(v) => Ok(DataValue::BigInt(v)),
            None => Err(number_math::arithmetic_exception("/ by zero")),
        }
    }

    /// Java `modImpl`:BigInteger.mod 恒非负,模数非正抛
    /// ArithmeticException("BigInteger: modulus not positive")。
    pub fn mod_impl(left: &DataValue, right: &DataValue) -> Result<DataValue, QLException> {
        let modulus = big_value(right);
        if modulus <= 0 {
            return Err(number_math::arithmetic_exception(
                "BigInteger: modulus not positive",
            ));
        }
        Ok(DataValue::BigInt(big_value(left).rem_euclid(modulus)))
    }

    /// Java `remainderImpl`:BigInteger.remainder 符号跟随被除数。
    pub fn remainder_impl(left: &DataValue, right: &DataValue) -> Result<DataValue, QLException> {
        match big_value(left).checked_rem(big_value(right)) {
            Some(v) => Ok(DataValue::BigInt(v)),
            None => Err(number_math::arithmetic_exception("/ by zero")),
        }
    }

    /// Java `unaryMinusImpl`。
    pub fn unary_minus_impl(left: &DataValue) -> Result<DataValue, QLException> {
        Ok(DataValue::BigInt(big_value(left).wrapping_neg()))
    }

    /// Java `unaryPlusImpl`。
    pub fn unary_plus_impl(left: &DataValue) -> Result<DataValue, QLException> {
        Ok(DataValue::BigInt(big_value(left)))
    }

    /// Java `bitwiseNegateImpl`(`BigInteger.not` == 按位取反 == -x-1)。
    pub fn bitwise_negate_impl(left: &DataValue) -> Result<DataValue, QLException> {
        Ok(DataValue::BigInt(!big_value(left)))
    }

    /// Java `orImpl`。
    pub fn or_impl(left: &DataValue, right: &DataValue) -> Result<DataValue, QLException> {
        Ok(DataValue::BigInt(big_value(left) | big_value(right)))
    }

    /// Java `andImpl`。
    pub fn and_impl(left: &DataValue, right: &DataValue) -> Result<DataValue, QLException> {
        Ok(DataValue::BigInt(big_value(left) & big_value(right)))
    }

    /// Java `xorImpl`。
    pub fn xor_impl(left: &DataValue, right: &DataValue) -> Result<DataValue, QLException> {
        Ok(DataValue::BigInt(big_value(left) ^ big_value(right)))
    }

    /// Java `leftShiftImpl`(`BigInteger.shiftLeft(int)`;负距离等价右移。
    /// i128 近似:距离按 127 掩码回绕,Java 则允许任意距离)。
    pub fn left_shift_impl(left: &DataValue, right: &DataValue) -> Result<DataValue, QLException> {
        let distance = convert::to_i64(right) as i32;
        if distance < 0 {
            return Self::right_shift_with(left, distance.unsigned_abs());
        }
        Ok(DataValue::BigInt(
            big_value(left).wrapping_shl(distance as u32),
        ))
    }

    /// Java `rightShiftImpl`(`BigInteger.shiftRight(int)`,算术右移)。
    pub fn right_shift_impl(left: &DataValue, right: &DataValue) -> Result<DataValue, QLException> {
        let distance = convert::to_i64(right) as i32;
        if distance < 0 {
            return Self::left_shift_with(left, distance.unsigned_abs());
        }
        Self::right_shift_with(left, distance as u32)
    }

    fn left_shift_with(left: &DataValue, distance: u32) -> Result<DataValue, QLException> {
        Ok(DataValue::BigInt(big_value(left).wrapping_shl(distance)))
    }

    fn right_shift_with(left: &DataValue, distance: u32) -> Result<DataValue, QLException> {
        Ok(DataValue::BigInt(big_value(left).wrapping_shr(distance)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn big_int_beyond_i64() {
        // 超出 long 范围的加法(i128 近似 BigInteger)。
        let big = (i64::MAX as i128) + 10;
        assert_eq!(
            BigIntegerMath::add_impl(&DataValue::BigInt(big), &DataValue::Long(1)).unwrap(),
            DataValue::BigInt(big + 1)
        );
        // mod 恒非负。
        assert_eq!(
            BigIntegerMath::mod_impl(&DataValue::BigInt(-7), &DataValue::Int(3)).unwrap(),
            DataValue::BigInt(2)
        );
        // remainder 符号跟随被除数。
        assert_eq!(
            BigIntegerMath::remainder_impl(&DataValue::BigInt(-7), &DataValue::Int(3)).unwrap(),
            DataValue::BigInt(-1)
        );
    }
}
