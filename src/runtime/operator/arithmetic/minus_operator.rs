//! 减法操作符 `-`
//!
//! 对应 Java: com.alibaba.qlexpress4.runtime.operator.arithmetic.MinusOperator
//! (@author bingo)。

use crate::exception::error_reporter::ErrorReporter;
use crate::exception::QLException;
use crate::ql_options::QLOptions;
use crate::ql_precedences;
use crate::runtime::qcontext::QContext;
use crate::runtime::value::{DataValue, QValue};

use crate::runtime::operator::base::base_binary_operator::BaseBinaryOperator;
use crate::runtime::operator::binary_operator::BinaryOperator;

/// 减法操作符。
///
/// 对应 Java: MinusOperator(单例模式,`getInstance()`;Rust 为零大小类型)。
pub struct MinusOperator;

impl MinusOperator {
    /// Java `getInstance()` 单例获取(无状态,直接构造)。
    pub fn get_instance() -> Self {
        MinusOperator
    }
}

impl BinaryOperator for MinusOperator {
    /// 对应 Java 方法: `execute(...)` —— 数值相减(Character 按码点参与)。
    fn execute(
        &self,
        left: &QValue,
        right: &QValue,
        _q_context: &mut dyn QContext,
        ql_options: &QLOptions,
        error_reporter: &dyn ErrorReporter,
    ) -> Result<DataValue, QLException> {
        BaseBinaryOperator::minus("-", left, right, ql_options, error_reporter)
    }

    /// 对应 Java 方法: `getOperator()` —— `"-"`。
    fn operator(&self) -> &str {
        "-"
    }

    /// 对应 Java 方法: `getPriority()` —— QLPrecedences.ADD。
    fn priority(&self) -> i32 {
        ql_precedences::ADD
    }
}
