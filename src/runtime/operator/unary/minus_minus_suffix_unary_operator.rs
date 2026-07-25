//! 后缀 `--` 操作符,对应 Java
//! `com.alibaba.qlexpress4.runtime.operator.unary.MinusMinusSuffixUnaryOperator`。

use crate::exception::error_codes;
use crate::exception::error_reporter::ErrorReporter;
use crate::exception::QLException;
use crate::runtime::left_value::LeftValue;
use crate::ql_precedences;
use crate::runtime::operator::base::UnaryOperator;
use crate::runtime::value::{DataValue, QValue};

/// 后缀 `--` 操作符。
///
/// 对应 Java: com.alibaba.qlexpress4.runtime.operator.unary.MinusMinusSuffixUnaryOperator
/// (@author bingo;执行体委托 `NumberMath.subtract(operand, 1)`,错误构造
/// 继承自 `BaseUnaryOperator`)。
///
/// 注意:Java 原文的返回值是**自减后的新值**(`return result;`,与前缀
/// `--` 的写法互换,疑似 Java 版笔误)。按「以 Java 源码为唯一语义参照」
/// 原样保留该行为。
#[derive(Clone, Copy, Debug, Default)]
pub struct MinusMinusSuffixUnaryOperator;

impl MinusMinusSuffixUnaryOperator {
    /// 对应 Java `MinusMinusSuffixUnaryOperator.getInstance()` 单例。
    pub fn get_instance() -> MinusMinusSuffixUnaryOperator {
        MinusMinusSuffixUnaryOperator
    }
}

impl UnaryOperator for MinusMinusSuffixUnaryOperator {
    /// 对应 Java `MinusMinusSuffixUnaryOperator.execute(Value value,
    /// ErrorReporter)`:
    /// ```java
    /// Object operand = value.get();
    /// if (!(operand instanceof Number)) throw ...;
    /// Number result = NumberMath.subtract((Number)operand, 1);
    /// if (value instanceof LeftValue) ((LeftValue)value).set(result, errorReporter);
    /// return result;
    /// ```
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

        // Java:NumberMath.subtract(operand, 1)(数值减一逻辑与前缀 `--`
        // 共享,Java 均委托 NumberMath.subtract)
        let result = super::minus_minus_prefix_unary_operator::number_sub_one(&operand);
        // Java:value instanceof LeftValue → set(result, errorReporter)
        if let Some(left_value) = value.as_left() {
            left_value.borrow_mut().set(result.clone(), error_reporter)?;
        }
        // Java 原文:return result(自减后的新值——与 ++ 后缀不同,见类注释)
        Ok(result)
    }

    /// 对应 Java `getOperator()`:操作符词素 `"--"`。
    fn operator(&self) -> &str {
        "--"
    }

    /// 对应 Java `getPriority()`:`QLPrecedences.UNARY_SUFFIX`。
    fn priority(&self) -> i32 {
        ql_precedences::UNARY_SUFFIX
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
    use crate::runtime::data::assignable_data_value::AssignableDataValue;
    use crate::runtime::value::Value;
    use std::cell::RefCell;
    use std::rc::Rc;

    fn run(value: QValue) -> Result<DataValue, QLException> {
        MinusMinusSuffixUnaryOperator::get_instance()
            .execute(&value, &PureErrReporter::INSTANCE)
    }

    #[test]
    fn suffix_minus_minus_writes_back_and_returns_new_value() {
        // Java 原文 return result:a-- 槽内变 1,表达式值也是自减后的 1
        let slot = Rc::new(RefCell::new(AssignableDataValue::new("a", DataValue::Int(2))));
        let result = run(QValue::Left(slot.clone())).unwrap();
        assert_eq!(result, DataValue::Int(1));
        assert_eq!(slot.borrow().get(), DataValue::Int(1));
    }

    #[test]
    fn suffix_minus_minus_float_widens_to_double() {
        // Java FloatingPointMath.subtractImpl:doubleValue() - 1
        assert_eq!(
            run(QValue::from(DataValue::Float(2.5))).unwrap(),
            DataValue::Double(1.5)
        );
    }

    #[test]
    fn suffix_minus_minus_rejects_non_number() {
        let err = run(QValue::from(DataValue::Str("a".into()))).unwrap_err();
        assert_eq!(err.error_code(), error_codes::INVALID_UNARY_OPERAND);
    }
}
