//! 逻辑或操作符 `||` / `or`
//!
//! 对应 Java: com.alibaba.qlexpress4.runtime.operator.logic.LogicOrOperator
//! (@author bingo)。

use crate::exception::error_reporter::ErrorReporter;
use crate::exception::QLException;
use crate::ql_options::QLOptions;
use crate::ql_precedences;
use crate::runtime::qcontext::QContext;
use crate::runtime::value::{DataValue, QValue};

use crate::runtime::operator::base::base_binary_operator::BaseBinaryOperator;
use crate::runtime::operator::binary_operator::BinaryOperator;

/// 逻辑或操作符。
///
/// 对应 Java: LogicOrOperator(带词素参数的实例缓存 `预注册 "||" 与 "or"`;
/// Rust 以 `&'static str` 持有词素,`get_instance` 对齐 `getInstance(String)`)。
pub struct LogicOrOperator {
    /// Java `private final String operator`。
    operator: &'static str,
}

impl LogicOrOperator {
    /// Java `getInstance(String operator)`(Java 从 INSTANCE_CACHE 取,
    /// 未注册词素返回 null;Rust 返回 Option)。
    /// 对应 Java：`LogicOrOperator#getInstance(String)`。
    pub fn get_instance(operator: &'static str) -> Option<Self> {
        match operator {
            "||" => Some(LogicOrOperator { operator: "||" }),
            "or" => Some(LogicOrOperator { operator: "or" }),
            _ => None,
        }
    }
}

impl BinaryOperator for LogicOrOperator {
    /// 对应 Java 方法: `execute(...)` —— 逻辑或(null 视为 false,非 Boolean 报 INVALID_BINARY_OPERAND)。
    fn execute(
        &self,
        left: &QValue,
        right: &QValue,
        _q_context: &mut dyn QContext,
        _ql_options: &QLOptions,
        error_reporter: &dyn ErrorReporter,
    ) -> Result<DataValue, QLException> {
        logic_op(self.operator, left, right, error_reporter, |a, b| a || b)
    }

    /// 对应 Java 方法: `getOperator()`。
    fn operator(&self) -> &str {
        self.operator
    }

    /// 对应 Java 方法: `getPriority()` —— QLPrecedences.OR。
    fn priority(&self) -> i32 {
        ql_precedences::OR
    }
}

/// Java LogicAnd/LogicOrOperator.execute 的公共逻辑:null 操作数视为
/// false;两侧必须都是 Boolean,否则报 INVALID_BINARY_OPERAND。
fn logic_op(
    operator: &str,
    left: &QValue,
    right: &QValue,
    error_reporter: &dyn ErrorReporter,
    op: impl Fn(bool, bool) -> bool,
) -> Result<DataValue, QLException> {
    // Java: if (leftValue == null) leftValue = false; ...
    let left_value = match left.get() {
        DataValue::Null => DataValue::Bool(false),
        v => v,
    };
    let right_value = match right.get() {
        DataValue::Null => DataValue::Bool(false),
        v => v,
    };
    match (&left_value, &right_value) {
        (DataValue::Bool(l), DataValue::Bool(r)) => Ok(DataValue::Bool(op(*l, *r))),
        _ => Err(BaseBinaryOperator::build_invalid_operand_type_exception(
            operator,
            left,
            right,
            error_reporter,
        )),
    }
}
