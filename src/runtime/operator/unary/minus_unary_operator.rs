//! 一元 `-` 操作符,对应 Java
//! `com.alibaba.qlexpress4.runtime.operator.unary.MinusUnaryOperator`。

use crate::exception::error_codes;
use crate::exception::error_reporter::ErrorReporter;
use crate::exception::QLException;
use crate::ql_precedences;
use crate::runtime::data::convert::to_f64;
use crate::runtime::operator::base::UnaryOperator;
use crate::runtime::value::{DataValue, QValue};

/// 一元 `-` 操作符(数值取负)。
///
/// 对应 Java: com.alibaba.qlexpress4.runtime.operator.unary.MinusUnaryOperator
/// (@author bingo;执行体委托 `NumberMath.unaryMinus`,错误构造继承自
/// `BaseUnaryOperator`)。
#[derive(Clone, Copy, Debug, Default)]
pub struct MinusUnaryOperator;

impl MinusUnaryOperator {
    /// 对应 Java `MinusUnaryOperator.getInstance()` 单例。
    pub fn get_instance() -> MinusUnaryOperator {
        MinusUnaryOperator
    }
}

impl UnaryOperator for MinusUnaryOperator {
    /// 对应 Java `MinusUnaryOperator.execute(Value value, ErrorReporter)`:
    /// 操作数必须是 `Number`,返回 `NumberMath.unaryMinus(operand)`。
    fn execute(
        &self,
        value: &QValue,
        error_reporter: &dyn ErrorReporter,
    ) -> Result<DataValue, QLException> {
        let operand = value.get();
        // Java:!(operand instanceof Number) → buildInvalidOperandTypeException
        if !operand.is_number() {
            return Err(build_invalid_operand_type_exception(
                value,
                self.operator(),
                error_reporter,
            ));
        }
        Ok(unary_minus(&operand))
    }

    /// 对应 Java `getOperator()`:操作符词素 `"-"`。
    fn operator(&self) -> &str {
        "-"
    }

    /// 对应 Java `getPriority()`:`QLPrecedences.UNARY`。
    fn priority(&self) -> i32 {
        ql_precedences::UNARY
    }
}

/// 对应 Java `NumberMath.unaryMinus(Number)`(经 `getMath(number)` 分发):
/// - `IntegerMath.unaryMinusImpl`:`-left.intValue()` → **Byte/Short/Int
///   一律提升为 Int**;
/// - `LongMath`:`-left.longValue()`;
/// - `BigIntegerMath`:`toBigInteger(left).negate()`;
/// - `BigDecimalMath`:`toBigDecimal(left).negate()`;
/// - `FloatingPointMath`:`-left.doubleValue()` → **Float 提升为 Double**。
fn unary_minus(operand: &DataValue) -> DataValue {
    match operand {
        DataValue::Byte(v) => DataValue::Int(-(*v as i32)),
        DataValue::Short(v) => DataValue::Int(-(*v as i32)),
        DataValue::Int(v) => DataValue::Int(v.wrapping_neg()),
        DataValue::Long(v) => DataValue::Long(v.wrapping_neg()),
        DataValue::BigInt(v) => DataValue::BigInt(-*v),
        // Java BigDecimal.negate():十进制字符串翻转符号(零不添负号)
        DataValue::BigDec(v) => DataValue::BigDec(big_dec_negate(v)),
        // Java FloatingPointMath:float/double 都以 -doubleValue 返回
        DataValue::Float(_) | DataValue::Double(_) => DataValue::Double(-to_f64(operand)),
        // 调用点已保证 is_number,其余类型不可达
        _ => unreachable!("unary minus on non-number"),
    }
}

/// 十进制字符串取负,对应 Java `BigDecimal.negate()` 在字符串存储上的
/// 等价实现(零值不产生 `-0`)。
fn big_dec_negate(dec: &str) -> String {
    let (negative, body) = match dec.strip_prefix('-') {
        Some(body) => (true, body),
        None => (false, dec),
    };
    let body = body.strip_prefix('+').unwrap_or(body);
    // 零值(BigDecimal("0.00").negate() == 0.00)不添负号
    if body.bytes().all(|b| b == b'0' || b == b'.') {
        return body.to_string();
    }
    if negative {
        body.to_string()
    } else {
        format!("-{body}")
    }
}

/// 对应 Java `BaseUnaryOperator.buildInvalidOperandTypeException`:
/// 错误码 `INVALID_UNARY_OPERAND`,参数为操作符、类型名与值。
fn build_invalid_operand_type_exception(
    value: &QValue,
    operator: &str,
    error_reporter: &dyn ErrorReporter,
) -> QLException {
    error_reporter.report_format(
        error_codes::INVALID_UNARY_OPERAND,
        error_codes::error_msg(error_codes::INVALID_UNARY_OPERAND),
        &[
            operator.to_string(),
            value.type_name().to_string(),
            value.get().string_value_of(),
        ],
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::exception::pure_err_reporter::PureErrReporter;

    fn run(value: DataValue) -> Result<DataValue, QLException> {
        MinusUnaryOperator::get_instance()
            .execute(&QValue::from(value), &PureErrReporter::INSTANCE)
    }

    #[test]
    fn unary_minus_promotes_byte_short_to_int() {
        // Java IntegerMath.unaryMinusImpl:-intValue()
        assert_eq!(run(DataValue::Byte(5)).unwrap(), DataValue::Int(-5));
        assert_eq!(run(DataValue::Short(-3)).unwrap(), DataValue::Int(3));
        assert_eq!(run(DataValue::Int(7)).unwrap(), DataValue::Int(-7));
    }

    #[test]
    fn unary_minus_negates_big_decimal_and_widens_float() {
        // Java BigDecimalMath:negate()
        assert_eq!(
            run(DataValue::BigDec("1.50".into())).unwrap(),
            DataValue::BigDec("-1.50".into())
        );
        assert_eq!(
            run(DataValue::BigDec("-2".into())).unwrap(),
            DataValue::BigDec("2".into())
        );
        // Java FloatingPointMath:Float → Double
        assert_eq!(run(DataValue::Float(1.5)).unwrap(), DataValue::Double(-1.5));
        assert_eq!(run(DataValue::Long(7)).unwrap(), DataValue::Long(-7));
        assert_eq!(run(DataValue::BigInt(7)).unwrap(), DataValue::BigInt(-7));
    }

    #[test]
    fn unary_minus_rejects_non_number() {
        let err = run(DataValue::Str("a".into())).unwrap_err();
        assert_eq!(err.error_code(), error_codes::INVALID_UNARY_OPERAND);
    }
}
