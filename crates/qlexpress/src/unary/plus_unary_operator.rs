//! 一元 `+` 操作符,对应 Java
//! `com.alibaba.qlexpress4.runtime.operator.unary.PlusUnaryOperator`。

use crate::exception::error_codes;
use crate::exception::error_reporter::ErrorReporter;
use crate::exception::QLException;
use crate::ql_precedences;
use crate::runtime::data::convert::to_f64;
use crate::runtime::operator::base::UnaryOperator;
use crate::runtime::value::{DataValue, QValue};

/// 一元 `+` 操作符(数值提升)。
///
/// 对应 Java: com.alibaba.qlexpress4.runtime.operator.unary.PlusUnaryOperator
/// (@author bingo;执行体委托 `NumberMath.unaryPlus`,错误构造继承自
/// `BaseUnaryOperator`)。
#[derive(Clone, Copy, Debug, Default)]
pub struct PlusUnaryOperator;

impl PlusUnaryOperator {
    /// 对应 Java `PlusUnaryOperator.getInstance()` 单例。
    pub fn get_instance() -> PlusUnaryOperator {
        PlusUnaryOperator
    }
}

impl UnaryOperator for PlusUnaryOperator {
    /// 对应 Java `PlusUnaryOperator.execute(Value value, ErrorReporter)`:
    /// 操作数必须是 `Number`,返回 `NumberMath.unaryPlus(operand)`。
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
        Ok(unary_plus(&operand))
    }

    /// 对应 Java `getOperator()`:操作符词素 `"+"`。
    fn operator(&self) -> &str {
        "+"
    }

    /// 对应 Java `getPriority()`:`QLPrecedences.UNARY`。
    fn priority(&self) -> i32 {
        ql_precedences::UNARY
    }
}

/// 对应 Java `NumberMath.unaryPlus(Number)`(经 `getMath(number)` 分发):
/// - `IntegerMath.unaryPlusImpl`:`left.intValue()` → **Byte/Short/Int 一律
///   提升为 Int**;
/// - `LongMath`:Long 保持 Long;
/// - `BigIntegerMath`:BigInteger 保持;
/// - `BigDecimalMath`:BigDecimal 保持;
/// - `FloatingPointMath`:`left.doubleValue()` → **Float 提升为 Double**。
fn unary_plus(operand: &DataValue) -> DataValue {
    match operand {
        DataValue::Byte(v) => DataValue::Int(*v as i32),
        DataValue::Short(v) => DataValue::Int(*v as i32),
        DataValue::Int(v) => DataValue::Int(*v),
        DataValue::Long(v) => DataValue::Long(*v),
        DataValue::BigInt(v) => DataValue::BigInt(v.clone()),
        DataValue::BigDec(v) => DataValue::BigDec(v.clone()),
        // Java FloatingPointMath:float/double 都以 doubleValue 返回
        DataValue::Float(_) | DataValue::Double(_) => DataValue::Double(to_f64(operand)),
        // 调用点已保证 is_number,其余类型不可达
        _ => unreachable!("unary plus on non-number"),
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
        PlusUnaryOperator::get_instance().execute(&QValue::from(value), &PureErrReporter::INSTANCE)
    }

    #[test]
    fn unary_plus_promotes_byte_short_to_int() {
        // Java IntegerMath.unaryPlusImpl:intValue()
        assert_eq!(run(DataValue::Byte(5)).unwrap(), DataValue::Int(5));
        assert_eq!(run(DataValue::Short(-3)).unwrap(), DataValue::Int(-3));
        assert_eq!(run(DataValue::Int(7)).unwrap(), DataValue::Int(7));
    }

    #[test]
    fn unary_plus_keeps_long_big_types_and_widens_float() {
        assert_eq!(run(DataValue::Long(7)).unwrap(), DataValue::Long(7));
        assert_eq!(run(DataValue::big_int(7)).unwrap(), DataValue::big_int(7));
        assert_eq!(
            run(DataValue::BigDec("1.50".into())).unwrap(),
            DataValue::BigDec("1.50".into())
        );
        // Java FloatingPointMath:Float → Double
        assert_eq!(run(DataValue::Float(1.5)).unwrap(), DataValue::Double(1.5));
        assert_eq!(run(DataValue::Double(2.5)).unwrap(), DataValue::Double(2.5));
    }

    #[test]
    fn unary_plus_rejects_non_number() {
        let err = run(DataValue::Str("a".into())).unwrap_err();
        assert_eq!(err.error_code(), error_codes::INVALID_UNARY_OPERAND);
        let err = run(DataValue::Bool(true)).unwrap_err();
        assert_eq!(err.error_code(), error_codes::INVALID_UNARY_OPERAND);
    }
}
