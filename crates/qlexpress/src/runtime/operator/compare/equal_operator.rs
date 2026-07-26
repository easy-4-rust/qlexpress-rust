//! 相等操作符 `==`(跨数值类型按数值比较)
//!
//! 对应 Java: com.alibaba.qlexpress4.runtime.operator.compare.EqualOperator
//! (@author bingo)。

use crate::exception::error_reporter::ErrorReporter;
use crate::exception::QLException;
use crate::ql_options::QLOptions;
use crate::ql_precedences;
use crate::runtime::qcontext::QContext;
use crate::runtime::value::{DataValue, QValue};

use crate::runtime::operator::base::base_binary_operator::BaseBinaryOperator;
use crate::runtime::operator::binary_operator::BinaryOperator;

/// 相等操作符。
///
/// 对应 Java: EqualOperator(单例模式,`getInstance()`;Rust 为零大小类型)。
pub struct EqualOperator;

impl EqualOperator {
    /// Java `getInstance()` 单例获取(无状态,直接构造)。
    pub fn get_instance() -> Self {
        EqualOperator
    }
}

impl BinaryOperator for EqualOperator {
    /// 对应 Java 方法: `execute(...)` —— equals 语义(数值跨类型)。
    fn execute(
        &self,
        left: &QValue,
        right: &QValue,
        _q_context: &mut dyn QContext,
        _ql_options: &QLOptions,
        error_reporter: &dyn ErrorReporter,
    ) -> Result<DataValue, QLException> {
        Ok(DataValue::Bool(BaseBinaryOperator::equals("==", left, right, error_reporter)?))
    }

    /// 对应 Java 方法: `getOperator()` —— `"=="`。
    fn operator(&self) -> &str {
        "=="
    }

    /// 对应 Java 方法: `getPriority()` —— QLPrecedences.EQUAL。
    fn priority(&self) -> i32 {
        ql_precedences::EQUAL
    }
}
