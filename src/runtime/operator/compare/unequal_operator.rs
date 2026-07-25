//! 不等操作符 `!=` / `<>`
//!
//! 对应 Java: com.alibaba.qlexpress4.runtime.operator.compare.UnequalOperator
//! (@author bingo)。

use crate::exception::error_reporter::ErrorReporter;
use crate::exception::QLException;
use crate::ql_options::QLOptions;
use crate::ql_precedences;
use crate::runtime::qcontext::QContext;
use crate::runtime::value::{DataValue, QValue};

use crate::runtime::operator::base::base_binary_operator::BaseBinaryOperator;
use crate::runtime::operator::binary_operator::BinaryOperator;

/// 不等操作符。
///
/// 对应 Java: UnequalOperator(带词素参数的实例缓存 `预注册 "!=" 与 "<>"`;
/// Rust 以 `&'static str` 持有词素,`get_instance` 对齐 `getInstance(String)`)。
pub struct UnequalOperator {
    /// Java `private final String operator`。
    operator: &'static str,
}

impl UnequalOperator {
    /// Java `getInstance(String operator)`(Java 从 INSTANCE_CACHE 取,
    /// 未注册词素返回 null;Rust 返回 Option)。
    pub fn get_instance(operator: &'static str) -> Option<Self> {
        match operator {
            "!=" => Some(UnequalOperator { operator: "!=" }),
            "<>" => Some(UnequalOperator { operator: "<>" }),
            _ => None,
        }
    }
}

impl BinaryOperator for UnequalOperator {
    /// 对应 Java 方法: `execute(...)` —— equals 取反。
    fn execute(
        &self,
        left: &QValue,
        right: &QValue,
        _q_context: &mut dyn QContext,
        _ql_options: &QLOptions,
        error_reporter: &dyn ErrorReporter,
    ) -> Result<DataValue, QLException> {
        Ok(DataValue::Bool(!BaseBinaryOperator::equals(self.operator, left, right, error_reporter)?))
    }

    /// 对应 Java 方法: `getOperator()`。
    fn operator(&self) -> &str {
        self.operator
    }

    /// 对应 Java 方法: `getPriority()` —— QLPrecedences.EQUAL。
    fn priority(&self) -> i32 {
        ql_precedences::EQUAL
    }
}
