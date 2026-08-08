//! `BigDecimalMath` 的 Java 语义单元测试。

use super::*;

fn div(l: &str, r: &str) -> DataValue {
    BigDecimalMath::divide_impl(
        &DataValue::BigDec(l.to_string()),
        &DataValue::BigDec(r.to_string()),
    )
    .unwrap()
}

#[test]
fn exact_division_keeps_big_decimal_scale() {
    assert_eq!(div("7", "2"), DataValue::BigDec("3.5".to_string()));
    assert_eq!(div("2.00", "2"), DataValue::BigDec("1.00".to_string()));
    assert_eq!(div("1", "8"), DataValue::BigDec("0.125".to_string()));
}

#[test]
fn non_terminating_division_uses_default_precision_and_half_up() {
    assert_eq!(div("1", "3"), DataValue::BigDec("0.3333333333".to_string()));
    assert_eq!(div("2", "3"), DataValue::BigDec("0.6666666667".to_string()));
    assert_eq!(
        div("10", "3"),
        DataValue::BigDec("3.3333333333".to_string())
    );
}

#[test]
fn division_by_zero_reports() {
    let err = BigDecimalMath::divide_impl(
        &DataValue::BigDec("1".to_string()),
        &DataValue::BigDec("0".to_string()),
    )
    .unwrap_err();
    assert_eq!(err.reason(), "Division by zero");
}

#[test]
fn add_sub_multiply_exact() {
    assert_eq!(
        BigDecimalMath::add_impl(
            &DataValue::BigDec("1.0".into()),
            &DataValue::BigDec("1.00".into())
        )
        .unwrap(),
        DataValue::BigDec("2.00".into())
    );
    assert_eq!(
        BigDecimalMath::subtract_impl(
            &DataValue::BigDec("0.3".into()),
            &DataValue::BigDec("0.1".into())
        )
        .unwrap(),
        DataValue::BigDec("0.2".into())
    );
    assert_eq!(
        BigDecimalMath::multiply_impl(
            &DataValue::BigDec("1.5".into()),
            &DataValue::BigDec("0.02".into())
        )
        .unwrap(),
        DataValue::BigDec("0.030".into())
    );
    assert_eq!(
        BigDecimalMath::add_impl(
            &DataValue::BigDec("-1.5".into()),
            &DataValue::BigDec("0.5".into())
        )
        .unwrap(),
        DataValue::BigDec("-1.0".into())
    );
}

#[test]
fn remainder_and_mod_sign_rules() {
    assert_eq!(
        BigDecimalMath::remainder_impl(
            &DataValue::BigDec("-7".into()),
            &DataValue::BigDec("3".into())
        )
        .unwrap(),
        DataValue::BigDec("-1".into())
    );
    assert_eq!(
        BigDecimalMath::mod_impl(
            &DataValue::BigDec("-7".into()),
            &DataValue::BigDec("3".into())
        )
        .unwrap(),
        DataValue::BigDec("2".into())
    );
    assert_eq!(
        BigDecimalMath::mod_impl(
            &DataValue::BigDec("-7".into()),
            &DataValue::BigDec("-3".into())
        )
        .unwrap(),
        DataValue::BigDec("-4".into())
    );
}

#[test]
fn integer_operands_also_flow_through() {
    assert_eq!(
        BigDecimalMath::divide_impl(&DataValue::Int(1), &DataValue::Long(4)).unwrap(),
        DataValue::BigDec("0.25".into())
    );
}

#[test]
fn exponent_input_and_java_to_string_preserve_signed_scale() {
    for (source, expected, scale, precision) in [
        ("1.0E20", "1.0E+20", -19, 2),
        ("1E+2", "1E+2", -2, 1),
        ("0E+7", "0E+7", -7, 1),
        ("0E-7", "0E-7", 7, 1),
        ("0.000001", "0.000001", 6, 1),
        ("0.0000001", "1E-7", 7, 1),
    ] {
        let decimal = parse_dec(source);
        assert_eq!(decimal.to_java_string(), expected, "source={source}");
        assert_eq!(decimal.scale, scale, "source={source}");
        assert_eq!(decimal.precision(), precision, "source={source}");
    }
}

#[test]
fn arithmetic_with_negative_scale_matches_jdk_oracle() {
    assert_eq!(
        BigDecimalMath::add_impl(
            &DataValue::BigDec("1.0E+2".into()),
            &DataValue::BigDec("1".into())
        )
        .unwrap(),
        DataValue::BigDec("101".into())
    );
    assert_eq!(
        BigDecimalMath::multiply_impl(
            &DataValue::BigDec("1.0E+2".into()),
            &DataValue::BigDec("2.00".into())
        )
        .unwrap(),
        DataValue::BigDec("200.0".into())
    );
    assert_eq!(div("1E+2", "2"), DataValue::BigDec("5E+1".into()));
    assert_eq!(div("1.0E+2", "2"), DataValue::BigDec("5E+1".into()));
    assert_eq!(div("100", "0.2"), DataValue::BigDec("5.0E+2".into()));
    assert_eq!(
        div("1E+20", "3"),
        DataValue::BigDec("3.3333333333E+19".into())
    );
    assert_eq!(
        BigDecimalMath::remainder_impl(
            &DataValue::BigDec("5.00E+2".into()),
            &DataValue::BigDec("3E+1".into())
        )
        .unwrap(),
        DataValue::BigDec("20".into())
    );
}
