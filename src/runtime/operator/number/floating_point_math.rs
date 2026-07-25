//! FloatingPoint(Double/Float)数值域实现。
//!
//! 对应 Java: com.alibaba.qlexpress4.runtime.operator.number.FloatingPointMath
//! (reference groovy source code)。语义要点:所有运算按
//! `Number.doubleValue()` 的 IEEE 双精度进行,除零/取余零得
//! Infinity/NaN 而非异常;位运算与移位不覆写(Java 基类抛
//! UnsupportedOperationException)。

use super::number_math;
use crate::exception::QLException;
use crate::runtime::data::convert;
use crate::runtime::value::DataValue;

/// 对应 Java: FloatingPointMath(单例,Rust 用零大小类型 + 关联函数)。
pub struct FloatingPointMath;

/// Java `Number.doubleValue()`。
fn double_value(v: &DataValue) -> f64 {
    convert::to_f64(v)
}

impl FloatingPointMath {
    /// Java `absImpl`。
    pub fn abs_impl(number: &DataValue) -> Result<DataValue, QLException> {
        Ok(DataValue::Double(double_value(number).abs()))
    }

    /// Java `addImpl`。
    pub fn add_impl(left: &DataValue, right: &DataValue) -> Result<DataValue, QLException> {
        Ok(DataValue::Double(double_value(left) + double_value(right)))
    }

    /// Java `subtractImpl`。
    pub fn subtract_impl(left: &DataValue, right: &DataValue) -> Result<DataValue, QLException> {
        Ok(DataValue::Double(double_value(left) - double_value(right)))
    }

    /// Java `multiplyImpl`。
    pub fn multiply_impl(left: &DataValue, right: &DataValue) -> Result<DataValue, QLException> {
        Ok(DataValue::Double(double_value(left) * double_value(right)))
    }

    /// Java `divideImpl`(IEEE:除零得 ±Infinity,0/0 得 NaN)。
    pub fn divide_impl(left: &DataValue, right: &DataValue) -> Result<DataValue, QLException> {
        Ok(DataValue::Double(double_value(left) / double_value(right)))
    }

    /// Java `compareToImpl`:`Double.compare` 语义(NaN 视为最大,
    /// -0.0 < 0.0),Rust `total_cmp` 一致。
    pub fn compare_to_impl(left: &DataValue, right: &DataValue) -> i32 {
        match double_value(left).total_cmp(&double_value(right)) {
            std::cmp::Ordering::Less => -1,
            std::cmp::Ordering::Equal => 0,
            std::cmp::Ordering::Greater => 1,
        }
    }

    /// Java `remainderImpl`(IEEE 754 取余,`x % 0 = NaN`)。
    pub fn remainder_impl(left: &DataValue, right: &DataValue) -> Result<DataValue, QLException> {
        Ok(DataValue::Double(double_value(left) % double_value(right)))
    }

    /// Java `modImpl`:`toBigInteger(l).mod(toBigInteger(r)).doubleValue()`。
    pub fn mod_impl(left: &DataValue, right: &DataValue) -> Result<DataValue, QLException> {
        let modulus = convert::to_i128(right);
        if modulus <= 0 {
            return Err(number_math::arithmetic_exception("BigInteger: modulus not positive"));
        }
        Ok(DataValue::Double(
            convert::to_i128(left).rem_euclid(modulus) as f64
        ))
    }

    /// Java `unaryMinusImpl`。
    pub fn unary_minus_impl(left: &DataValue) -> Result<DataValue, QLException> {
        Ok(DataValue::Double(-double_value(left)))
    }

    /// Java `unaryPlusImpl`。
    pub fn unary_plus_impl(left: &DataValue) -> Result<DataValue, QLException> {
        Ok(DataValue::Double(double_value(left)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ieee_semantics() {
        assert_eq!(
            FloatingPointMath::divide_impl(&DataValue::Int(1), &DataValue::Double(0.0)).unwrap(),
            DataValue::Double(f64::INFINITY)
        );
        assert!(matches!(
            FloatingPointMath::remainder_impl(&DataValue::Double(1.0), &DataValue::Double(0.0))
                .unwrap(),
            DataValue::Double(v) if v.is_nan()
        ));
        assert_eq!(
            FloatingPointMath::remainder_impl(&DataValue::Double(5.5), &DataValue::Double(2.0))
                .unwrap(),
            DataValue::Double(1.5)
        );
    }
}
