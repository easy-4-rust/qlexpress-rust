//! 数值运算门面(无状态),按操作数类型分派到具体数值域。
//!
//! 对应 Java: com.alibaba.qlexpress4.runtime.operator.number.NumberMath
//! (静态门面,`getMath(left, right)` 实现 Groovy 风格的数值提升矩阵:
//!
//! ```text
//!      bD bI  D  F  L  I
//!  bD  bD bD  D  D bD bD
//!  bI  bD bI  D  D bI bI
//!  D    D  D  D  D  D  D
//!  F    D  D  D  D  D  D
//!  L   bD bI  D  D  L  L
//!  I   bD bI  D  D  L  I
//! ```
//!
//! 提升矩阵已在 Stage 0 落于 `runtime/data/convert`(math_domain /
//! number_compare / promote),本文件直接复用;四则、位运算、移位等按域
//! 分派到同级五个 *Math 文件。Byte/Short/Character 在进入本层前已被
//! 提升为 Integer(Java 注释:`any Byte, Character or Short operands
//! will have been promoted to Integer`)。

use super::big_decimal_math::BigDecimalMath;
use super::big_integer_math::BigIntegerMath;
use super::floating_point_math::FloatingPointMath;
use super::integer_math::IntegerMath;
use super::long_math::LongMath;
use crate::exception::ql_exception::{QLException, QLExceptionKind};
use crate::runtime::data::convert::{self, MathDomain};
use crate::runtime::value::DataValue;

/// 数值运算错误标记(Java 抛出 `ArithmeticException` /
/// `UnsupportedOperationException` 这类非 QLExpress 异常;Rust 用带本
/// 标记码的 QLException 承载,操作符层识别后可改报 INVALID_ARITHMETIC,
/// 对齐 Java `BaseBinaryOperator.divide` 的 catch 逻辑)。
pub(crate) const ARITHMETIC_EXCEPTION: &str = "ARITHMETIC_EXCEPTION";

/// 对应 Java: NumberMath(静态工具类,无实例状态)。
pub struct NumberMath;

impl NumberMath {
    /// 处理 abs 对应的领域职责。
    /// 参数：`number`；返回：`Result<DataValue, QLException>`。
    /// 对应或承接 Java 源文件：`com/alibaba/qlexpress4/runtime/operator/number/NumberMath.java`，方法 `abs`；Rust 侧按所有权与 `Result` 语义适配。
    /// Java `NumberMath.abs(Number)`。
    pub fn abs(number: &DataValue) -> Result<DataValue, QLException> {
        match domain_of_one(number) {
            Some(MathDomain::Integer) => IntegerMath::abs_impl(number),
            Some(MathDomain::Long) => LongMath::abs_impl(number),
            Some(MathDomain::FloatingPoint) => FloatingPointMath::abs_impl(number),
            Some(MathDomain::BigInteger) => BigIntegerMath::abs_impl(number),
            Some(MathDomain::BigDecimal) => BigDecimalMath::abs_impl(number),
            None => Err(unsupported("abs()", number)),
        }
    }

    /// 处理 add 对应的领域职责。
    /// 参数：`left`、`right`；返回：`Result<DataValue, QLException>`。
    /// 对应或承接 Java 源文件：`com/alibaba/qlexpress4/runtime/operator/number/NumberMath.java`，方法 `add`；Rust 侧按所有权与 `Result` 语义适配。
    /// Java `NumberMath.add(left, right)`。
    pub fn add(left: &DataValue, right: &DataValue) -> Result<DataValue, QLException> {
        match convert::math_domain(left, right) {
            Some(MathDomain::Integer) => IntegerMath::add_impl(left, right),
            Some(MathDomain::Long) => LongMath::add_impl(left, right),
            Some(MathDomain::FloatingPoint) => FloatingPointMath::add_impl(left, right),
            Some(MathDomain::BigInteger) => BigIntegerMath::add_impl(left, right),
            Some(MathDomain::BigDecimal) => BigDecimalMath::add_impl(left, right),
            None => Err(unsupported("add()", left)),
        }
    }

    /// 处理 subtract 对应的领域职责。
    /// 参数：`left`、`right`；返回：`Result<DataValue, QLException>`。
    /// 对应或承接 Java 源文件：`com/alibaba/qlexpress4/runtime/operator/number/NumberMath.java`，方法 `subtract`；Rust 侧按所有权与 `Result` 语义适配。
    /// Java `NumberMath.subtract(left, right)`。
    pub fn subtract(left: &DataValue, right: &DataValue) -> Result<DataValue, QLException> {
        match convert::math_domain(left, right) {
            Some(MathDomain::Integer) => IntegerMath::subtract_impl(left, right),
            Some(MathDomain::Long) => LongMath::subtract_impl(left, right),
            Some(MathDomain::FloatingPoint) => FloatingPointMath::subtract_impl(left, right),
            Some(MathDomain::BigInteger) => BigIntegerMath::subtract_impl(left, right),
            Some(MathDomain::BigDecimal) => BigDecimalMath::subtract_impl(left, right),
            None => Err(unsupported("subtract()", left)),
        }
    }

    /// 处理 multiply 对应的领域职责。
    /// 参数：`left`、`right`；返回：`Result<DataValue, QLException>`。
    /// 对应或承接 Java 源文件：`com/alibaba/qlexpress4/runtime/operator/number/NumberMath.java`，方法 `multiply`；Rust 侧按所有权与 `Result` 语义适配。
    /// Java `NumberMath.multiply(left, right)`。
    pub fn multiply(left: &DataValue, right: &DataValue) -> Result<DataValue, QLException> {
        match convert::math_domain(left, right) {
            Some(MathDomain::Integer) => IntegerMath::multiply_impl(left, right),
            Some(MathDomain::Long) => LongMath::multiply_impl(left, right),
            Some(MathDomain::FloatingPoint) => FloatingPointMath::multiply_impl(left, right),
            Some(MathDomain::BigInteger) => BigIntegerMath::multiply_impl(left, right),
            Some(MathDomain::BigDecimal) => BigDecimalMath::multiply_impl(left, right),
            None => Err(unsupported("multiply()", left)),
        }
    }

    /// Java `NumberMath.divide(left, right)`。
    ///
    /// 语义要点:除法不提升为整除 —— 整型域委托 BigDecimalMath
    /// (`IntegerMath.divideImpl` → `BigDecimalMath.INSTANCE.divideImpl`),
    /// 因此 `7 / 2 == 3.5(BigDecimal)`;浮点域按 IEEE 双精度,
    /// `1.0 / 0 == Infinity`(不抛异常);整型除以零抛 ArithmeticException。
    pub fn divide(left: &DataValue, right: &DataValue) -> Result<DataValue, QLException> {
        match convert::math_domain(left, right) {
            Some(MathDomain::FloatingPoint) => FloatingPointMath::divide_impl(left, right),
            // Java: Integer/Long/BigInteger/BigDecimal 域的 divideImpl 全部
            // 落到 BigDecimalMath。
            Some(_) => BigDecimalMath::divide_impl(left, right),
            None => Err(unsupported("divide()", left)),
        }
    }

    /// Java `NumberMath.compareTo(left, right)`,返回 -1/0/1。
    pub fn compare_to(left: &DataValue, right: &DataValue) -> Result<i32, QLException> {
        match convert::number_compare(left, right) {
            Some(ord) => Ok(match ord {
                std::cmp::Ordering::Less => -1,
                std::cmp::Ordering::Equal => 0,
                std::cmp::Ordering::Greater => 1,
            }),
            None => Err(unsupported("compareTo()", left)),
        }
    }

    /// Java `NumberMath.or(left, right)`(仅整型域)。
    pub fn or(left: &DataValue, right: &DataValue) -> Result<DataValue, QLException> {
        match convert::math_domain(left, right) {
            Some(MathDomain::Integer) => IntegerMath::or_impl(left, right),
            Some(MathDomain::Long) => LongMath::or_impl(left, right),
            Some(MathDomain::BigInteger) => BigIntegerMath::or_impl(left, right),
            // Java 基类默认实现:浮点/BigDecimal 域不支持位运算。
            _ => Err(unsupported("or()", left)),
        }
    }

    /// Java `NumberMath.and(left, right)`(仅整型域)。
    pub fn and(left: &DataValue, right: &DataValue) -> Result<DataValue, QLException> {
        match convert::math_domain(left, right) {
            Some(MathDomain::Integer) => IntegerMath::and_impl(left, right),
            Some(MathDomain::Long) => LongMath::and_impl(left, right),
            Some(MathDomain::BigInteger) => BigIntegerMath::and_impl(left, right),
            _ => Err(unsupported("and()", left)),
        }
    }

    /// Java `NumberMath.xor(left, right)`(仅整型域)。
    pub fn xor(left: &DataValue, right: &DataValue) -> Result<DataValue, QLException> {
        match convert::math_domain(left, right) {
            Some(MathDomain::Integer) => IntegerMath::xor_impl(left, right),
            Some(MathDomain::Long) => LongMath::xor_impl(left, right),
            Some(MathDomain::BigInteger) => BigIntegerMath::xor_impl(left, right),
            _ => Err(unsupported("xor()", left)),
        }
    }

    /// 处理 int div 对应的领域职责。
    /// 参数：`left`、`right`；返回：`Result<DataValue, QLException>`。
    /// 对应或承接 Java 源文件：`com/alibaba/qlexpress4/runtime/operator/number/NumberMath.java`，方法 `intDiv`；Rust 侧按所有权与 `Result` 语义适配。
    /// Java `NumberMath.intDiv(left, right)`。
    pub fn int_div(left: &DataValue, right: &DataValue) -> Result<DataValue, QLException> {
        match convert::math_domain(left, right) {
            Some(MathDomain::Integer) => IntegerMath::int_div_impl(left, right),
            Some(MathDomain::Long) => LongMath::int_div_impl(left, right),
            Some(MathDomain::BigInteger) => BigIntegerMath::int_div_impl(left, right),
            _ => Err(unsupported("intDiv()", left)),
        }
    }

    /// Java `NumberMath.mod(left, right)`(保留向后兼容;
    /// BigInteger.mod 语义:结果非负,模数必须为正)。
    pub fn mod_op(left: &DataValue, right: &DataValue) -> Result<DataValue, QLException> {
        match convert::math_domain(left, right) {
            Some(MathDomain::Integer) => IntegerMath::mod_impl(left, right),
            Some(MathDomain::Long) => LongMath::mod_impl(left, right),
            Some(MathDomain::FloatingPoint) => FloatingPointMath::mod_impl(left, right),
            Some(MathDomain::BigInteger) => BigIntegerMath::mod_impl(left, right),
            Some(MathDomain::BigDecimal) => BigDecimalMath::mod_impl(left, right),
            None => Err(unsupported("mod()", left)),
        }
    }

    /// 处理 remainder 对应的领域职责。
    /// 参数：`left`、`right`；返回：`Result<DataValue, QLException>`。
    /// 对应或承接 Java 源文件：`com/alibaba/qlexpress4/runtime/operator/number/NumberMath.java`，方法 `remainder`；Rust 侧按所有权与 `Result` 语义适配。
    /// Java `NumberMath.remainder(left, right)`。
    pub fn remainder(left: &DataValue, right: &DataValue) -> Result<DataValue, QLException> {
        match convert::math_domain(left, right) {
            Some(MathDomain::Integer) => IntegerMath::remainder_impl(left, right),
            Some(MathDomain::Long) => LongMath::remainder_impl(left, right),
            Some(MathDomain::FloatingPoint) => FloatingPointMath::remainder_impl(left, right),
            Some(MathDomain::BigInteger) => BigIntegerMath::remainder_impl(left, right),
            Some(MathDomain::BigDecimal) => BigDecimalMath::remainder_impl(left, right),
            None => Err(unsupported("remainder()", left)),
        }
    }

    /// Java `NumberMath.leftShift(left, right)`。
    ///
    /// 语义要点:移位距离(右操作数)必须是整型,否则抛
    /// UnsupportedOperationException;左操作数决定计算域且**不**从
    /// Integer 提升到 Long(与 Java 移位语义一致,距离按位宽掩码:
    /// int 掩 31,long 掩 63)。
    pub fn left_shift(left: &DataValue, right: &DataValue) -> Result<DataValue, QLException> {
        assert_integral_shift_distance(right)?;
        match domain_of_one(left) {
            Some(MathDomain::Integer) => IntegerMath::left_shift_impl(left, right),
            Some(MathDomain::Long) => LongMath::left_shift_impl(left, right),
            Some(MathDomain::BigInteger) => BigIntegerMath::left_shift_impl(left, right),
            _ => Err(unsupported("leftShift()", left)),
        }
    }

    /// Java `NumberMath.rightShift(left, right)`(算术右移,符号扩展)。
    pub fn right_shift(left: &DataValue, right: &DataValue) -> Result<DataValue, QLException> {
        assert_integral_shift_distance(right)?;
        match domain_of_one(left) {
            Some(MathDomain::Integer) => IntegerMath::right_shift_impl(left, right),
            Some(MathDomain::Long) => LongMath::right_shift_impl(left, right),
            Some(MathDomain::BigInteger) => BigIntegerMath::right_shift_impl(left, right),
            _ => Err(unsupported("rightShift()", left)),
        }
    }

    /// Java `NumberMath.rightShiftUnsigned(left, right)`(逻辑右移,补零)。
    pub fn right_shift_unsigned(
        left: &DataValue,
        right: &DataValue,
    ) -> Result<DataValue, QLException> {
        assert_integral_shift_distance(right)?;
        match domain_of_one(left) {
            Some(MathDomain::Integer) => IntegerMath::right_shift_unsigned_impl(left, right),
            Some(MathDomain::Long) => LongMath::right_shift_unsigned_impl(left, right),
            // Java BigIntegerMath 未覆写 rightShiftUnsignedImpl → 不支持。
            _ => Err(unsupported("rightShiftUnsigned()", left)),
        }
    }

    /// 处理 bitwise negate 对应的领域职责。
    /// 参数：`left`；返回：`Result<DataValue, QLException>`。
    /// 对应或承接 Java 源文件：`com/alibaba/qlexpress4/runtime/operator/number/NumberMath.java`，方法 `bitwiseNegate`；Rust 侧按所有权与 `Result` 语义适配。
    /// Java `NumberMath.bitwiseNegate(left)`。
    pub fn bitwise_negate(left: &DataValue) -> Result<DataValue, QLException> {
        match domain_of_one(left) {
            Some(MathDomain::Integer) => IntegerMath::bitwise_negate_impl(left),
            Some(MathDomain::Long) => LongMath::bitwise_negate_impl(left),
            Some(MathDomain::BigInteger) => BigIntegerMath::bitwise_negate_impl(left),
            _ => Err(unsupported("bitwiseNegate()", left)),
        }
    }

    /// 处理 unary minus 对应的领域职责。
    /// 参数：`left`；返回：`Result<DataValue, QLException>`。
    /// 对应或承接 Java 源文件：`com/alibaba/qlexpress4/runtime/operator/number/NumberMath.java`，方法 `unaryMinus`；Rust 侧按所有权与 `Result` 语义适配。
    /// Java `NumberMath.unaryMinus(left)`。
    pub fn unary_minus(left: &DataValue) -> Result<DataValue, QLException> {
        match domain_of_one(left) {
            Some(MathDomain::Integer) => IntegerMath::unary_minus_impl(left),
            Some(MathDomain::Long) => LongMath::unary_minus_impl(left),
            Some(MathDomain::FloatingPoint) => FloatingPointMath::unary_minus_impl(left),
            Some(MathDomain::BigInteger) => BigIntegerMath::unary_minus_impl(left),
            Some(MathDomain::BigDecimal) => BigDecimalMath::unary_minus_impl(left),
            None => Err(unsupported("unaryMinus()", left)),
        }
    }

    /// 处理 unary plus 对应的领域职责。
    /// 参数：`left`；返回：`Result<DataValue, QLException>`。
    /// 对应或承接 Java 源文件：`com/alibaba/qlexpress4/runtime/operator/number/NumberMath.java`，方法 `unaryPlus`；Rust 侧按所有权与 `Result` 语义适配。
    /// Java `NumberMath.unaryPlus(left)`。
    pub fn unary_plus(left: &DataValue) -> Result<DataValue, QLException> {
        match domain_of_one(left) {
            Some(MathDomain::Integer) => IntegerMath::unary_plus_impl(left),
            Some(MathDomain::Long) => LongMath::unary_plus_impl(left),
            Some(MathDomain::FloatingPoint) => FloatingPointMath::unary_plus_impl(left),
            Some(MathDomain::BigInteger) => BigIntegerMath::unary_plus_impl(left),
            Some(MathDomain::BigDecimal) => BigDecimalMath::unary_plus_impl(left),
            None => Err(unsupported("unaryPlus()", left)),
        }
    }

    /// 判断 floating point 条件。
    /// 参数：`number`；返回：`bool`。
    /// 对应或承接 Java 源文件：`com/alibaba/qlexpress4/runtime/operator/number/NumberMath.java`，方法 `isFloatingPoint`；Rust 侧按所有权与 `Result` 语义适配。
    /// Java `NumberMath.isFloatingPoint(number)`。
    pub fn is_floating_point(number: &DataValue) -> bool {
        matches!(number, DataValue::Double(_) | DataValue::Float(_))
    }

    /// 判断 integer 条件。
    /// 参数：`number`；返回：`bool`。
    /// 对应或承接 Java 源文件：`com/alibaba/qlexpress4/runtime/operator/number/NumberMath.java`，方法 `isInteger`；Rust 侧按所有权与 `Result` 语义适配。
    /// Java `NumberMath.isInteger(number)`。
    pub fn is_integer(number: &DataValue) -> bool {
        matches!(number, DataValue::Int(_))
    }

    /// 判断 short 条件。
    /// 参数：`number`；返回：`bool`。
    /// 对应或承接 Java 源文件：`com/alibaba/qlexpress4/runtime/operator/number/NumberMath.java`，方法 `isShort`；Rust 侧按所有权与 `Result` 语义适配。
    /// Java `NumberMath.isShort(number)`。
    pub fn is_short(number: &DataValue) -> bool {
        matches!(number, DataValue::Short(_))
    }

    /// 判断 byte 条件。
    /// 参数：`number`；返回：`bool`。
    /// 对应或承接 Java 源文件：`com/alibaba/qlexpress4/runtime/operator/number/NumberMath.java`，方法 `isByte`；Rust 侧按所有权与 `Result` 语义适配。
    /// Java `NumberMath.isByte(number)`。
    pub fn is_byte(number: &DataValue) -> bool {
        matches!(number, DataValue::Byte(_))
    }

    /// 判断 long 条件。
    /// 参数：`number`；返回：`bool`。
    /// 对应或承接 Java 源文件：`com/alibaba/qlexpress4/runtime/operator/number/NumberMath.java`，方法 `isLong`；Rust 侧按所有权与 `Result` 语义适配。
    /// Java `NumberMath.isLong(number)`。
    pub fn is_long(number: &DataValue) -> bool {
        matches!(number, DataValue::Long(_))
    }

    /// 判断 big decimal 条件。
    /// 参数：`number`；返回：`bool`。
    /// 对应或承接 Java 源文件：`com/alibaba/qlexpress4/runtime/operator/number/NumberMath.java`，方法 `isBigDecimal`；Rust 侧按所有权与 `Result` 语义适配。
    /// Java `NumberMath.isBigDecimal(number)`。
    pub fn is_big_decimal(number: &DataValue) -> bool {
        matches!(number, DataValue::BigDec(_))
    }

    /// 判断 big integer 条件。
    /// 参数：`number`；返回：`bool`。
    /// 对应或承接 Java 源文件：`com/alibaba/qlexpress4/runtime/operator/number/NumberMath.java`，方法 `isBigInteger`；Rust 侧按所有权与 `Result` 语义适配。
    /// Java `NumberMath.isBigInteger(number)`。
    pub fn is_big_integer(number: &DataValue) -> bool {
        matches!(number, DataValue::BigInt(_))
    }

    /// Java `NumberMath.toBigDecimal(n)`:整型精确转换;Float/Double 走
    /// `new BigDecimal(n.toString())`(十进制最短表示,非二进制精确展开)。
    pub fn to_big_decimal(number: &DataValue) -> DataValue {
        DataValue::BigDec(convert::to_big_dec_string(number))
    }

    /// Java `NumberMath.toBigInteger(n)`:截断小数部分(向零取整)。
    pub fn to_big_integer(number: &DataValue) -> DataValue {
        DataValue::BigInt(convert::to_big_int(number))
    }
}

/// Java `NumberMath.getMath(Number)` 的单操作数版本(移位/一元运算用)。
fn domain_of_one(number: &DataValue) -> Option<MathDomain> {
    match number {
        DataValue::Byte(_) | DataValue::Short(_) | DataValue::Int(_) => Some(MathDomain::Integer),
        DataValue::Long(_) => Some(MathDomain::Long),
        DataValue::Float(_) | DataValue::Double(_) => Some(MathDomain::FloatingPoint),
        DataValue::BigInt(_) => Some(MathDomain::BigInteger),
        DataValue::BigDec(_) => Some(MathDomain::BigDecimal),
        _ => None,
    }
}

/// Java 移位前的右操作数检查:`isFloatingPoint(right) || isBigDecimal(right)`
/// → UnsupportedOperationException("Shift distance must be an integral
/// type, but ... was supplied")。
fn assert_integral_shift_distance(right: &DataValue) -> Result<(), QLException> {
    if NumberMath::is_floating_point(right) || NumberMath::is_big_decimal(right) {
        return Err(QLException::for_test(
            QLExceptionKind::Runtime,
            format!(
                "Shift distance must be an integral type, but {} ({}) was supplied",
                java_value_string(right),
                right.data_type_name()
            ),
            ARITHMETIC_EXCEPTION,
        ));
    }
    Ok(())
}

/// Java `NumberMath.createUnsupportedException`:消息逐字对齐
/// ("Cannot use {operation} on this number type: {class} with value: {value}")。
pub(crate) fn unsupported(operation: &str, left: &DataValue) -> QLException {
    QLException::for_test(
        QLExceptionKind::Runtime,
        format!(
            "Cannot use {} on this number type: {} with value: {}",
            operation,
            left.data_type_name(),
            java_value_string(left)
        ),
        ARITHMETIC_EXCEPTION,
    )
}

/// Java `ArithmeticException`(如 "/ by zero"、"Division by zero")的载体。
pub(crate) fn arithmetic_exception(message: impl Into<String>) -> QLException {
    QLException::for_test(QLExceptionKind::Runtime, message, ARITHMETIC_EXCEPTION)
}

/// Java `Object.toString()` 风格的值字符串(用于异常消息与字符串拼接):
/// 整型/BigInteger/BigDecimal 输出原始数字,浮点带 `.0`,null → "null"。
pub(crate) fn java_value_string(value: &DataValue) -> String {
    match value {
        DataValue::Null => "null".to_string(),
        DataValue::Bool(v) => v.to_string(),
        DataValue::Byte(v) => v.to_string(),
        DataValue::Short(v) => v.to_string(),
        DataValue::Int(v) => v.to_string(),
        DataValue::Long(v) => v.to_string(),
        DataValue::BigInt(v) => v.to_string(),
        DataValue::BigDec(v) => v.clone(),
        DataValue::Float(v) => java_f64_to_string(*v as f64),
        DataValue::Double(v) => java_f64_to_string(*v),
        DataValue::Char(v) => v.to_string(),
        DataValue::Str(v) => v.clone(),
        other => format!("{other:?}"),
    }
}

/// Java `Double.toString` 的常用路径近似:整数值带 `.0` 后缀
/// (Rust `f64::to_string` 会输出 "1",Java 输出 "1.0")。
pub(crate) fn java_f64_to_string(v: f64) -> String {
    if v.is_finite() && v.fract() == 0.0 && v.abs() < 1e16 {
        format!("{v:.1}")
    } else {
        v.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn integer_division_yields_big_decimal() {
        // Java 语义要点:7 / 2 = 3.5(BigDecimal),而非整除 3。
        let r = NumberMath::divide(&DataValue::Int(7), &DataValue::Int(2)).unwrap();
        assert_eq!(r, DataValue::BigDec("3.5".to_string()));
    }

    #[test]
    fn double_division_by_zero_is_infinity_not_panic() {
        // Java 语义要点:浮点除零得 Infinity,不抛异常。
        let r = NumberMath::divide(&DataValue::Double(1.0), &DataValue::Double(0.0)).unwrap();
        assert_eq!(r, DataValue::Double(f64::INFINITY));
        let r = NumberMath::divide(&DataValue::Double(-1.0), &DataValue::Int(0)).unwrap();
        assert_eq!(r, DataValue::Double(f64::NEG_INFINITY));
        // 0.0 / 0.0 = NaN
        let r = NumberMath::divide(&DataValue::Double(0.0), &DataValue::Double(0.0)).unwrap();
        assert!(matches!(r, DataValue::Double(v) if v.is_nan()));
    }

    #[test]
    fn int_division_by_zero_is_arithmetic_exception() {
        let err = NumberMath::divide(&DataValue::Int(1), &DataValue::Int(0)).unwrap_err();
        assert_eq!(err.error_code(), ARITHMETIC_EXCEPTION);
        assert_eq!(err.reason(), "Division by zero");
    }

    #[test]
    fn promotion_int_plus_long_is_long() {
        // 提升矩阵:I + L → L。
        let r = NumberMath::add(&DataValue::Int(1), &DataValue::Long(2)).unwrap();
        assert_eq!(r, DataValue::Long(3));
        // I + I → I(Java int 溢出回绕)。
        let r = NumberMath::add(&DataValue::Int(i32::MAX), &DataValue::Int(1)).unwrap();
        assert_eq!(r, DataValue::Int(i32::MIN));
        // L + L → L(long 溢出回绕)。
        let r = NumberMath::add(&DataValue::Long(i64::MAX), &DataValue::Long(1)).unwrap();
        assert_eq!(r, DataValue::Long(i64::MIN));
    }

    #[test]
    fn floating_point_wins_over_big_decimal() {
        let r = NumberMath::add(&DataValue::Double(0.1), &DataValue::BigDec("0.2".into())).unwrap();
        assert!(matches!(r, DataValue::Double(v) if (v - 0.3).abs() < 1e-15));
    }

    #[test]
    fn bitwise_ops_reject_floating_point() {
        let err = NumberMath::and(&DataValue::Double(1.0), &DataValue::Double(2.0)).unwrap_err();
        assert!(err.reason().contains("Cannot use and()"));
    }

    #[test]
    fn shift_distance_must_be_integral() {
        let err = NumberMath::left_shift(&DataValue::Int(1), &DataValue::Double(1.0)).unwrap_err();
        assert!(err
            .reason()
            .contains("Shift distance must be an integral type"));
    }

    #[test]
    fn shift_distance_is_masked_like_java() {
        // Java 语义要点:int 移位距离掩 31(1 << 32 == 1 << 0)。
        let r = NumberMath::left_shift(&DataValue::Int(1), &DataValue::Int(32)).unwrap();
        assert_eq!(r, DataValue::Int(1));
        // long 掩 63。
        let r = NumberMath::left_shift(&DataValue::Long(1), &DataValue::Int(64)).unwrap();
        assert_eq!(r, DataValue::Long(1));
    }
}
