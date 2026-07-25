//! 按位取反操作符 `~`(前缀一元)。
//!
//! 对应 Java: com.alibaba.qlexpress4.runtime.operator.bit.BitwiseInvertOperator
//! (@author bingo;仅接受 Number 操作数,委托 `NumberMath.bitwiseNegate`)。

use crate::exception::error_reporter::ErrorReporter;
use crate::exception::QLException;
use crate::ql_precedences;
use crate::runtime::value::{DataValue, QValue};

use crate::runtime::operator::base::base_unary_operator::BaseUnaryOperator;
use crate::runtime::operator::base::UnaryOperator;
use crate::runtime::operator::number::number_math::NumberMath;
use crate::runtime::operator::operator::Operator;

/// 按位取反操作符 `~`。
///
/// 对应 Java: BitwiseInvertOperator(单例模式,`getInstance()`)。
pub struct BitwiseInvertOperator;

impl BitwiseInvertOperator {
    /// Java `getInstance()` 单例获取(无状态,直接构造)。
    pub fn get_instance() -> Self {
        BitwiseInvertOperator
    }
}

impl UnaryOperator for BitwiseInvertOperator {
    /// 对应 Java 方法: `execute(Value value, ErrorReporter errorReporter)`
    /// —— 操作数必须是 Number,否则 INVALID_UNARY_OPERAND;按操作数
    /// 类型决定位宽(int/long/BigInteger 域)。
    fn execute(
        &self,
        value: &QValue,
        error_reporter: &dyn ErrorReporter,
    ) -> Result<DataValue, QLException> {
        let operand = value.get();
        if !operand.is_number() {
            return Err(BaseUnaryOperator::build_invalid_operand_type_exception(
                "~",
                value,
                error_reporter,
            ));
        }
        NumberMath::bitwise_negate(&operand)
    }

    /// 对应 Java 方法: `getOperator()` —— `"~"`。
    fn operator(&self) -> &str {
        "~"
    }

    /// 对应 Java 方法: `getPriority()` —— QLPrecedences.UNARY。
    fn priority(&self) -> i32 {
        ql_precedences::UNARY
    }
}

/// Java `implements Operator` 的显式实现(一元操作符不参与 blanket impl)。
impl Operator for BitwiseInvertOperator {
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
    fn invert_int_and_long() {
        // ~5 = -6(int 域);~5L = -6L(long 域)。
        assert_eq!(
            BitwiseInvertOperator::get_instance()
                .execute(&QValue::Data(DataValue::Int(5)), &PureErrReporter::INSTANCE)
                .unwrap(),
            DataValue::Int(-6)
        );
        assert_eq!(
            BitwiseInvertOperator::get_instance()
                .execute(&QValue::Data(DataValue::Long(5)), &PureErrReporter::INSTANCE)
                .unwrap(),
            DataValue::Long(-6)
        );
        // 非数值操作数报错。
        assert!(BitwiseInvertOperator::get_instance()
            .execute(&QValue::Data(DataValue::Bool(true)), &PureErrReporter::INSTANCE)
            .is_err());
    }
}
