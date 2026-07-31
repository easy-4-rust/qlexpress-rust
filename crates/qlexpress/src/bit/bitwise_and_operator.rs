//! 按位与操作符 `&`(Boolean 操作数按逻辑与,null 视为 false)
//!
//! 对应 Java: com.alibaba.qlexpress4.runtime.operator.bit.BitwiseAndOperator
//! (@author bingo)。

use crate::exception::error_reporter::ErrorReporter;
use crate::exception::QLException;
use crate::ql_options::QLOptions;
use crate::ql_precedences;
use crate::runtime::qcontext::QContext;
use crate::runtime::value::{DataValue, QValue};

use crate::runtime::operator::base::base_binary_operator::BaseBinaryOperator;
use crate::runtime::operator::binary_operator::BinaryOperator;

/// 按位与操作符 `&`(Boolean 操作数按逻辑与,null 视为 false)。
///
/// 对应 Java: BitwiseAndOperator(单例模式,`getInstance()`;Rust 为零大小类型)。
pub struct BitwiseAndOperator;

impl BitwiseAndOperator {
    /// Java `getInstance()` 单例获取(无状态,直接构造)。
    /// 对应 Java：`BitwiseAndOperator#getInstance()`。
    pub fn get_instance() -> Self {
        BitwiseAndOperator
    }
}

impl BinaryOperator for BitwiseAndOperator {
    /// 对应 Java 方法: `execute(...)` —— 位运算。
    fn execute(
        &self,
        left: &QValue,
        right: &QValue,
        _q_context: &mut dyn QContext,
        _ql_options: &QLOptions,
        error_reporter: &dyn ErrorReporter,
    ) -> Result<DataValue, QLException> {
        BaseBinaryOperator::bitwise_and("&", left, right, error_reporter)
    }

    /// 对应 Java 方法: `getOperator()` —— `"&"`。
    fn operator(&self) -> &str {
        "&"
    }

    /// 对应 Java 方法: `getPriority()` —— QLPrecedences.BIT_AND。
    fn priority(&self) -> i32 {
        ql_precedences::BIT_AND
    }
}
