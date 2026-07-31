//! 乘法操作符 `*`
//!
//! 对应 Java: com.alibaba.qlexpress4.runtime.operator.arithmetic.MultiplyOperator
//! (@author bingo)。

use crate::exception::error_reporter::ErrorReporter;
use crate::exception::QLException;
use crate::ql_options::QLOptions;
use crate::ql_precedences;
use crate::runtime::qcontext::QContext;
use crate::runtime::value::{DataValue, QValue};

use crate::runtime::operator::base::base_binary_operator::BaseBinaryOperator;
use crate::runtime::operator::binary_operator::BinaryOperator;

/// 乘法操作符。
///
/// 对应 Java: MultiplyOperator(单例模式,`getInstance()`;Rust 为零大小类型)。
pub struct MultiplyOperator;

impl MultiplyOperator {
    /// Java `getInstance()` 单例获取(无状态,直接构造)。
    /// 对应 Java：`MultiplyOperator#getInstance()`。
    pub fn get_instance() -> Self {
        MultiplyOperator
    }
}

impl BinaryOperator for MultiplyOperator {
    /// 对应 Java 方法: `execute(...)` —— 数值相乘(仅 Number)。
    fn execute(
        &self,
        left: &QValue,
        right: &QValue,
        _q_context: &mut dyn QContext,
        ql_options: &QLOptions,
        error_reporter: &dyn ErrorReporter,
    ) -> Result<DataValue, QLException> {
        BaseBinaryOperator::multiply("*", left, right, ql_options, error_reporter)
    }

    /// 对应 Java 方法: `getOperator()` —— `"*"`。
    fn operator(&self) -> &str {
        "*"
    }

    /// 对应 Java 方法: `getPriority()` —— QLPrecedences.MULTI。
    fn priority(&self) -> i32 {
        ql_precedences::MULTI
    }
}
