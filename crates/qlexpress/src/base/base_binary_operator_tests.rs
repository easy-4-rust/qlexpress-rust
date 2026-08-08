//! 从主对象文件机械搬移的聚焦单元测试；测试语义与来源标记保持不变。

use super::*;
use crate::exception::pure_err_reporter::PureErrReporter;

fn v(d: DataValue) -> QValue {
    QValue::Data(d)
}

fn opts() -> QLOptions {
    QLOptions::builder().build()
}

#[test]
fn plus_supports_string_concat_and_char_code() {
    // "a" + 1 = "a1";null 拼作 "null"。
    assert_eq!(
        BaseBinaryOperator::plus(
            "+",
            &v(DataValue::Str("a".into())),
            &v(DataValue::Int(1)),
            &opts(),
            &PureErrReporter::INSTANCE
        )
        .unwrap(),
        DataValue::string("a1")
    );
    assert_eq!(
        BaseBinaryOperator::plus(
            "+",
            &v(DataValue::Str("v=".into())),
            &v(DataValue::Null),
            &opts(),
            &PureErrReporter::INSTANCE
        )
        .unwrap(),
        DataValue::string("v=null")
    );
    // 'a' + 1 = 98(字符按码点转 int)。
    assert_eq!(
        BaseBinaryOperator::plus(
            "+",
            &v(DataValue::Char('a' as u16)),
            &v(DataValue::Int(1)),
            &opts(),
            &PureErrReporter::INSTANCE
        )
        .unwrap(),
        DataValue::Int(98)
    );
}

#[test]
fn equals_compares_across_numeric_types() {
    // Java 语义要点:== 跨数值类型(1 == 1L == 1.0 == "1.00"BigDecimal)。
    for (l, r) in [
        (DataValue::Int(1), DataValue::Long(1)),
        (DataValue::Int(1), DataValue::Double(1.0)),
        (DataValue::Long(1), DataValue::BigDec("1.00".into())),
        (DataValue::Char('a' as u16), DataValue::Int(97)),
    ] {
        assert!(
            BaseBinaryOperator::equals("==", &v(l), &v(r), &PureErrReporter::INSTANCE).unwrap()
        );
    }
    assert!(!BaseBinaryOperator::equals(
        "==",
        &v(DataValue::Int(1)),
        &v(DataValue::Double(1.5)),
        &PureErrReporter::INSTANCE
    )
    .unwrap());
    // 非 Comparable 走 Objects.equals:"a" == "a"。
    assert!(BaseBinaryOperator::equals(
        "==",
        &v(DataValue::Str("a".into())),
        &v(DataValue::Str("a".into())),
        &PureErrReporter::INSTANCE
    )
    .unwrap());
}

#[test]
fn boolean_bitwise_treats_null_as_false() {
    // Java 语义要点:true & null == false,null | true == true。
    assert_eq!(
        BaseBinaryOperator::bitwise_and(
            "&",
            &v(DataValue::Bool(true)),
            &v(DataValue::Null),
            &PureErrReporter::INSTANCE
        )
        .unwrap(),
        DataValue::Bool(false)
    );
    assert_eq!(
        BaseBinaryOperator::bitwise_or(
            "|",
            &v(DataValue::Null),
            &v(DataValue::Bool(true)),
            &PureErrReporter::INSTANCE
        )
        .unwrap(),
        DataValue::Bool(true)
    );
}

#[test]
fn java_protected_predicate_and_char_conversion_matrix() {
    let int_value = v(DataValue::Int(1));
    let long_value = v(DataValue::Long(1));
    let bool_value = v(DataValue::Bool(true));
    let null_value = v(DataValue::Null);
    let char_value = v(DataValue::Char('a' as u16));
    let string_value = v(DataValue::string("a"));
    let list_value = v(DataValue::list(Vec::new()));

    assert!(BaseBinaryOperator::is_same_type(
        &int_value,
        &v(DataValue::Int(2))
    ));
    assert!(!BaseBinaryOperator::is_same_type(&int_value, &long_value));
    assert!(BaseBinaryOperator::is_instanceof_comparable(&string_value));
    assert!(!BaseBinaryOperator::is_instanceof_comparable(&list_value));
    assert!(BaseBinaryOperator::is_both_boolean(
        &bool_value,
        &v(DataValue::Bool(false))
    ));
    assert!(BaseBinaryOperator::is_boolean_and_null(
        &bool_value,
        &null_value
    ));
    assert!(BaseBinaryOperator::is_both_number(&int_value, &long_value));
    assert!(BaseBinaryOperator::is_both_number_or_char(
        &char_value.get(),
        &int_value.get()
    ));
    assert!(BaseBinaryOperator::is_number_character(
        &char_value,
        &int_value
    ));
    assert!(BaseBinaryOperator::is_number(&int_value));
    assert_eq!(
        BaseBinaryOperator::char2number(&char_value.get()),
        DataValue::Int(97)
    );

    let assignment_error =
        BaseBinaryOperator::assert_left_value(&int_value, &PureErrReporter::INSTANCE).unwrap_err();
    assert_eq!(
        assignment_error.error_code(),
        error_codes::INVALID_ASSIGNMENT
    );
    assert_eq!(
        assignment_error.reason(),
        "value on the left side is not assignable"
    );
}

#[test]
fn divide_by_zero_reports_invalid_arithmetic() {
    let err = BaseBinaryOperator::divide(
        "/",
        &v(DataValue::Int(1)),
        &v(DataValue::Int(0)),
        &opts(),
        &PureErrReporter::INSTANCE,
    )
    .unwrap_err();
    assert_eq!(err.error_code(), error_codes::INVALID_ARITHMETIC);
    assert_eq!(err.reason(), "Division by zero");
}

#[test]
fn invalid_operand_message_aligned() {
    let err = BaseBinaryOperator::plus(
        "+",
        &v(DataValue::Bool(true)),
        &v(DataValue::Bool(false)),
        &opts(),
        &PureErrReporter::INSTANCE,
    )
    .unwrap_err();
    assert_eq!(err.error_code(), error_codes::INVALID_BINARY_OPERAND);
    assert!(err.reason().contains("the '+' operator can not be applied"));

    let list_err = BaseBinaryOperator::plus(
        "+",
        &v(DataValue::Bool(true)),
        &v(DataValue::list(Vec::new())),
        &opts(),
        &PureErrReporter::INSTANCE,
    )
    .unwrap_err();
    assert_eq!(
            list_err.reason(),
            "the '+' operator can not be applied to leftType:java.lang.Boolean with leftValue:true and rightType:java.util.ArrayList with rightValue:[]"
        );
}

#[test]
fn like_pattern_matching() {
    assert!(match_pattern(&"abc".into(), &"a%".into()));
    assert!(match_pattern(&"abc".into(), &"%b%".into()));
    assert!(!match_pattern(&"abc".into(), &"a%d".into()));
    assert!(match_pattern(&"abc".into(), &"abc".into()));
    assert!(match_pattern(&"abc".into(), &"%".into()));
}
