//! 逻辑右移操作符 `>>>`(高位补零)
//!
//! 对应 Java: com.alibaba.qlexpress4.runtime.operator.bit.BitwiseRightShiftUnsignedOperator
//! (@author bingo)。

use crate::exception::error_reporter::ErrorReporter;
use crate::exception::QLException;
use crate::ql_options::QLOptions;
use crate::ql_precedences;
use crate::runtime::qcontext::QContext;
use crate::runtime::value::{DataValue, QValue};

use crate::runtime::operator::base::base_binary_operator::BaseBinaryOperator;
use crate::runtime::operator::binary_operator::BinaryOperator;

/// 逻辑右移操作符 `>>>`(高位补零)。
///
/// 对应 Java: BitwiseRightShiftUnsignedOperator(单例模式,`getInstance()`;Rust 为零大小类型)。
pub struct BitwiseRightShiftUnsignedOperator;

impl BitwiseRightShiftUnsignedOperator {
    /// Java `getInstance()` 单例获取(无状态,直接构造)。
    /// 对应 Java：`BitwiseRightShiftUnsignedOperator#getInstance()`。
    pub fn get_instance() -> Self {
        BitwiseRightShiftUnsignedOperator
    }
}

impl BinaryOperator for BitwiseRightShiftUnsignedOperator {
    /// 对应 Java 方法: `execute(...)` —— 位运算。
    fn execute(
        &self,
        left: &QValue,
        right: &QValue,
        _q_context: &mut dyn QContext,
        _ql_options: &QLOptions,
        error_reporter: &dyn ErrorReporter,
    ) -> Result<DataValue, QLException> {
        BaseBinaryOperator::right_shift_unsigned(">>>", left, right, error_reporter)
    }

    /// 对应 Java 方法: `getOperator()` —— `">>>"`。
    fn operator(&self) -> &str {
        ">>>"
    }

    /// 对应 Java 方法: `getPriority()` —— QLPrecedences.BIT_MOVE。
    fn priority(&self) -> i32 {
        ql_precedences::BIT_MOVE
    }
}
