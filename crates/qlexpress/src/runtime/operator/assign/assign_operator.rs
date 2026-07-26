//! `=` 赋值操作符,对应 Java
//! `com.alibaba.qlexpress4.runtime.operator.assign.AssignOperator`。

use crate::exception::error_codes;
use crate::exception::error_reporter::ErrorReporter;
use crate::exception::QLException;
use crate::ql_options::QLOptions;
use crate::ql_precedences;
use crate::runtime::left_value::LeftValue;
use crate::runtime::operator::base::BinaryOperator;
use crate::runtime::qcontext::QContext;
use crate::runtime::value::{DataValue, QValue};

/// `=` 赋值二元操作符。
///
/// 对应 Java: com.alibaba.qlexpress4.runtime.operator.assign.AssignOperator
/// (@author bingo;`assertLeftValue` 继承自 `BaseBinaryOperator`)。
#[derive(Clone, Copy, Debug, Default)]
pub struct AssignOperator;

impl AssignOperator {
    /// 对应 Java `AssignOperator.getInstance()` 单例。
    pub fn get_instance() -> AssignOperator {
        AssignOperator
    }
}

impl BinaryOperator for AssignOperator {
    /// 对应 Java `AssignOperator.execute(Value left, Value right, QRuntime,
    /// QLOptions, ErrorReporter)`:
    /// ```java
    /// assertLeftValue(left, errorReporter);
    /// LeftValue leftValue = (LeftValue)left;
    /// Object newValue = right.get();
    /// leftValue.set(newValue, errorReporter);
    /// return newValue;
    /// ```
    fn execute(
        &self,
        left: &QValue,
        right: &QValue,
        _q_context: &mut dyn QContext,
        _ql_options: &QLOptions,
        error_reporter: &dyn ErrorReporter,
    ) -> Result<DataValue, QLException> {
        assign(left, right, error_reporter)
    }

    /// 对应 Java `getOperator()`:操作符词素 `"="`。
    fn operator(&self) -> &str {
        "="
    }

    /// 对应 Java `getPriority()`:`QLPrecedences.ASSIGN`。
    fn priority(&self) -> i32 {
        ql_precedences::ASSIGN
    }
}

/// 赋值核心逻辑,对应 Java `AssignOperator.execute` 的方法体。
///
/// Java 语义要点:
/// - 左操作数必须是 `LeftValue`,否则 `assertLeftValue` 抛
///   `INVALID_ASSIGNMENT`(参数 "on the left side");
/// - `LeftValue.set(newValue, errorReporter)` 内部按声明类型做
///   `ObjTypeConvertor` 转换,不兼容时抛 `INCOMPATIBLE_ASSIGNMENT_TYPE`
///   (见 [`crate::runtime::left_value::LeftValue::set`]);
/// - 返回值是**转换前**的 `right.get()`(Java 返回局部变量 `newValue`,
///   不是 `LeftValue` 转换后的内部值)。
pub(crate) fn assign(
    left: &QValue,
    right: &QValue,
    error_reporter: &dyn ErrorReporter,
) -> Result<DataValue, QLException> {
    // Java:assertLeftValue(left, errorReporter)
    let left_value = match left.as_left() {
        Some(left_value) => left_value,
        None => {
            return Err(error_reporter.report_format(
                error_codes::INVALID_ASSIGNMENT,
                error_codes::error_msg(error_codes::INVALID_ASSIGNMENT),
                // Java 原文实参:"on the left side"
                &["on the left side".to_string()],
            ));
        }
    };
    let new_value = right.get();
    // Java:leftValue.set(newValue, errorReporter) — 声明类型转换在 set 内
    left_value
        .borrow_mut()
        .set(new_value.clone(), error_reporter)?;
    // Java:return newValue(转换前的右值)
    Ok(new_value)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::exception::pure_err_reporter::PureErrReporter;
    use crate::runtime::data::assignable_data_value::AssignableDataValue;
    use crate::runtime::data::convert::obj_type_convertor::TargetType;
    use crate::runtime::value::Value;
    use std::cell::RefCell;
    use std::rc::Rc;

    fn run(left: QValue, right: DataValue) -> Result<DataValue, QLException> {
        assign(&left, &QValue::from(right), &PureErrReporter::INSTANCE)
    }

    #[test]
    fn assign_sets_left_value_and_returns_right_value() {
        // Java:a = 1 → 变量槽被写穿,表达式值为右值
        let slot = Rc::new(RefCell::new(AssignableDataValue::new("a", DataValue::Null)));
        let result = run(QValue::Left(slot.clone()), DataValue::Int(42)).unwrap();
        assert_eq!(result, DataValue::Int(42));
        assert_eq!(slot.borrow().get(), DataValue::Int(42));
    }

    #[test]
    fn assign_converts_to_declared_type() {
        // Java:int a = 1L → LeftValue.set 内按声明类型 int 转换
        let slot = Rc::new(RefCell::new(AssignableDataValue::with_type(
            "a",
            DataValue::Null,
            TargetType::Int,
        )));
        run(QValue::Left(slot.clone()), DataValue::Long(7)).unwrap();
        assert_eq!(slot.borrow().get(), DataValue::Int(7));
    }

    #[test]
    fn assign_incompatible_type_reports_error_code() {
        // Java:String s = 1 → INCOMPATIBLE_ASSIGNMENT_TYPE
        let slot = Rc::new(RefCell::new(AssignableDataValue::with_type(
            "s",
            DataValue::Null,
            TargetType::Boolean,
        )));
        let err = run(QValue::Left(slot), DataValue::Int(1)).unwrap_err();
        assert_eq!(err.error_code(), error_codes::INCOMPATIBLE_ASSIGNMENT_TYPE);
    }

    #[test]
    fn assign_to_non_left_value_rejected() {
        // Java:assertLeftValue → INVALID_ASSIGNMENT(字面量不可赋值)
        let err = run(QValue::from(DataValue::Int(1)), DataValue::Int(2)).unwrap_err();
        assert_eq!(err.error_code(), error_codes::INVALID_ASSIGNMENT);
    }
}
