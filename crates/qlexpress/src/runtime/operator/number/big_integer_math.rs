//! BigInteger 数值域实现。
//!
//! 对应 Java: com.alibaba.qlexpress4.runtime.operator.number.BigIntegerMath
//! (reference groovy source code)。使用 `num_bigint::BigInt` 保持 Java
//! `BigInteger` 的任意精度、补码位运算与移位语义。

use super::big_decimal_math::BigDecimalMath;
use super::number_math;
use crate::exception::QLException;
use crate::runtime::data::convert;
use crate::runtime::value::DataValue;
use num_bigint::BigInt;
use num_traits::{Signed, Zero};

/// 对应 Java: BigIntegerMath(单例,Rust 用零大小类型 + 关联函数)。
pub struct BigIntegerMath;

/// Java `NumberMath.toBigInteger(n)`(截断小数)。
fn big_value(v: &DataValue) -> BigInt {
    convert::to_big_int(v)
}

impl BigIntegerMath {
    /// 处理 abs impl 对应的领域职责。
    /// 参数：`number`；返回：`Result<DataValue, QLException>`。
    /// 对应或承接 Java 源文件：`com/alibaba/qlexpress4/runtime/operator/number/BigIntegerMath.java`，方法 `absImpl`；Rust 侧按所有权与 `Result` 语义适配。
    /// Java `absImpl`。
    /// 对应 Java: com.alibaba.qlexpress4.runtime.operator.number.BigIntegerMath#absImpl。
    pub fn abs_impl(number: &DataValue) -> Result<DataValue, QLException> {
        Ok(DataValue::BigInt(big_value(number).abs()))
    }

    /// 添加或注册 impl。
    /// 参数：`left`、`right`；返回：`Result<DataValue, QLException>`。
    /// 对应或承接 Java 源文件：`com/alibaba/qlexpress4/runtime/operator/number/BigIntegerMath.java`，方法 `addImpl`；Rust 侧按所有权与 `Result` 语义适配。
    /// Java `addImpl`。
    /// 对应 Java: com.alibaba.qlexpress4.runtime.operator.number.BigIntegerMath#addImpl。
    pub fn add_impl(left: &DataValue, right: &DataValue) -> Result<DataValue, QLException> {
        Ok(DataValue::BigInt(big_value(left) + big_value(right)))
    }

    /// 处理 subtract impl 对应的领域职责。
    /// 参数：`left`、`right`；返回：`Result<DataValue, QLException>`。
    /// 对应或承接 Java 源文件：`com/alibaba/qlexpress4/runtime/operator/number/BigIntegerMath.java`，方法 `subtractImpl`；Rust 侧按所有权与 `Result` 语义适配。
    /// Java `subtractImpl`。
    /// 对应 Java: com.alibaba.qlexpress4.runtime.operator.number.BigIntegerMath#subtractImpl。
    pub fn subtract_impl(left: &DataValue, right: &DataValue) -> Result<DataValue, QLException> {
        Ok(DataValue::BigInt(big_value(left) - big_value(right)))
    }

    /// 处理 multiply impl 对应的领域职责。
    /// 参数：`left`、`right`；返回：`Result<DataValue, QLException>`。
    /// 对应或承接 Java 源文件：`com/alibaba/qlexpress4/runtime/operator/number/BigIntegerMath.java`，方法 `multiplyImpl`；Rust 侧按所有权与 `Result` 语义适配。
    /// Java `multiplyImpl`。
    /// 对应 Java: com.alibaba.qlexpress4.runtime.operator.number.BigIntegerMath#multiplyImpl。
    pub fn multiply_impl(left: &DataValue, right: &DataValue) -> Result<DataValue, QLException> {
        Ok(DataValue::BigInt(big_value(left) * big_value(right)))
    }

    /// Java `divideImpl`:委托 BigDecimalMath。
    /// 对应 Java: com.alibaba.qlexpress4.runtime.operator.number.BigIntegerMath#divideImpl。
    pub fn divide_impl(left: &DataValue, right: &DataValue) -> Result<DataValue, QLException> {
        BigDecimalMath::divide_impl(left, right)
    }

    /// 处理 compare to impl 对应的领域职责。
    /// 参数：`left`、`right`；返回：`i32`。
    /// 对应或承接 Java 源文件：`com/alibaba/qlexpress4/runtime/operator/number/BigIntegerMath.java`，方法 `compareToImpl`；Rust 侧按所有权与 `Result` 语义适配。
    /// Java `compareToImpl`。
    /// 对应 Java: com.alibaba.qlexpress4.runtime.operator.number.BigIntegerMath#compareToImpl。
    pub fn compare_to_impl(left: &DataValue, right: &DataValue) -> i32 {
        match big_value(left).cmp(&big_value(right)) {
            std::cmp::Ordering::Less => -1,
            std::cmp::Ordering::Equal => 0,
            std::cmp::Ordering::Greater => 1,
        }
    }

    /// Java `intDivImpl`(截断向零;除零抛 "/ by zero")。
    /// 对应 Java: com.alibaba.qlexpress4.runtime.operator.number.BigIntegerMath#intDivImpl。
    pub fn int_div_impl(left: &DataValue, right: &DataValue) -> Result<DataValue, QLException> {
        let divisor = big_value(right);
        if divisor.is_zero() {
            Err(number_math::arithmetic_exception("/ by zero"))
        } else {
            Ok(DataValue::BigInt(big_value(left) / divisor))
        }
    }

    /// Java `modImpl`:BigInteger.mod 恒非负,模数非正抛
    /// ArithmeticException("BigInteger: modulus not positive")。
    /// 对应 Java: com.alibaba.qlexpress4.runtime.operator.number.BigIntegerMath#modImpl。
    pub fn mod_impl(left: &DataValue, right: &DataValue) -> Result<DataValue, QLException> {
        let modulus = big_value(right);
        if modulus <= BigInt::zero() {
            return Err(number_math::arithmetic_exception(
                "BigInteger: modulus not positive",
            ));
        }
        let remainder = big_value(left) % &modulus;
        Ok(DataValue::BigInt(if remainder.is_negative() {
            remainder + modulus
        } else {
            remainder
        }))
    }

    /// Java `remainderImpl`:BigInteger.remainder 符号跟随被除数。
    /// 对应 Java: com.alibaba.qlexpress4.runtime.operator.number.BigIntegerMath#remainderImpl。
    pub fn remainder_impl(left: &DataValue, right: &DataValue) -> Result<DataValue, QLException> {
        let divisor = big_value(right);
        if divisor.is_zero() {
            Err(number_math::arithmetic_exception("/ by zero"))
        } else {
            Ok(DataValue::BigInt(big_value(left) % divisor))
        }
    }

    /// 处理 unary minus impl 对应的领域职责。
    /// 参数：`left`；返回：`Result<DataValue, QLException>`。
    /// 对应或承接 Java 源文件：`com/alibaba/qlexpress4/runtime/operator/number/BigIntegerMath.java`，方法 `unaryMinusImpl`；Rust 侧按所有权与 `Result` 语义适配。
    /// Java `unaryMinusImpl`。
    /// 对应 Java: com.alibaba.qlexpress4.runtime.operator.number.BigIntegerMath#unaryMinusImpl。
    pub fn unary_minus_impl(left: &DataValue) -> Result<DataValue, QLException> {
        Ok(DataValue::BigInt(-big_value(left)))
    }

    /// 处理 unary plus impl 对应的领域职责。
    /// 参数：`left`；返回：`Result<DataValue, QLException>`。
    /// 对应或承接 Java 源文件：`com/alibaba/qlexpress4/runtime/operator/number/BigIntegerMath.java`，方法 `unaryPlusImpl`；Rust 侧按所有权与 `Result` 语义适配。
    /// Java `unaryPlusImpl`。
    /// 对应 Java: com.alibaba.qlexpress4.runtime.operator.number.BigIntegerMath#unaryPlusImpl。
    pub fn unary_plus_impl(left: &DataValue) -> Result<DataValue, QLException> {
        Ok(DataValue::BigInt(big_value(left)))
    }

    /// Java `bitwiseNegateImpl`(`BigInteger.not` == 按位取反 == -x-1)。
    /// 对应 Java: com.alibaba.qlexpress4.runtime.operator.number.BigIntegerMath#bitwiseNegateImpl。
    pub fn bitwise_negate_impl(left: &DataValue) -> Result<DataValue, QLException> {
        Ok(DataValue::BigInt(!big_value(left)))
    }

    /// 处理 or impl 对应的领域职责。
    /// 参数：`left`、`right`；返回：`Result<DataValue, QLException>`。
    /// 对应或承接 Java 源文件：`com/alibaba/qlexpress4/runtime/operator/number/BigIntegerMath.java`，方法 `orImpl`；Rust 侧按所有权与 `Result` 语义适配。
    /// Java `orImpl`。
    /// 对应 Java: com.alibaba.qlexpress4.runtime.operator.number.BigIntegerMath#orImpl。
    pub fn or_impl(left: &DataValue, right: &DataValue) -> Result<DataValue, QLException> {
        Ok(DataValue::BigInt(big_value(left) | big_value(right)))
    }

    /// 处理 and impl 对应的领域职责。
    /// 参数：`left`、`right`；返回：`Result<DataValue, QLException>`。
    /// 对应或承接 Java 源文件：`com/alibaba/qlexpress4/runtime/operator/number/BigIntegerMath.java`，方法 `andImpl`；Rust 侧按所有权与 `Result` 语义适配。
    /// Java `andImpl`。
    /// 对应 Java: com.alibaba.qlexpress4.runtime.operator.number.BigIntegerMath#andImpl。
    pub fn and_impl(left: &DataValue, right: &DataValue) -> Result<DataValue, QLException> {
        Ok(DataValue::BigInt(big_value(left) & big_value(right)))
    }

    /// 处理 xor impl 对应的领域职责。
    /// 参数：`left`、`right`；返回：`Result<DataValue, QLException>`。
    /// 对应或承接 Java 源文件：`com/alibaba/qlexpress4/runtime/operator/number/BigIntegerMath.java`，方法 `xorImpl`；Rust 侧按所有权与 `Result` 语义适配。
    /// Java `xorImpl`。
    /// 对应 Java: com.alibaba.qlexpress4.runtime.operator.number.BigIntegerMath#xorImpl。
    pub fn xor_impl(left: &DataValue, right: &DataValue) -> Result<DataValue, QLException> {
        Ok(DataValue::BigInt(big_value(left) ^ big_value(right)))
    }

    /// Java `leftShiftImpl`(`BigInteger.shiftLeft(int)`；负距离等价右移)。
    /// 对应 Java: com.alibaba.qlexpress4.runtime.operator.number.BigIntegerMath#leftShiftImpl。
    pub fn left_shift_impl(left: &DataValue, right: &DataValue) -> Result<DataValue, QLException> {
        let distance = convert::to_i64(right) as i32;
        if distance < 0 {
            return Self::right_shift_with(left, distance.unsigned_abs());
        }
        Ok(DataValue::BigInt(big_value(left) << distance as usize))
    }

    /// Java `rightShiftImpl`(`BigInteger.shiftRight(int)`,算术右移)。
    /// 对应 Java: com.alibaba.qlexpress4.runtime.operator.number.BigIntegerMath#rightShiftImpl。
    pub fn right_shift_impl(left: &DataValue, right: &DataValue) -> Result<DataValue, QLException> {
        let distance = convert::to_i64(right) as i32;
        if distance < 0 {
            return Self::left_shift_with(left, distance.unsigned_abs());
        }
        Self::right_shift_with(left, distance as u32)
    }

    fn left_shift_with(left: &DataValue, distance: u32) -> Result<DataValue, QLException> {
        Ok(DataValue::BigInt(big_value(left) << distance as usize))
    }

    fn right_shift_with(left: &DataValue, distance: u32) -> Result<DataValue, QLException> {
        Ok(DataValue::BigInt(big_value(left) >> distance as usize))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn big_int_beyond_i64() {
        // 超出 long 范围仍保持精确。
        let big = BigInt::from(i64::MAX) + BigInt::from(10);
        assert_eq!(
            BigIntegerMath::add_impl(&DataValue::BigInt(big.clone()), &DataValue::Long(1)).unwrap(),
            DataValue::BigInt(big + BigInt::from(1))
        );
        // mod 恒非负。
        assert_eq!(
            BigIntegerMath::mod_impl(&DataValue::big_int(-7), &DataValue::Int(3)).unwrap(),
            DataValue::big_int(2)
        );
        // remainder 符号跟随被除数。
        assert_eq!(
            BigIntegerMath::remainder_impl(&DataValue::big_int(-7), &DataValue::Int(3)).unwrap(),
            DataValue::big_int(-1)
        );
    }

    #[test]
    fn arithmetic_is_not_limited_to_i128() {
        let huge = BigInt::parse_bytes(
            b"121932631137021795226185032733622923332237463801111263526900",
            10,
        )
        .expect("任意精度整数");
        assert_eq!(
            BigIntegerMath::add_impl(
                &DataValue::BigInt(huge.clone()),
                &DataValue::BigInt(huge.clone())
            )
            .unwrap(),
            DataValue::BigInt(huge * BigInt::from(2))
        );
    }

    #[test]
    fn java_big_integer_domain_operation_matrix() {
        assert_eq!(
            BigIntegerMath::abs_impl(&DataValue::big_int(-7)).unwrap(),
            DataValue::big_int(7)
        );
        assert_eq!(
            BigIntegerMath::subtract_impl(&DataValue::big_int(7), &DataValue::Int(9)).unwrap(),
            DataValue::big_int(-2)
        );
        assert_eq!(
            BigIntegerMath::multiply_impl(&DataValue::big_int(7), &DataValue::Int(9)).unwrap(),
            DataValue::big_int(63)
        );
        assert_eq!(
            BigIntegerMath::divide_impl(&DataValue::big_int(7), &DataValue::Int(2)).unwrap(),
            DataValue::BigDec("3.5".into())
        );
        assert_eq!(
            BigIntegerMath::compare_to_impl(&DataValue::big_int(7), &DataValue::big_int(9)),
            -1
        );
        assert_eq!(
            BigIntegerMath::compare_to_impl(&DataValue::big_int(9), &DataValue::big_int(9)),
            0
        );
        assert_eq!(
            BigIntegerMath::compare_to_impl(&DataValue::big_int(10), &DataValue::big_int(9)),
            1
        );
        assert_eq!(
            BigIntegerMath::int_div_impl(&DataValue::big_int(-7), &DataValue::Int(3)).unwrap(),
            DataValue::big_int(-2)
        );
        assert_eq!(
            BigIntegerMath::unary_minus_impl(&DataValue::big_int(7)).unwrap(),
            DataValue::big_int(-7)
        );
        assert_eq!(
            BigIntegerMath::unary_plus_impl(&DataValue::big_int(-7)).unwrap(),
            DataValue::big_int(-7)
        );
        assert_eq!(
            BigIntegerMath::bitwise_negate_impl(&DataValue::big_int(0)).unwrap(),
            DataValue::big_int(-1)
        );
        assert_eq!(
            BigIntegerMath::or_impl(&DataValue::big_int(0b1010), &DataValue::big_int(0b0101))
                .unwrap(),
            DataValue::big_int(0b1111)
        );
        assert_eq!(
            BigIntegerMath::and_impl(&DataValue::big_int(0b1010), &DataValue::big_int(0b0110))
                .unwrap(),
            DataValue::big_int(0b0010)
        );
        assert_eq!(
            BigIntegerMath::xor_impl(&DataValue::big_int(0b1010), &DataValue::big_int(0b0110))
                .unwrap(),
            DataValue::big_int(0b1100)
        );
        assert_eq!(
            BigIntegerMath::left_shift_impl(&DataValue::big_int(1), &DataValue::Int(5)).unwrap(),
            DataValue::big_int(32)
        );
        assert_eq!(
            BigIntegerMath::right_shift_impl(&DataValue::big_int(-8), &DataValue::Int(2)).unwrap(),
            DataValue::big_int(-2)
        );
        // BigInteger 的负移位距离会切换方向。
        assert_eq!(
            BigIntegerMath::left_shift_impl(&DataValue::big_int(8), &DataValue::Int(-2)).unwrap(),
            DataValue::big_int(2)
        );
        assert_eq!(
            BigIntegerMath::right_shift_impl(&DataValue::big_int(2), &DataValue::Int(-3)).unwrap(),
            DataValue::big_int(16)
        );
    }

    #[test]
    fn java_big_integer_error_edges() {
        assert_eq!(
            BigIntegerMath::int_div_impl(&DataValue::big_int(1), &DataValue::Int(0))
                .unwrap_err()
                .reason(),
            "/ by zero"
        );
        assert_eq!(
            BigIntegerMath::remainder_impl(&DataValue::big_int(1), &DataValue::Int(0))
                .unwrap_err()
                .reason(),
            "/ by zero"
        );
        for modulus in [0, -1] {
            assert_eq!(
                BigIntegerMath::mod_impl(&DataValue::big_int(1), &DataValue::Int(modulus))
                    .unwrap_err()
                    .reason(),
                "BigInteger: modulus not positive"
            );
        }
    }
}
