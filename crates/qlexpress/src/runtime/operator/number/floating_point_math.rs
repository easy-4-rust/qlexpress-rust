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
    /// 处理 abs impl 对应的领域职责。
    /// 参数：`number`；返回：`Result<DataValue, QLException>`。
    /// 对应或承接 Java 源文件：`com/alibaba/qlexpress4/runtime/operator/number/FloatingPointMath.java`，方法 `absImpl`；Rust 侧按所有权与 `Result` 语义适配。
    /// Java `absImpl`。
    /// 对应 Java: com.alibaba.qlexpress4.runtime.operator.number.FloatingPointMath#absImpl。
    pub fn abs_impl(number: &DataValue) -> Result<DataValue, QLException> {
        Ok(DataValue::Double(double_value(number).abs()))
    }

    /// 添加或注册 impl。
    /// 参数：`left`、`right`；返回：`Result<DataValue, QLException>`。
    /// 对应或承接 Java 源文件：`com/alibaba/qlexpress4/runtime/operator/number/FloatingPointMath.java`，方法 `addImpl`；Rust 侧按所有权与 `Result` 语义适配。
    /// Java `addImpl`。
    /// 对应 Java: com.alibaba.qlexpress4.runtime.operator.number.FloatingPointMath#addImpl。
    pub fn add_impl(left: &DataValue, right: &DataValue) -> Result<DataValue, QLException> {
        Ok(DataValue::Double(double_value(left) + double_value(right)))
    }

    /// 处理 subtract impl 对应的领域职责。
    /// 参数：`left`、`right`；返回：`Result<DataValue, QLException>`。
    /// 对应或承接 Java 源文件：`com/alibaba/qlexpress4/runtime/operator/number/FloatingPointMath.java`，方法 `subtractImpl`；Rust 侧按所有权与 `Result` 语义适配。
    /// Java `subtractImpl`。
    /// 对应 Java: com.alibaba.qlexpress4.runtime.operator.number.FloatingPointMath#subtractImpl。
    pub fn subtract_impl(left: &DataValue, right: &DataValue) -> Result<DataValue, QLException> {
        Ok(DataValue::Double(double_value(left) - double_value(right)))
    }

    /// 处理 multiply impl 对应的领域职责。
    /// 参数：`left`、`right`；返回：`Result<DataValue, QLException>`。
    /// 对应或承接 Java 源文件：`com/alibaba/qlexpress4/runtime/operator/number/FloatingPointMath.java`，方法 `multiplyImpl`；Rust 侧按所有权与 `Result` 语义适配。
    /// Java `multiplyImpl`。
    /// 对应 Java: com.alibaba.qlexpress4.runtime.operator.number.FloatingPointMath#multiplyImpl。
    pub fn multiply_impl(left: &DataValue, right: &DataValue) -> Result<DataValue, QLException> {
        Ok(DataValue::Double(double_value(left) * double_value(right)))
    }

    /// Java `divideImpl`(IEEE:除零得 ±Infinity,0/0 得 NaN)。
    /// 对应 Java: com.alibaba.qlexpress4.runtime.operator.number.FloatingPointMath#divideImpl。
    pub fn divide_impl(left: &DataValue, right: &DataValue) -> Result<DataValue, QLException> {
        Ok(DataValue::Double(double_value(left) / double_value(right)))
    }

    /// Java `compareToImpl`:`Double.compare` 语义(NaN 视为最大,
    /// -0.0 < 0.0),Rust `total_cmp` 一致。
    /// 对应 Java: com.alibaba.qlexpress4.runtime.operator.number.FloatingPointMath#compareToImpl。
    pub fn compare_to_impl(left: &DataValue, right: &DataValue) -> i32 {
        match double_value(left).total_cmp(&double_value(right)) {
            std::cmp::Ordering::Less => -1,
            std::cmp::Ordering::Equal => 0,
            std::cmp::Ordering::Greater => 1,
        }
    }

    /// Java `remainderImpl`(IEEE 754 取余,`x % 0 = NaN`)。
    /// 对应 Java: com.alibaba.qlexpress4.runtime.operator.number.FloatingPointMath#remainderImpl。
    pub fn remainder_impl(left: &DataValue, right: &DataValue) -> Result<DataValue, QLException> {
        Ok(DataValue::Double(double_value(left) % double_value(right)))
    }

    /// 处理 mod impl 对应的领域职责。
    /// 参数：`left`、`right`；返回：`Result<DataValue, QLException>`。
    /// 对应或承接 Java 源文件：`com/alibaba/qlexpress4/runtime/operator/number/FloatingPointMath.java`，方法 `modImpl`；Rust 侧按所有权与 `Result` 语义适配。
    /// Java `modImpl`:`toBigInteger(l).mod(toBigInteger(r)).doubleValue()`。
    /// 对应 Java: com.alibaba.qlexpress4.runtime.operator.number.FloatingPointMath#modImpl。
    pub fn mod_impl(left: &DataValue, right: &DataValue) -> Result<DataValue, QLException> {
        let modulus = convert::to_i128(right);
        if modulus <= 0 {
            return Err(number_math::arithmetic_exception(
                "BigInteger: modulus not positive",
            ));
        }
        Ok(DataValue::Double(
            convert::to_i128(left).rem_euclid(modulus) as f64,
        ))
    }

    /// 处理 unary minus impl 对应的领域职责。
    /// 参数：`left`；返回：`Result<DataValue, QLException>`。
    /// 对应或承接 Java 源文件：`com/alibaba/qlexpress4/runtime/operator/number/FloatingPointMath.java`，方法 `unaryMinusImpl`；Rust 侧按所有权与 `Result` 语义适配。
    /// Java `unaryMinusImpl`。
    /// 对应 Java: com.alibaba.qlexpress4.runtime.operator.number.FloatingPointMath#unaryMinusImpl。
    pub fn unary_minus_impl(left: &DataValue) -> Result<DataValue, QLException> {
        Ok(DataValue::Double(-double_value(left)))
    }

    /// 处理 unary plus impl 对应的领域职责。
    /// 参数：`left`；返回：`Result<DataValue, QLException>`。
    /// 对应或承接 Java 源文件：`com/alibaba/qlexpress4/runtime/operator/number/FloatingPointMath.java`，方法 `unaryPlusImpl`；Rust 侧按所有权与 `Result` 语义适配。
    /// Java `unaryPlusImpl`。
    /// 对应 Java: com.alibaba.qlexpress4.runtime.operator.number.FloatingPointMath#unaryPlusImpl。
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
