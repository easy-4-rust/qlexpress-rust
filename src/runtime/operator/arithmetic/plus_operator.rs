//! 加法操作符 `+`(QLExpress 只支持 String 和 Number 类型的 +)
//!
//! 对应 Java: com.alibaba.qlexpress4.runtime.operator.arithmetic.PlusOperator
//! (@author bingo;QLExpress只支持String和Number类型的+)。

use crate::exception::error_reporter::ErrorReporter;
use crate::exception::QLException;
use crate::ql_options::QLOptions;
use crate::ql_precedences;
use crate::runtime::qcontext::QContext;
use crate::runtime::value::{DataValue, QValue};

use crate::runtime::operator::base::base_binary_operator::BaseBinaryOperator;
use crate::runtime::operator::binary_operator::BinaryOperator;

/// 加法操作符。
///
/// 对应 Java: PlusOperator(单例模式,`getInstance()`;Rust 为零大小类型)。
pub struct PlusOperator;

impl PlusOperator {
    /// Java `getInstance()` 单例获取(无状态,直接构造)。
    pub fn get_instance() -> Self {
        PlusOperator
    }
}

impl BinaryOperator for PlusOperator {
    /// 对应 Java 方法: `execute(...)` —— 字符串拼接或数值相加(含 precise 模式)。
    fn execute(
        &self,
        left: &QValue,
        right: &QValue,
        _q_context: &mut dyn QContext,
        ql_options: &QLOptions,
        error_reporter: &dyn ErrorReporter,
    ) -> Result<DataValue, QLException> {
        BaseBinaryOperator::plus("+", left, right, ql_options, error_reporter)
    }

    /// 对应 Java 方法: `getOperator()` —— `"+"`。
    fn operator(&self) -> &str {
        "+"
    }

    /// 对应 Java 方法: `getPriority()` —— QLPrecedences.ADD。
    fn priority(&self) -> i32 {
        ql_precedences::ADD
    }
}
