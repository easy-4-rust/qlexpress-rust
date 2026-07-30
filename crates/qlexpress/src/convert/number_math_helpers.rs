//! Rust 化新增辅助：数值域（MathDomain）+ 数值提升/比较/转换工具集。
//!
//! 对应 Java 语义（分散在多处）：
//! - `com.alibaba.qlexpress4.runtime.operator.number.NumberMath.getMath(left, right)`
//!   → [`math_domain`]（按域提升矩阵：FloatingPoint > BigDecimal > BigInteger > Long > Integer）
//! - `NumberMath.compareTo(left, right)` → [`number_compare`]
//! - `NumberMath.toBigDecimal(n)` → [`to_big_dec_string`]（字符串化存储）
//! - `BigDecimal.toBigInteger()`（trunc toward zero） → [`big_dec_to_big_int`]
//! - `BigDecimal.compareTo(...)`（忽略 scale） → [`big_dec_compare`]
//! - `Number.doubleValue()` / `Number.longValue()` → [`to_f64`] / [`to_i64`]
//!
//! 历史与定位：该模块原嵌于 `convert/mod.rs`，为遵守
//! "mod.rs 禁止定义类型/逻辑"（SPEC §2）规范迁出为独立文件。
//! Java 没有单独的对应类，因此标为 🆕 Rust 化新增。

use std::cmp::Ordering;

use num_bigint::BigInt;
use num_traits::ToPrimitive;

use crate::runtime::value::DataValue;
use crate::utils::basic_util::NumKind;

/// 数值种类判定，对应 Java `NumberMath` 内部对操作数种类的判定。
/// 非数值返回 `None`。
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

/// 二元数值运算所选择的"数域"，对应 Java
/// `NumberMath.getMath(left, right)`：FloatingPoint > BigDecimal > BigInteger > Long > Integer。
/// 与 Java 一致：Byte/Short 进入 Integer 域计算。
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum MathDomain {
    /// Java `IntegerMath` 域（含 Byte/Short）。
    Integer,
    /// Java `LongMath` 域。
    Long,
    /// Java `BigIntegerMath` 域。
    BigInteger,
    /// Java `BigDecimalMath` 域。
    BigDecimal,
    /// Java `FloatingPointMath` 域（Float/Double）。
    FloatingPoint,
}

/// 取单个值所属数域；非数值返回 `None`。
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

/// Java `NumberMath.getMath(left, right)` 提升矩阵：取两侧数域的较高者。
/// 对应 Java: 无（Rust 原生适配）。
pub fn math_domain(left: &DataValue, right: &DataValue) -> Option<MathDomain> {
    let l = domain_of(left)?;
    let r = domain_of(right)?;
    Some(l.max(r))
}

/// 跨类型数值比较，对应 Java `NumberMath.compareTo(left, right)` 语义。
/// 任一操作数非数值时返回 `None`。
pub fn number_compare(left: &DataValue, right: &DataValue) -> Option<Ordering> {
    match math_domain(left, right)? {
        MathDomain::FloatingPoint => {
            // Java FloatingPointMath.compareTo 使用 Double.compare 语义；
            // `total_cmp` 与其一致（NaN 最大，-0.0 < 0.0）。
            Some(to_f64(left).total_cmp(&to_f64(right)))
        }
        MathDomain::BigDecimal => Some(big_dec_compare(
            &to_big_dec_string(left),
            &to_big_dec_string(right),
        )),
        MathDomain::BigInteger => Some(to_big_int(left).cmp(&to_big_int(right))),
        MathDomain::Long | MathDomain::Integer => Some(to_i64(left).cmp(&to_i64(right))),
    }
}

/// 按指定数域提升两侧操作数（Java `NumberMath` 操作数提升）。
/// 对应 Java: 无（Rust 原生适配）。
pub fn promote(left: &DataValue, right: &DataValue, domain: MathDomain) -> (DataValue, DataValue) {
    let conv = |v: &DataValue| -> DataValue {
        match domain {
            MathDomain::FloatingPoint => DataValue::Double(to_f64(v)),
            MathDomain::BigDecimal => DataValue::BigDec(to_big_dec_string(v)),
            MathDomain::BigInteger => DataValue::BigInt(to_big_int(v)),
            MathDomain::Long => DataValue::Long(to_i64(v)),
            MathDomain::Integer => DataValue::Int(to_i64(v) as i32),
        }
    };
    (conv(left), conv(right))
}

/// 转换为 f64。
/// 参数：`value`；返回：`f64`。
/// 对应或承接 Java 源文件：`com/alibaba/qlexpress4/runtime/operator/number/NumberMath.java`，方法 `toF64`；Rust 侧按所有权与 `Result` 语义适配。
/// Java `Number.doubleValue()`。
/// 对应 Java: 无（Rust 原生适配）。
pub fn to_f64(value: &DataValue) -> f64 {
    match value {
        DataValue::Byte(v) => *v as f64,
        DataValue::Short(v) => *v as f64,
        DataValue::Int(v) => *v as f64,
        DataValue::Long(v) => *v as f64,
        DataValue::Float(v) => *v as f64,
        DataValue::Double(v) => *v,
        DataValue::BigInt(v) => v
            .to_f64()
            .unwrap_or_else(|| v.to_string().parse::<f64>().unwrap_or(f64::NAN)),
        DataValue::BigDec(v) => v.parse::<f64>().unwrap_or(f64::NAN),
        _ => f64::NAN,
    }
}

/// 转换为 `i128`。对任意精度整数保留低 128 位，等价于 Java
/// `BigInteger.longValue()` 的二进制补码截断规则扩展到 128 位。
/// 对应 Java: 无（Rust 原生适配）。
pub fn to_i128(value: &DataValue) -> i128 {
    match value {
        DataValue::Byte(v) => *v as i128,
        DataValue::Short(v) => *v as i128,
        DataValue::Int(v) => *v as i128,
        DataValue::Long(v) => *v as i128,
        DataValue::Float(v) => *v as i128,
        DataValue::Double(v) => *v as i128,
        DataValue::BigInt(v) => big_int_low_i128(v),
        DataValue::BigDec(v) => big_dec_to_i128(v),
        _ => 0,
    }
}

/// 转换为 i64。
/// 参数：`value`；返回：`i64`。
/// 对应或承接 Java 源文件：`com/alibaba/qlexpress4/runtime/operator/number/NumberMath.java`，方法 `toI64`；Rust 侧按所有权与 `Result` 语义适配。
/// Java `Number.longValue()`。
/// 对应 Java: 无（Rust 原生适配）。
pub fn to_i64(value: &DataValue) -> i64 {
    match value {
        DataValue::Byte(v) => *v as i64,
        DataValue::Short(v) => *v as i64,
        DataValue::Int(v) => *v as i64,
        DataValue::Long(v) => *v,
        DataValue::Float(v) => *v as i64,
        DataValue::Double(v) => *v as i64,
        DataValue::BigInt(v) => big_int_low_i64(v),
        DataValue::BigDec(v) => big_dec_to_i128(v) as i64,
        _ => 0,
    }
}

/// 转换到 Java `BigInteger` 数域。
/// 对应 Java: 无（Rust 原生适配）。
pub fn to_big_int(value: &DataValue) -> BigInt {
    match value {
        DataValue::Byte(v) => BigInt::from(*v),
        DataValue::Short(v) => BigInt::from(*v),
        DataValue::Int(v) => BigInt::from(*v),
        DataValue::Long(v) => BigInt::from(*v),
        DataValue::Float(v) => BigInt::from(*v as i128),
        DataValue::Double(v) => BigInt::from(*v as i128),
        DataValue::BigInt(v) => v.clone(),
        DataValue::BigDec(v) => big_dec_to_big_int(v),
        _ => BigInt::from(0),
    }
}

/// Java `NumberMath.toBigDecimal(n)` 在本项目的字符串存储形态：
/// 整数保留精确位数；二进制浮点使用最短可往返表示
/// （近似 Java `new BigDecimal(double)` 的精确展开，详见 stage notes）。
/// 对应 Java: 无（Rust 原生适配）。
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

/// Java `BigDecimal.toBigInteger()`：向零截断小数部分。
/// 对应 Java: 无（Rust 原生适配）。
pub fn big_dec_to_i128(dec: &str) -> i128 {
    big_int_low_i128(&big_dec_to_big_int(dec))
}

/// Java `BigDecimal.toBigInteger()`：向零截断小数部分并保持任意精度。
/// 对应 Java: 无（Rust 原生适配）。
pub fn big_dec_to_big_int(dec: &str) -> BigInt {
    let (negative, int_part, _) = split_decimal(dec);
    let digits = int_part.trim_start_matches('0');
    let magnitude = if digits.is_empty() {
        BigInt::from(0)
    } else {
        BigInt::parse_bytes(digits.as_bytes(), 10).unwrap_or_else(|| BigInt::from(0))
    };
    if negative {
        -magnitude
    } else {
        magnitude
    }
}

fn big_int_low_i128(value: &BigInt) -> i128 {
    let bytes = value.to_signed_bytes_le();
    let fill = if value.sign() == num_bigint::Sign::Minus {
        0xff
    } else {
        0
    };
    let mut low = [fill; 16];
    let copy_len = bytes.len().min(low.len());
    low[..copy_len].copy_from_slice(&bytes[..copy_len]);
    i128::from_le_bytes(low)
}

fn big_int_low_i64(value: &BigInt) -> i64 {
    let bytes = value.to_signed_bytes_le();
    let fill = if value.sign() == num_bigint::Sign::Minus {
        0xff
    } else {
        0
    };
    let mut low = [fill; 8];
    let copy_len = bytes.len().min(low.len());
    low[..copy_len].copy_from_slice(&bytes[..copy_len]);
    i64::from_le_bytes(low)
}

/// 按数值比较两个十进制字符串（与 `BigDecimal.compareTo` 一致，忽略 scale：
/// `1.0 == 1.00`）。
/// 对应 Java: 无（Rust 原生适配）。
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
        return if neg_a {
            Ordering::Less
        } else {
            Ordering::Greater
        };
    }
    let magnitude = compare_magnitude(int_a, &frac_a, int_b, &frac_b);
    if neg_a {
        magnitude.reverse()
    } else {
        magnitude
    }
}

/// 比较两个十进制 magnitude（同号情况）：先比整数位数/字典序，再逐位比小数。
fn compare_magnitude(int_a: &str, frac_a: &str, int_b: &str, frac_b: &str) -> Ordering {
    match int_a.len().cmp(&int_b.len()) {
        Ordering::Equal => {}
        other => return other,
    }
    match int_a.cmp(int_b) {
        Ordering::Equal => {}
        other => return other,
    }
    // 逐位比较小数部分，较短者补零。
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

/// 将十进制字符串拆分为 `(负数标记, 整数位数字, 小数位数字)`。
/// 非数字字符视为 `0`，与引擎解析用户 `BigDec` 输入时的宽容行为一致。
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
