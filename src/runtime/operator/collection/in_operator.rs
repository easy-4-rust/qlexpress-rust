//! `in` 操作符,对应 Java
//! `com.alibaba.qlexpress4.runtime.operator.collection.InOperator`。

use crate::exception::error_codes;
use crate::exception::error_reporter::ErrorReporter;
use crate::exception::QLException;
use crate::ql_options::QLOptions;
use crate::ql_precedences;
use crate::runtime::data::convert::number_compare;
use crate::runtime::operator::base::BinaryOperator;
use crate::runtime::qcontext::QContext;
use crate::runtime::value::{DataValue, QValue};

/// `in` 二元操作符(包含判断)。
///
/// 对应 Java: com.alibaba.qlexpress4.runtime.operator.collection.InOperator
/// (@author bingo;语义主体在其父类 `BaseBinaryOperator.in` 与 `equals` 中,
/// 本文件自带与 Java 对等的完整逻辑)。
#[derive(Clone, Copy, Debug, Default)]
pub struct InOperator;

impl InOperator {
    /// 对应 Java `InOperator.getInstance()` 单例。
    pub fn get_instance() -> InOperator {
        InOperator
    }
}

impl BinaryOperator for InOperator {
    /// 对应 Java `InOperator.execute(Value left, Value right, QRuntime,
    /// QLOptions, ErrorReporter)`:委托给 `in(left, right, errorReporter)`。
    fn execute(
        &self,
        left: &QValue,
        right: &QValue,
        _q_context: &mut dyn QContext,
        _ql_options: &QLOptions,
        error_reporter: &dyn ErrorReporter,
    ) -> Result<DataValue, QLException> {
        Ok(DataValue::Bool(op_in(
            left,
            right,
            self.operator(),
            error_reporter,
        )?))
    }

    /// 对应 Java `getOperator()`:操作符词素 `"in"`。
    fn operator(&self) -> &str {
        "in"
    }

    /// 对应 Java `getPriority()`:`QLPrecedences.IN_LIKE`。
    fn priority(&self) -> i32 {
        ql_precedences::IN_LIKE
    }
}

/// 对应 Java `BaseBinaryOperator.in(Value, Value, ErrorReporter)`。
///
/// Java 语义要点(逐条对齐):
/// - 两侧均为 `null` → `true`;仅一侧为 `null` → `false`;
/// - 右操作数是 `Collection`(→ [`DataValue::List`]):逐元素用
///   `BaseBinaryOperator.equals` 判定包含;
/// - 右操作数是 Java 数组(→ [`DataValue::Array`]):同上逐元素判定;
/// - 右操作数是 `String`:`right.contains(String.valueOf(left))`,
///   即左操作数先按 Java `String.valueOf` 渲染再查子串;
/// - **Map 不在其列**:Java 的 `Map` 不是 `Collection`,右操作数为 map
///   时落入 else 分支,抛 `INVALID_BINARY_OPERAND`(不是 key 也不是
///   value 包含)。
pub(crate) fn op_in(
    left: &QValue,
    right: &QValue,
    operator: &str,
    error_reporter: &dyn ErrorReporter,
) -> Result<bool, QLException> {
    let left_operand = left.get();
    let right_operand = right.get();
    // Java:leftOperand == null && rightOperand == null → true
    if left_operand.is_null() && right_operand.is_null() {
        return Ok(true);
    }
    // Java:leftOperand == null || rightOperand == null → false
    if left_operand.is_null() || right_operand.is_null() {
        return Ok(false);
    }

    match &right_operand {
        // Java:rightOperand instanceof Collection
        DataValue::List(elements) => {
            for element in elements.borrow().iter() {
                if op_equals(&left_operand, element) {
                    return Ok(true);
                }
            }
            Ok(false)
        }
        // Java:rightOperand.getClass().isArray()
        DataValue::Array(elements) => {
            for element in elements.borrow().iter() {
                if op_equals(&left_operand, element) {
                    return Ok(true);
                }
            }
            Ok(false)
        }
        // Java:rightOperand instanceof String → contains(String.valueOf(left))
        DataValue::Str(haystack) => Ok(haystack.contains(&left_operand.string_value_of())),
        // Java else 分支(含 Map):buildInvalidOperandTypeException
        _ => Err(build_invalid_operand_type_exception(
            left,
            right,
            operator,
            error_reporter,
        )),
    }
}

/// 对应 Java `BaseBinaryOperator.equals(Value, Value, ErrorReporter)`:
/// - 两侧都是数字 → `NumberMath.compareTo == 0`(跨数值类型提升比较,
///   `1 == 1L == 1.0`);
/// - 一侧数字一侧字符 → 字符转 int 后数值比较(`'A' == 65`);
/// - 同类型且 `Comparable` → `compareTo == 0`(String/Boolean 等,等价于
///   值相等;容器类型 Java 走 `Objects.equals`,即结构相等);
/// - 其余 → `Objects.equals`(引用/结构相等)。
///
/// [`DataValue` 的 `PartialEq`](crate::runtime::value::DataValue) 已实现
/// 数值跨类型提升与容器结构相等,此处仅补 Java 的「数字-字符」交叉分支。
fn op_equals(left: &DataValue, right: &DataValue) -> bool {
    if left == right {
        return true;
    }
    // Java:isNumberCharacter → char2Number 后 NumberMath.compareTo
    match (left, right) {
        (DataValue::Char(c), num) if num.is_number() => {
            number_compare(&DataValue::Int(*c as i32), num)
                == Some(std::cmp::Ordering::Equal)
        }
        (num, DataValue::Char(c)) if num.is_number() => {
            number_compare(num, &DataValue::Int(*c as i32))
                == Some(std::cmp::Ordering::Equal)
        }
        _ => false,
    }
}

/// 对应 Java `BaseBinaryOperator.buildInvalidOperandTypeException`:
/// 错误码 `INVALID_BINARY_OPERAND`,参数为操作符、左右类型名与值。
fn build_invalid_operand_type_exception(
    left: &QValue,
    right: &QValue,
    operator: &str,
    error_reporter: &dyn ErrorReporter,
) -> QLException {
    error_reporter.report_format(
        error_codes::INVALID_BINARY_OPERAND,
        error_codes::error_msg(error_codes::INVALID_BINARY_OPERAND),
        &[
            operator.to_string(),
            left.type_name().to_string(),
            left.get().string_value_of(),
            right.type_name().to_string(),
            right.get().string_value_of(),
        ],
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::exception::pure_err_reporter::PureErrReporter;
    use crate::runtime::data::index_map::IndexMap;

    // 操作符从不读写 QContext,测试直接驱动核心逻辑函数 `op_in`。
    fn run(left: DataValue, right: DataValue) -> Result<bool, QLException> {
        op_in(
            &QValue::from(left),
            &QValue::from(right),
            "in",
            &PureErrReporter::INSTANCE,
        )
    }

    #[test]
    fn in_list_uses_numeric_promotion_equality() {
        let list = DataValue::list(vec![DataValue::Int(1), DataValue::Int(2)]);
        // Java:equals 走 NumberMath.compareTo,跨类型数值相等
        assert!(run(DataValue::Long(1), list.clone()).unwrap());
        assert!(run(DataValue::Double(2.0), list.clone()).unwrap());
        assert!(!run(DataValue::Int(3), list).unwrap());
    }

    #[test]
    fn in_list_matches_char_against_number() {
        // Java:isNumberCharacter 分支,'A' == 65
        let list = DataValue::list(vec![DataValue::Int(65)]);
        assert!(run(DataValue::Char('A'), list).unwrap());
    }

    #[test]
    fn in_array() {
        let array = DataValue::array(vec![DataValue::Str("x".into())]);
        assert!(run(DataValue::Str("x".into()), array.clone()).unwrap());
        assert!(!run(DataValue::Str("y".into()), array).unwrap());
    }

    #[test]
    fn in_string_is_substring_of_string_value_of() {
        // Java:((String)right).contains(String.valueOf(left))
        assert!(run(DataValue::Str("bc".into()), DataValue::Str("abcd".into())).unwrap());
        // 左操作数非字符串时按 String.valueOf 渲染:1 in "a1b" → true
        assert!(run(DataValue::Int(1), DataValue::Str("a1b".into())).unwrap());
        assert!(!run(DataValue::Int(5), DataValue::Str("a1b".into())).unwrap());
    }

    #[test]
    fn in_null_semantics() {
        assert!(run(DataValue::Null, DataValue::Null).unwrap());
        assert!(!run(DataValue::Null, DataValue::list(vec![])).unwrap());
        assert!(!run(DataValue::Int(1), DataValue::Null).unwrap());
    }

    #[test]
    fn in_map_is_invalid_operand_not_key_nor_value_lookup() {
        // Java:Map 不是 Collection,落入 else → INVALID_BINARY_OPERAND
        let map = DataValue::map(IndexMap::from_entries(vec![(
            DataValue::Int(1),
            DataValue::Int(2),
        )]));
        let err = run(DataValue::Int(1), map).unwrap_err();
        assert_eq!(err.error_code(), error_codes::INVALID_BINARY_OPERAND);
    }
}
