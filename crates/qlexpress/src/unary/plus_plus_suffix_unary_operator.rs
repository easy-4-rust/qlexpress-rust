//! 后缀 `++` 操作符,对应 Java
//! `com.alibaba.qlexpress4.runtime.operator.unary.PlusPlusSuffixUnaryOperator`。

use crate::exception::error_codes;
use crate::exception::error_reporter::ErrorReporter;
use crate::exception::QLException;
use crate::ql_precedences;
use crate::runtime::operator::base::UnaryOperator;
use crate::runtime::value::{DataValue, QValue};

/// 后缀 `++` 操作符(先取值,后自增)。
///
/// 对应 Java: com.alibaba.qlexpress4.runtime.operator.unary.PlusPlusSuffixUnaryOperator
/// (@author bingo;执行体委托 `NumberMath.add(operand, 1)`,错误构造继承自
/// `BaseUnaryOperator`)。
#[derive(Clone, Copy, Debug, Default)]
pub struct PlusPlusSuffixUnaryOperator;

impl PlusPlusSuffixUnaryOperator {
    /// 对应 Java `PlusPlusSuffixUnaryOperator.getInstance()` 单例。
    pub fn get_instance() -> PlusPlusSuffixUnaryOperator {
        PlusPlusSuffixUnaryOperator
    }
}

impl UnaryOperator for PlusPlusSuffixUnaryOperator {
    /// 对应 Java `PlusPlusSuffixUnaryOperator.execute(Value value,
    /// ErrorReporter)`:
    /// ```java
    /// Object operand = value.get();
    /// if (!(operand instanceof Number)) throw ...;
    /// if (value instanceof LeftValue)
    ///     ((LeftValue)value).set(NumberMath.add((Number)operand, 1), errorReporter);
    /// return operand;
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

        // Java:value instanceof LeftValue → set(NumberMath.add(operand, 1))
        // 数值加一逻辑与前缀 `++` 共享(Java 均委托 NumberMath.add)。
        if let Some(left_value) = value.as_left() {
            let result = super::plus_plus_prefix_unary_operator::number_add_one(&operand);
            left_value.borrow_mut().set(result, error_reporter)?;
        }
        // Java 后缀:return operand(自增前的原值)——与前缀的唯一差异
        Ok(operand)
    }

    /// 对应 Java `getOperator()`:操作符词素 `"++"`。
    fn operator(&self) -> &str {
        "++"
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
        PlusPlusSuffixUnaryOperator::get_instance().execute(&value, &PureErrReporter::INSTANCE)
    }

    #[test]
    fn suffix_plus_plus_returns_original_and_writes_back() {
        // Java:a++ → 表达式值是自增前的 1,槽内变 2(与前缀的关键差异)
        let slot = Rc::new(RefCell::new(AssignableDataValue::new(
            "a",
            DataValue::Int(1),
        )));
        let result = run(QValue::Left(slot.clone())).unwrap();
        assert_eq!(result, DataValue::Int(1));
        assert_eq!(slot.borrow().get(), DataValue::Int(2));
    }

    #[test]
    fn suffix_plus_plus_on_immutable_value_returns_operand() {
        // Java:非 LeftValue 不写回,但仍返回原操作数
        assert_eq!(
            run(QValue::from(DataValue::Long(5))).unwrap(),
            DataValue::Long(5)
        );
        assert_eq!(
            run(QValue::from(DataValue::BigDec("1.50".into()))).unwrap(),
            DataValue::BigDec("1.50".into())
        );
    }

    #[test]
    fn suffix_plus_plus_rejects_non_number() {
        let err = run(QValue::from(DataValue::Bool(true))).unwrap_err();
        assert_eq!(err.error_code(), error_codes::INVALID_UNARY_OPERAND);
    }
}
