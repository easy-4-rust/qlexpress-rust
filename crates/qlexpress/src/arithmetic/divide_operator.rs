//! 除法操作符 `/`
//!
//! 对应 Java: com.alibaba.qlexpress4.runtime.operator.arithmetic.DivideOperator
//! (@author bingo)。

use crate::exception::error_reporter::ErrorReporter;
use crate::exception::QLException;
use crate::ql_options::QLOptions;
use crate::ql_precedences;
use crate::runtime::qcontext::QContext;
use crate::runtime::value::{DataValue, QValue};

use crate::runtime::operator::base::base_binary_operator::BaseBinaryOperator;
use crate::runtime::operator::binary_operator::BinaryOperator;

/// 除法操作符。
///
/// 对应 Java: DivideOperator(单例模式,`getInstance()`;Rust 为零大小类型)。
pub struct DivideOperator;

impl DivideOperator {
    /// Java `getInstance()` 单例获取(无状态,直接构造)。
    /// 对应 Java：`DivideOperator#getInstance()`。
    pub fn get_instance() -> Self {
        DivideOperator
    }
}

impl BinaryOperator for DivideOperator {
    /// 对应 Java 方法: `execute(...)` —— 数值相除(整型结果 BigDecimal,浮点按 IEEE)。
    fn execute(
        &self,
        left: &QValue,
        right: &QValue,
        _q_context: &mut dyn QContext,
        ql_options: &QLOptions,
        error_reporter: &dyn ErrorReporter,
    ) -> Result<DataValue, QLException> {
        BaseBinaryOperator::divide("/", left, right, ql_options, error_reporter)
    }

    /// 对应 Java 方法: `getOperator()` —— `"/"`。
    fn operator(&self) -> &str {
        "/"
    }

    /// 对应 Java 方法: `getPriority()` —— QLPrecedences.MULTI。
    fn priority(&self) -> i32 {
        ql_precedences::MULTI
    }
}
