//! 一元操作符抽象基类。
//!
//! 对应 Java: com.alibaba.qlexpress4.runtime.operator.base.BaseUnaryOperator
//! (abstract class implements UnaryOperator;提供
//! `buildInvalidOperandTypeException` 供具体一元操作符复用)。

use crate::exception::error_codes;
use crate::exception::error_reporter::ErrorReporter;
use crate::exception::QLException;
use crate::runtime::value::QValue;

use crate::runtime::operator::number::number_math;

/// 对应 Java: BaseUnaryOperator(abstract,@author bingo)。
///
/// Rust 说明:同 BaseBinaryOperator,以零大小类型 + 关联函数代替
/// Java 的抽象基类继承;`operator` 参数对应 Java 的 `getOperator()`。
pub struct BaseUnaryOperator;

impl BaseUnaryOperator {
    /// Java `buildInvalidOperandTypeException(value, errorReporter)`
    /// —— INVALID_UNARY_OPERAND,参数顺序:操作符、类型名、值。
    /// 对应 Java：`BaseUnaryOperator#buildInvalidOperandTypeException(Value,ErrorReporter)`。
    pub fn build_invalid_operand_type_exception(
        operator: &str,
        value: &QValue,
        error_reporter: &dyn ErrorReporter,
    ) -> QLException {
        error_reporter.report_format(
            error_codes::INVALID_UNARY_OPERAND,
            error_codes::error_msg(error_codes::INVALID_UNARY_OPERAND),
            &[
                operator.to_string(),
                value.type_name().to_string(),
                number_math::java_value_string(&value.get()),
            ],
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::exception::pure_err_reporter::PureErrReporter;
    use crate::runtime::value::DataValue;

    /// SOURCE_PARITY: Java `BaseUnaryOperator#buildInvalidOperandTypeException`。
    #[test]
    fn invalid_operand_reports_operator_type_and_java_value_in_order() {
        let error = BaseUnaryOperator::build_invalid_operand_type_exception(
            "!",
            &QValue::Data(DataValue::string("text")),
            &PureErrReporter::INSTANCE,
        );

        assert_eq!(error.error_code(), error_codes::INVALID_UNARY_OPERAND);
        assert_eq!(
            error.reason(),
            "the '!' operator can not be applied to type java.lang.String with value text"
        );
    }
}
