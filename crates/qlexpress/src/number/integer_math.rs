//! Integer 数值域实现(Byte/Short 提升为 int 后同域)。
//!
//! 对应 Java: com.alibaba.qlexpress4.runtime.operator.number.IntegerMath
//! (reference groovy source code)。语义要点:Java int 运算溢出**回绕**
//! (不提升),Rust 用 `wrapping_*` 复刻;除法委托 BigDecimalMath;
//! 整除/取余遇零抛 ArithmeticException("/ by zero")。

use super::big_decimal_math::BigDecimalMath;
use super::number_math;
use crate::exception::QLException;
use crate::runtime::data::convert;
use crate::runtime::value::DataValue;

/// 对应 Java: IntegerMath(单例,Rust 用零大小类型 + 关联函数)。
pub struct IntegerMath;

/// Java `Number.intValue()`(进入本域时操作数必为 Byte/Short/Int)。
fn int_value(v: &DataValue) -> i32 {
    convert::to_i64(v) as i32
}

impl IntegerMath {
    /// 返回当前数值域的绝对值。
    /// 参数：`number`；返回：`Result<DataValue, QLException>`。
    /// 对应或承接 Java 源文件：`com/alibaba/qlexpress4/runtime/operator/number/IntegerMath.java`，方法 `absImpl`；Rust 侧按所有权与 `Result` 语义适配。
    /// Java `absImpl`。
    /// 对应 Java: com.alibaba.qlexpress4.runtime.operator.number.IntegerMath#absImpl。
    pub fn abs_impl(number: &DataValue) -> Result<DataValue, QLException> {
        Ok(DataValue::Int(int_value(number).wrapping_abs()))
    }

    /// Java `addImpl`(int 溢出回绕)。
    /// 对应 Java: com.alibaba.qlexpress4.runtime.operator.number.IntegerMath#addImpl。
    pub fn add_impl(left: &DataValue, right: &DataValue) -> Result<DataValue, QLException> {
        Ok(DataValue::Int(
            int_value(left).wrapping_add(int_value(right)),
        ))
    }

    /// 在当前数值域执行减法。
    /// 参数：`left`、`right`；返回：`Result<DataValue, QLException>`。
    /// 对应或承接 Java 源文件：`com/alibaba/qlexpress4/runtime/operator/number/IntegerMath.java`，方法 `subtractImpl`；Rust 侧按所有权与 `Result` 语义适配。
    /// Java `subtractImpl`。
    /// 对应 Java: com.alibaba.qlexpress4.runtime.operator.number.IntegerMath#subtractImpl。
    pub fn subtract_impl(left: &DataValue, right: &DataValue) -> Result<DataValue, QLException> {
        Ok(DataValue::Int(
            int_value(left).wrapping_sub(int_value(right)),
        ))
    }

    /// 在当前数值域执行乘法。
    /// 参数：`left`、`right`；返回：`Result<DataValue, QLException>`。
    /// 对应或承接 Java 源文件：`com/alibaba/qlexpress4/runtime/operator/number/IntegerMath.java`，方法 `multiplyImpl`；Rust 侧按所有权与 `Result` 语义适配。
    /// Java `multiplyImpl`。
    /// 对应 Java: com.alibaba.qlexpress4.runtime.operator.number.IntegerMath#multiplyImpl。
    pub fn multiply_impl(left: &DataValue, right: &DataValue) -> Result<DataValue, QLException> {
        Ok(DataValue::Int(
            int_value(left).wrapping_mul(int_value(right)),
        ))
    }

    /// Java `divideImpl`:委托 BigDecimalMath(整型除法结果是 BigDecimal)。
    /// 对应 Java: com.alibaba.qlexpress4.runtime.operator.number.IntegerMath#divideImpl。
    pub fn divide_impl(left: &DataValue, right: &DataValue) -> Result<DataValue, QLException> {
        BigDecimalMath::divide_impl(left, right)
    }

    /// 在当前数值域比较两个操作数。
    /// 参数：`left`、`right`；返回：`i32`。
    /// 对应或承接 Java 源文件：`com/alibaba/qlexpress4/runtime/operator/number/IntegerMath.java`，方法 `compareToImpl`；Rust 侧按所有权与 `Result` 语义适配。
    /// Java `compareToImpl`。
    /// 对应 Java: com.alibaba.qlexpress4.runtime.operator.number.IntegerMath#compareToImpl。
    pub fn compare_to_impl(left: &DataValue, right: &DataValue) -> i32 {
        match int_value(left).cmp(&int_value(right)) {
            std::cmp::Ordering::Less => -1,
            std::cmp::Ordering::Equal => 0,
            std::cmp::Ordering::Greater => 1,
        }
    }

    /// 在当前整数域执行按位或。
    /// 参数：`left`、`right`；返回：`Result<DataValue, QLException>`。
    /// 对应或承接 Java 源文件：`com/alibaba/qlexpress4/runtime/operator/number/IntegerMath.java`，方法 `orImpl`；Rust 侧按所有权与 `Result` 语义适配。
    /// Java `orImpl`。
    /// 对应 Java: com.alibaba.qlexpress4.runtime.operator.number.IntegerMath#orImpl。
    pub fn or_impl(left: &DataValue, right: &DataValue) -> Result<DataValue, QLException> {
        Ok(DataValue::Int(int_value(left) | int_value(right)))
    }

    /// 在当前整数域执行按位与。
    /// 参数：`left`、`right`；返回：`Result<DataValue, QLException>`。
    /// 对应或承接 Java 源文件：`com/alibaba/qlexpress4/runtime/operator/number/IntegerMath.java`，方法 `andImpl`；Rust 侧按所有权与 `Result` 语义适配。
    /// Java `andImpl`。
    /// 对应 Java: com.alibaba.qlexpress4.runtime.operator.number.IntegerMath#andImpl。
    pub fn and_impl(left: &DataValue, right: &DataValue) -> Result<DataValue, QLException> {
        Ok(DataValue::Int(int_value(left) & int_value(right)))
    }

    /// 在当前整数域执行按位异或。
    /// 参数：`left`、`right`；返回：`Result<DataValue, QLException>`。
    /// 对应或承接 Java 源文件：`com/alibaba/qlexpress4/runtime/operator/number/IntegerMath.java`，方法 `xorImpl`；Rust 侧按所有权与 `Result` 语义适配。
    /// Java `xorImpl`。
    /// 对应 Java: com.alibaba.qlexpress4.runtime.operator.number.IntegerMath#xorImpl。
    pub fn xor_impl(left: &DataValue, right: &DataValue) -> Result<DataValue, QLException> {
        Ok(DataValue::Int(int_value(left) ^ int_value(right)))
    }

    /// Java `intDivImpl`(除零抛 ArithmeticException "/ by zero")。
    /// 对应 Java: com.alibaba.qlexpress4.runtime.operator.number.IntegerMath#intDivImpl。
    pub fn int_div_impl(left: &DataValue, right: &DataValue) -> Result<DataValue, QLException> {
        let dividend = int_value(left);
        let divisor = int_value(right);
        if divisor == 0 {
            Err(number_math::arithmetic_exception("/ by zero"))
        } else {
            // Java 对 MIN_VALUE / -1 不抛溢出异常，而是按补码回绕为 MIN_VALUE。
            Ok(DataValue::Int(dividend.wrapping_div(divisor)))
        }
    }

    /// Java `modImpl`:`toBigInteger(l).mod(toBigInteger(r)).intValue()`
    /// —— BigInteger.mod 恒非负,模数必须为正。
    /// 对应 Java: com.alibaba.qlexpress4.runtime.operator.number.IntegerMath#modImpl。
    pub fn mod_impl(left: &DataValue, right: &DataValue) -> Result<DataValue, QLException> {
        let modulus = convert::to_i128(right);
        if modulus <= 0 {
            return Err(number_math::arithmetic_exception(
                "BigInteger: modulus not positive",
            ));
        }
        let v = convert::to_i128(left).rem_euclid(modulus);
        Ok(DataValue::Int(v as i32))
    }

    /// Java `remainderImpl`(int 取余,符号跟随被除数;除零抛 "/ by zero")。
    /// 对应 Java: com.alibaba.qlexpress4.runtime.operator.number.IntegerMath#remainderImpl。
    pub fn remainder_impl(left: &DataValue, right: &DataValue) -> Result<DataValue, QLException> {
        let dividend = int_value(left);
        let divisor = int_value(right);
        if divisor == 0 {
            Err(number_math::arithmetic_exception("/ by zero"))
        } else {
            // Java 对 MIN_VALUE % -1 返回 0。
            Ok(DataValue::Int(dividend.wrapping_rem(divisor)))
        }
    }

    /// 返回当前数值取相反数后的结果。
    /// 参数：`left`；返回：`Result<DataValue, QLException>`。
    /// 对应或承接 Java 源文件：`com/alibaba/qlexpress4/runtime/operator/number/IntegerMath.java`，方法 `unaryMinusImpl`；Rust 侧按所有权与 `Result` 语义适配。
    /// Java `unaryMinusImpl`。
    /// 对应 Java: com.alibaba.qlexpress4.runtime.operator.number.IntegerMath#unaryMinusImpl。
    pub fn unary_minus_impl(left: &DataValue) -> Result<DataValue, QLException> {
        Ok(DataValue::Int(int_value(left).wrapping_neg()))
    }

    /// 返回当前数值的一元正号结果。
    /// 参数：`left`；返回：`Result<DataValue, QLException>`。
    /// 对应或承接 Java 源文件：`com/alibaba/qlexpress4/runtime/operator/number/IntegerMath.java`，方法 `unaryPlusImpl`；Rust 侧按所有权与 `Result` 语义适配。
    /// Java `unaryPlusImpl`。
    /// 对应 Java: com.alibaba.qlexpress4.runtime.operator.number.IntegerMath#unaryPlusImpl。
    pub fn unary_plus_impl(left: &DataValue) -> Result<DataValue, QLException> {
        Ok(DataValue::Int(int_value(left)))
    }

    /// 在当前整数域执行按位取反。
    /// 参数：`left`；返回：`Result<DataValue, QLException>`。
    /// 对应或承接 Java 源文件：`com/alibaba/qlexpress4/runtime/operator/number/IntegerMath.java`，方法 `bitwiseNegateImpl`；Rust 侧按所有权与 `Result` 语义适配。
    /// Java `bitwiseNegateImpl`。
    /// 对应 Java: com.alibaba.qlexpress4.runtime.operator.number.IntegerMath#bitwiseNegateImpl。
    pub fn bitwise_negate_impl(left: &DataValue) -> Result<DataValue, QLException> {
        Ok(DataValue::Int(!int_value(left)))
    }

    /// Java `leftShiftImpl`(距离按 31 掩码,`wrapping_shl` 同语义)。
    /// 对应 Java: com.alibaba.qlexpress4.runtime.operator.number.IntegerMath#leftShiftImpl。
    pub fn left_shift_impl(left: &DataValue, right: &DataValue) -> Result<DataValue, QLException> {
        Ok(DataValue::Int(
            int_value(left).wrapping_shl(int_value(right) as u32),
        ))
    }

    /// Java `rightShiftImpl`(算术右移,符号扩展)。
    /// 对应 Java: com.alibaba.qlexpress4.runtime.operator.number.IntegerMath#rightShiftImpl。
    pub fn right_shift_impl(left: &DataValue, right: &DataValue) -> Result<DataValue, QLException> {
        Ok(DataValue::Int(
            int_value(left).wrapping_shr(int_value(right) as u32),
        ))
    }

    /// Java `rightShiftUnsignedImpl`(逻辑右移,高位补零)。
    /// 对应 Java: com.alibaba.qlexpress4.runtime.operator.number.IntegerMath#rightShiftUnsignedImpl。
    pub fn right_shift_unsigned_impl(
        left: &DataValue,
        right: &DataValue,
    ) -> Result<DataValue, QLException> {
        let shift = (int_value(right) as u32) & 31;
        Ok(DataValue::Int(((int_value(left) as u32) >> shift) as i32))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wrapping_overflow_like_java_int() {
        assert_eq!(
            IntegerMath::multiply_impl(&DataValue::Int(100_000), &DataValue::Int(100_000)).unwrap(),
            DataValue::Int(100_000i32.wrapping_mul(100_000))
        );
        assert_eq!(
            IntegerMath::unary_minus_impl(&DataValue::Int(i32::MIN)).unwrap(),
            DataValue::Int(i32::MIN)
        );
    }

    #[test]
    fn remainder_by_zero_reports() {
        let err = IntegerMath::remainder_impl(&DataValue::Int(1), &DataValue::Int(0)).unwrap_err();
        assert_eq!(err.reason(), "/ by zero");
    }

    #[test]
    fn unsigned_right_shift_fills_zero() {
        // Java 语义要点:-1 >>> 1 = 2147483647(逻辑右移不符号扩展)。
        assert_eq!(
            IntegerMath::right_shift_unsigned_impl(&DataValue::Int(-1), &DataValue::Int(1))
                .unwrap(),
            DataValue::Int(0x7FFF_FFFF)
        );
        // 算术右移保持符号:-1 >> 1 = -1。
        assert_eq!(
            IntegerMath::right_shift_impl(&DataValue::Int(-1), &DataValue::Int(1)).unwrap(),
            DataValue::Int(-1)
        );
    }

    #[test]
    fn mod_is_non_negative_like_big_integer() {
        assert_eq!(
            IntegerMath::mod_impl(&DataValue::Int(-7), &DataValue::Int(3)).unwrap(),
            DataValue::Int(2)
        );
    }

    #[test]
    fn java_min_value_division_and_remainder_do_not_throw() {
        assert_eq!(
            IntegerMath::int_div_impl(&DataValue::Int(i32::MIN), &DataValue::Int(-1)).unwrap(),
            DataValue::Int(i32::MIN)
        );
        assert_eq!(
            IntegerMath::remainder_impl(&DataValue::Int(i32::MIN), &DataValue::Int(-1)).unwrap(),
            DataValue::Int(0)
        );
        assert_eq!(
            IntegerMath::int_div_impl(&DataValue::Int(-7), &DataValue::Int(3)).unwrap(),
            DataValue::Int(-2)
        );
    }
}
