//! 小于等于操作符 `<=`
//!
//! 对应 Java: com.alibaba.qlexpress4.runtime.operator.compare.LessEqualOperator
//! (@author bingo)。

use crate::exception::error_reporter::ErrorReporter;
use crate::exception::QLException;
use crate::ql_options::QLOptions;
use crate::ql_precedences;
use crate::runtime::qcontext::QContext;
use crate::runtime::value::{DataValue, QValue};

use crate::runtime::operator::base::base_binary_operator::BaseBinaryOperator;
use crate::runtime::operator::binary_operator::BinaryOperator;

/// 小于等于操作符 `<=`。
///
/// 对应 Java: LessEqualOperator(单例模式,`getInstance()`;Rust 为零大小类型)。
pub struct LessEqualOperator;

impl LessEqualOperator {
    /// Java `getInstance()` 单例获取(无状态,直接构造)。
    /// 对应 Java：`LessEqualOperator#getInstance()`。
    pub fn get_instance() -> Self {
        LessEqualOperator
    }
}

impl BinaryOperator for LessEqualOperator {
    /// 对应 Java 方法: `execute(...)` —— compare 比较;`avoidNullPointer` 模式下遇 null 返回 false。
    fn execute(
        &self,
        left: &QValue,
        right: &QValue,
        _q_context: &mut dyn QContext,
        ql_options: &QLOptions,
        error_reporter: &dyn ErrorReporter,
    ) -> Result<DataValue, QLException> {
        // Java: qlOptions.isAvoidNullPointer() && (left.get() == null || right.get() == null) -> false
        if ql_options.is_avoid_null_pointer() && (left.get().is_null() || right.get().is_null()) {
            return Ok(DataValue::Bool(false));
        }
        Ok(DataValue::Bool(
            BaseBinaryOperator::compare("<=", left, right, error_reporter)? <= 0,
        ))
    }

    /// 对应 Java 方法: `getOperator()` —— `"<="`。
    fn operator(&self) -> &str {
        "<="
    }

    /// 对应 Java 方法: `getPriority()` —— QLPrecedences.COMPARE。
    fn priority(&self) -> i32 {
        ql_precedences::COMPARE
    }
}
