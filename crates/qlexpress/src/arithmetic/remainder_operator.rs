//! 取余操作符 `%`
//!
//! 对应 Java: com.alibaba.qlexpress4.runtime.operator.arithmetic.RemainderOperator
//! (@author bingo)。

use crate::exception::error_reporter::ErrorReporter;
use crate::exception::QLException;
use crate::ql_options::QLOptions;
use crate::ql_precedences;
use crate::runtime::qcontext::QContext;
use crate::runtime::value::{DataValue, QValue};

use crate::runtime::operator::base::base_binary_operator::BaseBinaryOperator;
use crate::runtime::operator::binary_operator::BinaryOperator;

/// 取余操作符。
///
/// 对应 Java: RemainderOperator(带词素参数的实例缓存 `仅预注册 "%";"mod" 在 Java 中被注释掉`;
/// Rust 以 `&'static str` 持有词素,`get_instance` 对齐 `getInstance(String)`)。
pub struct RemainderOperator {
    /// Java `private final String operator`。
    operator: &'static str,
}

impl RemainderOperator {
    /// Java `getInstance(String operator)`(Java 从 INSTANCE_CACHE 取,
    /// 未注册词素返回 null;Rust 返回 Option)。
    /// 对应 Java：`RemainderOperator#getInstance(String)`。
    pub fn get_instance(operator: &'static str) -> Option<Self> {
        match operator {
            "%" => Some(RemainderOperator { operator: "%" }),
            _ => None,
        }
    }
}

impl BinaryOperator for RemainderOperator {
    /// 对应 Java 方法: `execute(...)` —— 数值取余(符号跟被除数)。
    fn execute(
        &self,
        left: &QValue,
        right: &QValue,
        _q_context: &mut dyn QContext,
        ql_options: &QLOptions,
        error_reporter: &dyn ErrorReporter,
    ) -> Result<DataValue, QLException> {
        BaseBinaryOperator::remainder(self.operator, left, right, ql_options, error_reporter)
    }

    /// 对应 Java 方法: `getOperator()`。
    fn operator(&self) -> &str {
        self.operator
    }

    /// 对应 Java 方法: `getPriority()` —— QLPrecedences.MULTI。
    fn priority(&self) -> i32 {
        ql_precedences::MULTI
    }
}
