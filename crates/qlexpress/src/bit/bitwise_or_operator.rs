//! 按位或操作符 `|`(Boolean 操作数按逻辑或,null 视为 false)
//!
//! 对应 Java: com.alibaba.qlexpress4.runtime.operator.bit.BitwiseOrOperator
//! (@author bingo)。

use crate::exception::error_reporter::ErrorReporter;
use crate::exception::QLException;
use crate::ql_options::QLOptions;
use crate::ql_precedences;
use crate::runtime::qcontext::QContext;
use crate::runtime::value::{DataValue, QValue};

use crate::runtime::operator::base::base_binary_operator::BaseBinaryOperator;
use crate::runtime::operator::binary_operator::BinaryOperator;

/// 按位或操作符 `|`(Boolean 操作数按逻辑或,null 视为 false)。
///
/// 对应 Java: BitwiseOrOperator(单例模式,`getInstance()`;Rust 为零大小类型)。
pub struct BitwiseOrOperator;

impl BitwiseOrOperator {
    /// Java `getInstance()` 单例获取(无状态,直接构造)。
    /// 对应 Java：`BitwiseOrOperator#getInstance()`。
    pub fn get_instance() -> Self {
        BitwiseOrOperator
    }
}

impl BinaryOperator for BitwiseOrOperator {
    /// 对应 Java 方法: `execute(...)` —— 位运算。
    fn execute(
        &self,
        left: &QValue,
        right: &QValue,
        _q_context: &mut dyn QContext,
        _ql_options: &QLOptions,
        error_reporter: &dyn ErrorReporter,
    ) -> Result<DataValue, QLException> {
        BaseBinaryOperator::bitwise_or("|", left, right, error_reporter)
    }

    /// 对应 Java 方法: `getOperator()` —— `"|"`。
    fn operator(&self) -> &str {
        "|"
    }

    /// 对应 Java 方法: `getPriority()` —— QLPrecedences.BIT_OR。
    fn priority(&self) -> i32 {
        ql_precedences::BIT_OR
    }
}
