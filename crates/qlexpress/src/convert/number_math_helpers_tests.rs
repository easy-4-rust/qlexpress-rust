//! 从主对象文件机械搬移的聚焦单元测试；测试语义与来源标记保持不变。

use super::*;

#[test]
fn promotion_matrix_matches_number_math() {
    // FloatingPoint 优先级最高（甚至高于 BigDecimal）。
    assert_eq!(
        math_domain(&DataValue::Float(1.0), &DataValue::BigDec("2".into())),
        Some(MathDomain::FloatingPoint)
    );
    assert_eq!(
        math_domain(&DataValue::BigDec("1".into()), &DataValue::big_int(2)),
        Some(MathDomain::BigDecimal)
    );
    assert_eq!(
        math_domain(&DataValue::big_int(1), &DataValue::Long(2)),
        Some(MathDomain::BigInteger)
    );
    assert_eq!(
        math_domain(&DataValue::Long(1), &DataValue::Int(2)),
        Some(MathDomain::Long)
    );
    // Byte/Short 进入 Integer 域。
    assert_eq!(
        math_domain(&DataValue::Byte(1), &DataValue::Short(2)),
        Some(MathDomain::Integer)
    );
    assert_eq!(
        math_domain(&DataValue::Bool(true), &DataValue::Int(1)),
        None
    );
}

#[test]
fn compare_aligns_with_number_math_compare_to() {
    assert_eq!(
        number_compare(&DataValue::Int(1), &DataValue::Double(1.0)),
        Some(Ordering::Equal)
    );
    assert_eq!(
        number_compare(
            &DataValue::BigDec("1.0".into()),
            &DataValue::BigDec("1.00".into())
        ),
        Some(Ordering::Equal)
    );
    assert_eq!(
        number_compare(&DataValue::Long(2), &DataValue::Int(10)),
        Some(Ordering::Less)
    );
    assert_eq!(
        number_compare(&DataValue::big_int(i128::MAX), &DataValue::Long(i64::MAX)),
        Some(Ordering::Greater)
    );
    assert_eq!(
        number_compare(
            &DataValue::BigDec("-0.5".into()),
            &DataValue::BigDec("0.25".into())
        ),
        Some(Ordering::Less)
    );
    assert_eq!(
        number_compare(&DataValue::Str("1".into()), &DataValue::Int(1)),
        None
    );
    assert_eq!(
        number_compare(&DataValue::Double(-0.0), &DataValue::Double(0.0)),
        Some(Ordering::Less)
    );
    let negative_nan = f64::from_bits(0xfff8_0000_0000_0042);
    assert_eq!(
        number_compare(
            &DataValue::Double(negative_nan),
            &DataValue::Double(f64::NAN)
        ),
        Some(Ordering::Equal)
    );
    assert_eq!(
        number_compare(
            &DataValue::Double(negative_nan),
            &DataValue::Double(f64::MAX)
        ),
        Some(Ordering::Greater)
    );
}

#[test]
fn promote_converts_operands_to_common_domain() {
    let (l, r) = promote(
        &DataValue::Int(3),
        &DataValue::Double(0.5),
        MathDomain::FloatingPoint,
    );
    assert_eq!(l, DataValue::Double(3.0));
    assert_eq!(r, DataValue::Double(0.5));

    let (l, r) = promote(&DataValue::Byte(2), &DataValue::Long(7), MathDomain::Long);
    assert_eq!(l, DataValue::Long(2));
    assert_eq!(r, DataValue::Long(7));
}

#[test]
fn big_dec_truncates_toward_zero_like_java() {
    assert_eq!(big_dec_to_i128("12.99"), 12);
    assert_eq!(big_dec_to_i128("-12.99"), -12);
    assert_eq!(big_dec_to_i128("0.5"), 0);
    assert_eq!(big_dec_to_i128("1.23E+4"), 12_300);
    assert_eq!(big_dec_to_i128("-1.23E+4"), -12_300);
    assert_eq!(big_dec_to_i128("1.23E-4"), 0);
}

/// SOURCE_PARITY: Java `BigDecimal.compareTo` 按数值比较指数文本并忽略
/// scale，而不是把 `E` 后的数字拼进 coefficient。
#[test]
fn big_decimal_compare_supports_exponents_and_negative_scale() {
    assert_eq!(big_dec_compare("1E+2", "100.00"), Ordering::Equal);
    assert_eq!(big_dec_compare("9.99E+19", "1E+20"), Ordering::Less);
    assert_eq!(big_dec_compare("-1E-7", "-0.00000009"), Ordering::Less);
    assert_eq!(big_dec_compare("0E+20", "0.000"), Ordering::Equal);
}

/// SOURCE_PARITY: Java `BigDecimal.toString()` 的 adjusted exponent
/// 边界与正指数加号。
#[test]
fn java_big_decimal_string_matches_jdk_oracle() {
    let cases = [
        ("1.0E20", "1.0E+20"),
        ("1E+2", "1E+2"),
        ("0E+7", "0E+7"),
        ("0E-7", "0E-7"),
        ("0.000001", "0.000001"),
        ("0.0000001", "1E-7"),
        ("100.00", "100.00"),
    ];
    for (value, expected) in cases {
        assert_eq!(java_big_decimal_to_string(value), expected);
    }
}

/// SOURCE_PARITY: Java `Double.toString(double)` 的普通/科学记数法边界、
/// 特殊值和 IEEE-754 极值文本。
#[test]
fn java_double_to_string_matches_jdk_oracle_matrix() {
    let cases = [
        (1.0, "1.0"),
        (1_000_000.0, "1000000.0"),
        (10_000_000.0, "1.0E7"),
        (0.001, "0.001"),
        (0.0001, "1.0E-4"),
        (1.2345678901234567, "1.2345678901234567"),
        (1.0E20, "1.0E20"),
        (-0.0, "-0.0"),
        (f64::from_bits(1), "4.9E-324"),
        (f64::MIN_POSITIVE, "2.2250738585072014E-308"),
        (f64::MAX, "1.7976931348623157E308"),
        (f64::NAN, "NaN"),
        (f64::INFINITY, "Infinity"),
        (f64::NEG_INFINITY, "-Infinity"),
    ];
    for (value, expected) in cases {
        assert_eq!(java_f64_to_string(value), expected, "value={value:?}");
    }
}

/// SOURCE_PARITY: Java `Float.toString(float)` 必须按单精度值选择最短
/// 文本，不能先扩展为 f64 后套用 Rust `Display`。
#[test]
fn java_float_to_string_matches_jdk_oracle_matrix() {
    let cases = [
        (1.0_f32, "1.0"),
        (1_000_000.0_f32, "1000000.0"),
        (10_000_000.0_f32, "1.0E7"),
        (0.001_f32, "0.001"),
        (0.0001_f32, "1.0E-4"),
        (1.2345678_f32, "1.2345678"),
        (-0.0_f32, "-0.0"),
        (f32::from_bits(1), "1.4E-45"),
        (f32::MIN_POSITIVE, "1.1754944E-38"),
        (f32::MAX, "3.4028235E38"),
        (f32::NAN, "NaN"),
        (f32::INFINITY, "Infinity"),
        (f32::NEG_INFINITY, "-Infinity"),
    ];
    for (value, expected) in cases {
        assert_eq!(java_f32_to_string(value), expected, "value={value:?}");
    }
}
