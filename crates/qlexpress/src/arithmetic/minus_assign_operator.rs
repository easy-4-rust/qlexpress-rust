//! 减赋值操作符 `-=`
//!
//! 对应 Java: com.alibaba.qlexpress4.runtime.operator.arithmetic.MinusAssignOperator
//! (@author bingo)。

use crate::exception::error_reporter::ErrorReporter;
use crate::exception::QLException;
use crate::ql_options::QLOptions;
use crate::ql_precedences;
use crate::runtime::qcontext::QContext;
use crate::runtime::value::{DataValue, QValue};

use crate::runtime::operator::base::base_binary_operator::BaseBinaryOperator;
use crate::runtime::operator::binary_operator::BinaryOperator;

/// 减赋值操作符。
///
/// 对应 Java: MinusAssignOperator(复合赋值操作符:要求左操作数为 LeftValue,
/// 计算后写回并返回结果)。
pub struct MinusAssignOperator;

impl MinusAssignOperator {
    /// Java `getInstance()` 单例获取(无状态,直接构造)。
    /// 对应 Java：`MinusAssignOperator#getInstance()`。
    pub fn get_instance() -> Self {
        MinusAssignOperator
    }
}

impl BinaryOperator for MinusAssignOperator {
    /// 对应 Java 方法: `execute(...)`。
    ///
    /// 语义要点(Java 原版逐行对齐):先 `assertLeftValue`,再计算
    /// `minus(...)`,随后 `leftValue.set(result, errorReporter)` 写回
    /// 左值(Java 经引用别名写穿),最终返回计算结果。
    fn execute(
        &self,
        left: &QValue,
        right: &QValue,
        _q_context: &mut dyn QContext,
        ql_options: &QLOptions,
        error_reporter: &dyn ErrorReporter,
    ) -> Result<DataValue, QLException> {
        BaseBinaryOperator::assert_left_value(left, error_reporter)?;
        let result = BaseBinaryOperator::minus("-=", left, right, ql_options, error_reporter)?;
        // Java: ((LeftValue)left).set(result, errorReporter)
        let left_value = left.as_left().expect("assert_left_value 已校验");
        left_value
            .borrow_mut()
            .set(result.clone(), error_reporter)?;
        Ok(result)
    }

    /// 对应 Java 方法: `getOperator()` —— `"-="`。
    fn operator(&self) -> &str {
        "-="
    }

    /// 对应 Java 方法: `getPriority()` —— QLPrecedences.ASSIGN。
    fn priority(&self) -> i32 {
        ql_precedences::ASSIGN
    }
}
