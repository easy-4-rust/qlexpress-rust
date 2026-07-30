//! 逻辑非操作符 `!`(前缀一元)。
//!
//! 对应 Java: com.alibaba.qlexpress4.runtime.operator.logic.LogicNotOperator
//! (@author bingo;null 操作数视为 false,仅接受 Boolean)。

use crate::exception::error_reporter::ErrorReporter;
use crate::exception::QLException;
use crate::ql_precedences;
use crate::runtime::value::{DataValue, QValue};

use crate::runtime::operator::base::base_unary_operator::BaseUnaryOperator;
use crate::runtime::operator::base::UnaryOperator;
use crate::runtime::operator::Operator;

/// 逻辑非操作符 `!`。
///
/// 对应 Java: LogicNotOperator(单例模式,`getInstance()`)。
pub struct LogicNotOperator;

impl LogicNotOperator {
    /// Java `getInstance()` 单例获取(无状态,直接构造)。
    pub fn get_instance() -> Self {
        LogicNotOperator
    }
}

impl UnaryOperator for LogicNotOperator {
    /// 对应 Java 方法: `execute(Value value, ErrorReporter errorReporter)`。
    ///
    /// 语义要点:null 视为 false(!null == true);非 Boolean 操作数报
    /// INVALID_UNARY_OPERAND。
    fn execute(
        &self,
        value: &QValue,
        error_reporter: &dyn ErrorReporter,
    ) -> Result<DataValue, QLException> {
        // Java: if (operand == null) operand = false;
        let operand = match value.get() {
            DataValue::Null => DataValue::Bool(false),
            v => v,
        };
        match operand {
            DataValue::Bool(b) => Ok(DataValue::Bool(!b)),
            _ => Err(BaseUnaryOperator::build_invalid_operand_type_exception(
                "!",
                value,
                error_reporter,
            )),
        }
    }

    /// 对应 Java 方法: `getOperator()` —— `"!"`。
    fn operator(&self) -> &str {
        "!"
    }

    /// 对应 Java 方法: `getPriority()` —— QLPrecedences.UNARY。
    fn priority(&self) -> i32 {
        ql_precedences::UNARY
    }
}

/// Java `implements Operator` 的显式实现(一元操作符不参与 blanket impl)。
impl Operator for LogicNotOperator {
    fn operator(&self) -> &str {
        UnaryOperator::operator(self)
    }

    fn priority(&self) -> i32 {
        UnaryOperator::priority(self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::exception::pure_err_reporter::PureErrReporter;

    #[test]
    fn not_semantics() {
        let op = LogicNotOperator::get_instance();
        assert_eq!(
            op.execute(
                &QValue::Data(DataValue::Bool(true)),
                &PureErrReporter::INSTANCE
            )
            .unwrap(),
            DataValue::Bool(false)
        );
        // Java 语义要点:!null == true(null 视为 false)。
        assert_eq!(
            op.execute(&QValue::Data(DataValue::Null), &PureErrReporter::INSTANCE)
                .unwrap(),
            DataValue::Bool(true)
        );
        assert!(op
            .execute(&QValue::Data(DataValue::Int(1)), &PureErrReporter::INSTANCE)
            .is_err());
    }
}
