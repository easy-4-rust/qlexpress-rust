//! Type-conversion rules, mirroring Java `runtime/data/convert/` plus the
//! numeric promotion matrix of `runtime/operator/number/NumberMath`
//! (promotion is centralized here per the Stage-0 task).

pub mod obj_type_convertor;
pub mod parameters_type_convertor;

pub use obj_type_convertor::{ObjTypeConvertor, QConverted, TargetType};
pub use parameters_type_convertor::ParametersTypeConvertor;

use std::cmp::Ordering;

use crate::runtime::value::DataValue;
use crate::utils::basic_util::NumKind;

/// The numeric kind of a value, or `None` for non-numbers.
pub fn num_kind(value: &DataValue) -> Option<NumKind> {
    match value {
        DataValue::Byte(_) => Some(NumKind::Byte),
        DataValue::Short(_) => Some(NumKind::Short),
        DataValue::Int(_) => Some(NumKind::Int),
        DataValue::Long(_) => Some(NumKind::Long),
        DataValue::Float(_) => Some(NumKind::Float),
        DataValue::Double(_) => Some(NumKind::Double),
        DataValue::BigInt(_) => Some(NumKind::BigInteger),
        DataValue::BigDec(_) => Some(NumKind::BigDecimal),
        _ => None,
    }
}

/// The math "domain" chosen for a binary numeric operation, mirroring
/// `NumberMath.getMath(left, right)`:
/// FloatingPoint > BigDecimal > BigInteger > Long > Integer.
/// Byte/Short compute in the Integer domain, as in Java.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum MathDomain {
    Integer,
    Long,
    BigInteger,
    BigDecimal,
    FloatingPoint,
}

fn domain_of(value: &DataValue) -> Option<MathDomain> {
    match value {
        DataValue::Byte(_) | DataValue::Short(_) | DataValue::Int(_) => Some(MathDomain::Integer),
        DataValue::Long(_) => Some(MathDomain::Long),
        DataValue::BigInt(_) => Some(MathDomain::BigInteger),
        DataValue::BigDec(_) => Some(MathDomain::BigDecimal),
        DataValue::Float(_) | DataValue::Double(_) => Some(MathDomain::FloatingPoint),
        _ => None,
    }
}

/// Java `NumberMath.getMath(left, right)` promotion matrix.
pub fn math_domain(left: &DataValue, right: &DataValue) -> Option<MathDomain> {
    let l = domain_of(left)?;
    let r = domain_of(right)?;
    Some(l.max(r))
}

/// Numeric comparison across types, mirroring
/// `NumberMath.compareTo(left, right)` semantics. Returns `None` when either
/// operand is not numeric.
pub fn number_compare(left: &DataValue, right: &DataValue) -> Option<Ordering> {
    match math_domain(left, right)? {
        MathDomain::FloatingPoint => {
            // Java FloatingPointMath.compareTo uses Double.compare semantics;
            // `total_cmp` matches it (NaN greatest, -0.0 < 0.0).
            Some(to_f64(left).total_cmp(&to_f64(right)))
        }
        MathDomain::BigDecimal => Some(big_dec_compare(
            &to_big_dec_string(left),
            &to_big_dec_string(right),
        )),
        MathDomain::BigInteger => Some(to_i128(left).cmp(&to_i128(right))),
        MathDomain::Long | MathDomain::Integer => Some(to_i64(left).cmp(&to_i64(right))),
    }
}

/// Promote both operands into the given domain (Java `NumberMath` operand
/// promotion).
pub fn promote(left: &DataValue, right: &DataValue, domain: MathDomain) -> (DataValue, DataValue) {
    let conv = |v: &DataValue| -> DataValue {
        match domain {
            MathDomain::FloatingPoint => DataValue::Double(to_f64(v)),
            MathDomain::BigDecimal => DataValue::BigDec(to_big_dec_string(v)),
            MathDomain::BigInteger => DataValue::BigInt(to_i128(v)),
            MathDomain::Long => DataValue::Long(to_i64(v)),
            MathDomain::Integer => DataValue::Int(to_i64(v) as i32),
        }
    };
    (conv(left), conv(right))
}

/// Java `Number.doubleValue()`.
pub fn to_f64(value: &DataValue) -> f64 {
    match value {
        DataValue::Byte(v) => *v as f64,
        DataValue::Short(v) => *v as f64,
        DataValue::Int(v) => *v as f64,
        DataValue::Long(v) => *v as f64,
        DataValue::Float(v) => *v as f64,
        DataValue::Double(v) => *v,
        DataValue::BigInt(v) => *v as f64,
        DataValue::BigDec(v) => v.parse::<f64>().unwrap_or(f64::NAN),
        _ => f64::NAN,
    }
}

/// Java `Number.longValue()` widened to `i128` (BigInteger domain).
pub fn to_i128(value: &DataValue) -> i128 {
    match value {
        DataValue::Byte(v) => *v as i128,
        DataValue::Short(v) => *v as i128,
        DataValue::Int(v) => *v as i128,
        DataValue::Long(v) => *v as i128,
        DataValue::Float(v) => *v as i128,
        DataValue::Double(v) => *v as i128,
        DataValue::BigInt(v) => *v,
        DataValue::BigDec(v) => big_dec_to_i128(v),
        _ => 0,
    }
}

/// Java `Number.longValue()`.
pub fn to_i64(value: &DataValue) -> i64 {
    match value {
        DataValue::Byte(v) => *v as i64,
        DataValue::Short(v) => *v as i64,
        DataValue::Int(v) => *v as i64,
        DataValue::Long(v) => *v,
        DataValue::Float(v) => *v as i64,
        DataValue::Double(v) => *v as i64,
        DataValue::BigInt(v) => *v as i64,
        DataValue::BigDec(v) => big_dec_to_i128(v) as i64,
        _ => 0,
    }
}

/// Java `NumberMath.toBigDecimal(n)` rendered as our string storage:
/// integral numbers keep their exact digits; binary floating point uses the
/// shortest round-trip representation (approximation of Java's exact
/// `new BigDecimal(double)` expansion — see stage notes).
pub fn to_big_dec_string(value: &DataValue) -> String {
    match value {
        DataValue::Byte(v) => v.to_string(),
        DataValue::Short(v) => v.to_string(),
        DataValue::Int(v) => v.to_string(),
        DataValue::Long(v) => v.to_string(),
        DataValue::BigInt(v) => v.to_string(),
        DataValue::Float(v) => v.to_string(),
        DataValue::Double(v) => v.to_string(),
        DataValue::BigDec(v) => v.clone(),
        _ => "0".to_string(),
    }
}

/// Java `BigDecimal.toBigInteger()`: truncate the fraction toward zero.
pub fn big_dec_to_i128(dec: &str) -> i128 {
    let (negative, int_part, _) = split_decimal(dec);
    let digits: String = int_part.trim_start_matches('0').to_string();
    let magnitude: i128 = if digits.is_empty() { 0 } else { digits.parse().unwrap_or(0) };
    if negative {
        -magnitude
    } else {
        magnitude
    }
}

/// Compare two decimal strings by numeric value (like
/// `BigDecimal.compareTo`, ignoring scale: `1.0 == 1.00`).
pub fn big_dec_compare(a: &str, b: &str) -> Ordering {
    let (neg_a, int_a, frac_a) = split_decimal(a);
    let (neg_b, int_b, frac_b) = split_decimal(b);

    let int_a = int_a.trim_start_matches('0');
    let int_b = int_b.trim_start_matches('0');
    let zero_a = int_a.is_empty() && frac_a.chars().all(|c| c == '0');
    let zero_b = int_b.is_empty() && frac_b.chars().all(|c| c == '0');
    if zero_a && zero_b {
        return Ordering::Equal;
    }
    if neg_a != neg_b {
        return if neg_a { Ordering::Less } else { Ordering::Greater };
    }
    let magnitude = compare_magnitude(int_a, &frac_a, int_b, &frac_b);
    if neg_a {
        magnitude.reverse()
    } else {
        magnitude
    }
}

fn compare_magnitude(int_a: &str, frac_a: &str, int_b: &str, frac_b: &str) -> Ordering {
    match int_a.len().cmp(&int_b.len()) {
        Ordering::Equal => {}
        other => return other,
    }
    match int_a.cmp(int_b) {
        Ordering::Equal => {}
        other => return other,
    }
    // Compare fractions digit by digit, padding the shorter with zeros.
    let max_len = frac_a.len().max(frac_b.len());
    let mut fa = frac_a.chars();
    let mut fb = frac_b.chars();
    for _ in 0..max_len {
        let da = fa.next().unwrap_or('0');
        let db = fb.next().unwrap_or('0');
        match da.cmp(&db) {
            Ordering::Equal => continue,
            other => return other,
        }
    }
    Ordering::Equal
}

/// Split a decimal string into (negative, integer digits, fraction digits).
/// Non-digit junk is treated as `0`, matching the forgiving behavior the
/// engine needs when parsing user-supplied `BigDec` payloads.
fn split_decimal(dec: &str) -> (bool, String, String) {
    let trimmed = dec.trim();
    let (negative, body) = match trimmed.strip_prefix('-') {
        Some(rest) => (true, rest),
        None => (false, trimmed.strip_prefix('+').unwrap_or(trimmed)),
    };
    let (int_part, frac_part) = match body.split_once('.') {
        Some((i, f)) => (i, f),
        None => (body, ""),
    };
    let clean = |s: &str| -> String { s.chars().filter(|c| c.is_ascii_digit()).collect() };
    (negative, clean(int_part), clean(frac_part))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn promotion_matrix_matches_number_math() {
        // FloatingPoint wins over everything (even BigDecimal).
        assert_eq!(
            math_domain(&DataValue::Float(1.0), &DataValue::BigDec("2".into())),
            Some(MathDomain::FloatingPoint)
        );
        assert_eq!(
            math_domain(&DataValue::BigDec("1".into()), &DataValue::BigInt(2)),
            Some(MathDomain::BigDecimal)
        );
        assert_eq!(
            math_domain(&DataValue::BigInt(1), &DataValue::Long(2)),
            Some(MathDomain::BigInteger)
        );
        assert_eq!(
            math_domain(&DataValue::Long(1), &DataValue::Int(2)),
            Some(MathDomain::Long)
        );
        // Byte/Short compute in the Integer domain.
        assert_eq!(
            math_domain(&DataValue::Byte(1), &DataValue::Short(2)),
            Some(MathDomain::Integer)
        );
        assert_eq!(math_domain(&DataValue::Bool(true), &DataValue::Int(1)), None);
    }

    #[test]
    fn compare_aligns_with_number_math_compare_to() {
        assert_eq!(
            number_compare(&DataValue::Int(1), &DataValue::Double(1.0)),
            Some(Ordering::Equal)
        );
        assert_eq!(
            number_compare(&DataValue::BigDec("1.0".into()), &DataValue::BigDec("1.00".into())),
            Some(Ordering::Equal)
        );
        assert_eq!(
            number_compare(&DataValue::Long(2), &DataValue::Int(10)),
            Some(Ordering::Less)
        );
        assert_eq!(
            number_compare(&DataValue::BigInt(i128::MAX), &DataValue::Long(i64::MAX)),
            Some(Ordering::Greater)
        );
        assert_eq!(
            number_compare(&DataValue::BigDec("-0.5".into()), &DataValue::BigDec("0.25".into())),
            Some(Ordering::Less)
        );
        assert_eq!(number_compare(&DataValue::Str("1".into()), &DataValue::Int(1)), None);
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
    }
}
