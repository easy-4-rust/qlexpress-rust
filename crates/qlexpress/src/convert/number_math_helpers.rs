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
use num_traits::{Signed, ToPrimitive, Zero};

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
            // 所有 NaN payload/sign 必须先 canonicalize，不能直接用
            // Rust `total_cmp`（它会区分负 NaN 与 payload）。
            Some(java_double_compare(to_f64(left), to_f64(right)))
        }
        MathDomain::BigDecimal => Some(big_dec_compare(
            &to_big_dec_string(left),
            &to_big_dec_string(right),
        )),
        MathDomain::BigInteger => Some(to_big_int(left).cmp(&to_big_int(right))),
        MathDomain::Long | MathDomain::Integer => Some(to_i64(left).cmp(&to_i64(right))),
    }
}

/// Java `Double.compare(double, double)`：普通数值先比较，随后使用
/// `doubleToLongBits`（canonical NaN）区分 `-0.0`、`+0.0` 与 NaN。
fn java_double_compare(left: f64, right: f64) -> Ordering {
    if left < right {
        return Ordering::Less;
    }
    if left > right {
        return Ordering::Greater;
    }
    let left_bits = canonical_double_bits(left) as i64;
    let right_bits = canonical_double_bits(right) as i64;
    left_bits.cmp(&right_bits)
}

fn canonical_double_bits(value: f64) -> u64 {
    if value.is_nan() {
        0x7ff8_0000_0000_0000
    } else {
        value.to_bits()
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

/// 转换为 Java `Number.intValue()` 语义的 `i32`。
///
/// Java `Double.intValue()`/`Float.intValue()` 在数值超过 `Integer` 范围时
/// 饱和到最大/最小值；不能先按 `longValue()` 转换再截断，否则会把高位溢出
/// 错误地折返到低 32 位。整数和任意精度数值仍保留 Java 窄化转换的低位规则。
/// 对应 Java：`java.lang.Number#intValue`。
pub fn to_i32(value: &DataValue) -> i32 {
    match value {
        DataValue::Float(v) => *v as i32,
        DataValue::Double(v) => *v as i32,
        _ => to_i64(value) as i32,
    }
}

/// 转换到 Java `BigInteger` 数域。
/// 对应 Java: 无（Rust 原生适配）。
pub fn to_big_int(value: &DataValue) -> BigInt {
    try_to_big_int(value).unwrap_or_else(BigInt::zero)
}

/// Java `NumberMath.toBigInteger(Number)` 的可失败版本。
///
/// 有限 Float/Double 必须先经 Java `toString()` 再构造 BigDecimal 并向零
/// 截断，因而不受 i128 范围限制；NaN/Infinity 对应 Java
/// `NumberFormatException`，返回 `None` 交由调用层映射异常/不可转换。
pub fn try_to_big_int(value: &DataValue) -> Option<BigInt> {
    match value {
        DataValue::Byte(v) => Some(BigInt::from(*v)),
        DataValue::Short(v) => Some(BigInt::from(*v)),
        DataValue::Int(v) => Some(BigInt::from(*v)),
        DataValue::Long(v) => Some(BigInt::from(*v)),
        DataValue::Float(v) if v.is_finite() => Some(big_dec_to_big_int(&java_f32_to_string(*v))),
        DataValue::Double(v) if v.is_finite() => Some(big_dec_to_big_int(&java_f64_to_string(*v))),
        DataValue::BigInt(v) => Some(v.clone()),
        DataValue::BigDec(v) => Some(big_dec_to_big_int(v)),
        _ => None,
    }
}

/// Java `NumberMath.toBigDecimal(n)` 在本项目的字符串存储形态：
/// 整数保留精确位数；二进制浮点先按 `Float/Double.toString()` 生成 Java
/// 规范文本，再对应 `new BigDecimal(n.toString())` 构造十进制值。
/// 对应 Java: 无（Rust 原生适配）。
pub fn to_big_dec_string(value: &DataValue) -> String {
    match value {
        DataValue::Byte(v) => v.to_string(),
        DataValue::Short(v) => v.to_string(),
        DataValue::Int(v) => v.to_string(),
        DataValue::Long(v) => v.to_string(),
        DataValue::BigInt(v) => v.to_string(),
        DataValue::Float(v) => java_f32_to_string(*v),
        DataValue::Double(v) => java_f64_to_string(*v),
        DataValue::BigDec(v) => v.clone(),
        _ => "0".to_string(),
    }
}

/// Java `Float.toString(float)` 的最短可往返文本和指数展示阈值。
///
/// 参数：`value` 为待格式化的 IEEE-754 单精度值。
/// 返回：与 Java `Float.toString(float)` 一致的文本。
/// 对应 Java：`java.lang.Float#toString(float)`。
pub fn java_f32_to_string(value: f32) -> String {
    if let Some(special) = java_float_special_string(value as f64) {
        return special;
    }

    let bits = value.to_bits();
    let negative = bits >> 31 != 0;
    let magnitude_bits = bits & 0x7fff_ffff;
    let exponent_bits = (magnitude_bits >> 23) & 0xff;
    let fraction = magnitude_bits & 0x7f_ffff;
    let (significand, binary_exponent) = if exponent_bits == 0 {
        (u64::from(fraction), -149)
    } else {
        (
            u64::from((1_u32 << 23) | fraction),
            exponent_bits as i32 - 127 - 23,
        )
    };

    java_finite_float_to_string(
        negative,
        significand,
        binary_exponent,
        (value.abs() as f64).log10().floor() as i32,
        9,
        |candidate| {
            candidate
                .parse::<f32>()
                .is_ok_and(|parsed| parsed.to_bits() == magnitude_bits)
        },
    )
}

/// Java `Double.toString(double)` 的最短可往返文本和指数展示阈值。
///
/// 参数：`value` 为待格式化的 IEEE-754 双精度值。
/// 返回：与 Java `Double.toString(double)` 一致的文本。
/// 对应 Java：`java.lang.Double#toString(double)`。
pub fn java_f64_to_string(value: f64) -> String {
    if let Some(special) = java_float_special_string(value) {
        return special;
    }

    let bits = value.to_bits();
    let negative = bits >> 63 != 0;
    let magnitude_bits = bits & 0x7fff_ffff_ffff_ffff;
    let exponent_bits = (magnitude_bits >> 52) & 0x7ff;
    let fraction = magnitude_bits & 0x000f_ffff_ffff_ffff;
    let (significand, binary_exponent) = if exponent_bits == 0 {
        (fraction, -1074)
    } else {
        ((1_u64 << 52) | fraction, exponent_bits as i32 - 1023 - 52)
    };

    java_finite_float_to_string(
        negative,
        significand,
        binary_exponent,
        value.abs().log10().floor() as i32,
        17,
        |candidate| {
            candidate
                .parse::<f64>()
                .is_ok_and(|parsed| parsed.to_bits() == magnitude_bits)
        },
    )
}

/// 处理 Java 对零、无穷与 NaN 的固定拼写；有限非零值返回 `None`。
fn java_float_special_string(value: f64) -> Option<String> {
    if value.is_nan() {
        return Some("NaN".to_string());
    }
    if value == f64::INFINITY {
        return Some("Infinity".to_string());
    }
    if value == f64::NEG_INFINITY {
        return Some("-Infinity".to_string());
    }
    if value == 0.0 {
        return Some(if value.is_sign_negative() {
            "-0.0".to_string()
        } else {
            "0.0".to_string()
        });
    }
    None
}

/// 在精确二进制有理数上搜索 Java 规定的最短十进制。
///
/// Java 的选择规则不是简单复用 Rust `Display`：先选择能往返为原浮点值的
/// 最少有效位，再在同位数候选中选择与精确值距离最近者，距离相同取偶数。
/// 科学记数法至少保留两位有效数字，这也是 `Double.MIN_VALUE` 必须输出
/// `4.9E-324` 而不是 Rust `Display` 的 `5e-324` 的原因。
fn java_finite_float_to_string(
    negative: bool,
    significand: u64,
    binary_exponent: i32,
    approximate_decimal_exponent: i32,
    max_significant_digits: usize,
    mut round_trips: impl FnMut(&str) -> bool,
) -> String {
    let (numerator, denominator) = exact_binary_rational(significand, binary_exponent);
    let decimal_exponent =
        exact_decimal_exponent(&numerator, &denominator, approximate_decimal_exponent);
    let min_significant_digits = if (-3..7).contains(&decimal_exponent) {
        1
    } else {
        2
    };

    for significant_digits in min_significant_digits..=max_significant_digits {
        let decimal_power = decimal_exponent - significant_digits as i32 + 1;
        let (scaled_numerator, scaled_denominator) =
            scale_for_decimal_digits(&numerator, &denominator, decimal_power);
        let floor = &scaled_numerator / &scaled_denominator;

        let mut best: Option<(BigInt, BigInt)> = None;
        // 最近候选必在 floor/ceil；额外检查相邻整数覆盖十进制 decade 边界。
        for offset in -1_i32..=2 {
            let candidate = &floor + BigInt::from(offset);
            if candidate <= BigInt::zero() {
                continue;
            }
            let parse_text = format!("{candidate}e{decimal_power}");
            if !round_trips(&parse_text) {
                continue;
            }

            let distance = (&candidate * &scaled_denominator - &scaled_numerator).abs();
            let replace = match &best {
                None => true,
                Some((best_candidate, best_distance)) => match distance.cmp(best_distance) {
                    Ordering::Less => true,
                    Ordering::Greater => false,
                    Ordering::Equal => is_even(&candidate) && !is_even(best_candidate),
                },
            };
            if replace {
                best = Some((candidate, distance));
            }
        }

        if let Some((candidate, _)) = best {
            // decade 边界附近的二进制值可能让精确 exponent 落在前一 decade，
            // 例如 f32 0.0001 的候选为 100e-6。Java 输出最短等价值
            // 1.0E-4，因此在渲染前移除候选整数的十进制尾零并同步指数。
            let mut digits = candidate.to_string();
            let mut normalized_power = decimal_power;
            while digits.len() > 1 && digits.ends_with('0') {
                digits.pop();
                normalized_power += 1;
            }
            return format_java_decimal(negative, &digits, normalized_power);
        }
    }

    // IEEE-754 f32/f64 分别最多需要 9/17 位即可唯一往返；到达这里表示内部
    // 算法不变量被破坏，静默退化会重新引入跨语言语义偏差。
    unreachable!("a finite IEEE-754 value must round-trip within its maximum decimal precision")
}

/// 将 `significand * 2^binary_exponent` 化为精确的正有理数。
fn exact_binary_rational(significand: u64, binary_exponent: i32) -> (BigInt, BigInt) {
    if binary_exponent >= 0 {
        (
            BigInt::from(significand) << binary_exponent as usize,
            BigInt::from(1_u8),
        )
    } else {
        (
            BigInt::from(significand),
            BigInt::from(1_u8) << (-binary_exponent) as usize,
        )
    }
}

/// 精确修正 `log10` 给出的 decade，保证 `10^k <= value < 10^(k+1)`。
fn exact_decimal_exponent(numerator: &BigInt, denominator: &BigInt, mut exponent: i32) -> i32 {
    while compare_rational_to_power_of_ten(numerator, denominator, exponent) == Ordering::Less {
        exponent -= 1;
    }
    while compare_rational_to_power_of_ten(numerator, denominator, exponent + 1) != Ordering::Less {
        exponent += 1;
    }
    exponent
}

/// 比较正有理数 `numerator / denominator` 与 `10^exponent`。
fn compare_rational_to_power_of_ten(
    numerator: &BigInt,
    denominator: &BigInt,
    exponent: i32,
) -> Ordering {
    if exponent >= 0 {
        numerator.cmp(&(denominator * pow10(exponent as usize)))
    } else {
        (numerator * pow10((-exponent) as usize)).cmp(denominator)
    }
}

/// 把精确值缩放到指定十进制有效位整数所在的有理数。
fn scale_for_decimal_digits(
    numerator: &BigInt,
    denominator: &BigInt,
    decimal_power: i32,
) -> (BigInt, BigInt) {
    if decimal_power >= 0 {
        (
            numerator.clone(),
            denominator * pow10(decimal_power as usize),
        )
    } else {
        (
            numerator * pow10((-decimal_power) as usize),
            denominator.clone(),
        )
    }
}

fn pow10(exponent: usize) -> BigInt {
    BigInt::from(10_u8).pow(exponent as u32)
}

fn is_even(value: &BigInt) -> bool {
    (value % BigInt::from(2_u8)).is_zero()
}

/// 根据 Java 的 `10^-3 <= m < 10^7` 普通记数法边界输出候选。
fn format_java_decimal(negative: bool, digits: &str, decimal_power: i32) -> String {
    let sign = if negative { "-" } else { "" };
    let exponent = digits.len() as i32 - 1 + decimal_power;
    if !(-3..7).contains(&exponent) {
        let tail = if digits.len() == 1 { "0" } else { &digits[1..] };
        return format!("{sign}{}.{}E{exponent}", &digits[..1], tail);
    }

    let decimal_position = digits.len() as i32 + decimal_power;
    if decimal_position <= 0 {
        return format!(
            "{sign}0.{}{}",
            "0".repeat((-decimal_position) as usize),
            digits
        );
    }
    let decimal_position = decimal_position as usize;
    if decimal_position >= digits.len() {
        return format!(
            "{sign}{}{}.0",
            digits,
            "0".repeat(decimal_position - digits.len())
        );
    }
    format!(
        "{sign}{}.{}",
        &digits[..decimal_position],
        &digits[decimal_position..]
    )
}

/// Java `BigDecimal.toBigInteger()`：向零截断小数部分。
/// 对应 Java: 无（Rust 原生适配）。
pub fn big_dec_to_i128(dec: &str) -> i128 {
    big_int_low_i128(&big_dec_to_big_int(dec))
}

/// Java `BigDecimal.toBigInteger()`：向零截断小数部分并保持任意精度。
/// 对应 Java: 无（Rust 原生适配）。
pub fn big_dec_to_big_int(dec: &str) -> BigInt {
    let (negative, mut digits, scale) = parse_decimal_parts(dec);
    if digits.is_empty() {
        return BigInt::zero();
    }
    if scale > 0 {
        let fractional_digits = scale as usize;
        if fractional_digits >= digits.len() {
            return BigInt::zero();
        }
        digits.truncate(digits.len() - fractional_digits);
    } else if scale < 0 {
        digits.extend(std::iter::repeat_n('0', (-scale) as usize));
    }
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
    let (neg_a, digits_a, scale_a) = parse_decimal_parts(a);
    let (neg_b, digits_b, scale_b) = parse_decimal_parts(b);
    let zero_a = digits_a.is_empty();
    let zero_b = digits_b.is_empty();
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
    let magnitude = compare_decimal_magnitude(&digits_a, scale_a, &digits_b, scale_b);
    if neg_a {
        magnitude.reverse()
    } else {
        magnitude
    }
}

/// Java `BigDecimal.toString()` 的规范文本。
///
/// 参数：`decimal` 为等价于 Java `BigDecimal` 的十进制/指数文本。
/// 返回：保留 unscaled value 与 signed scale 的 Java 规范表示。
/// 对应 Java：`java.math.BigDecimal#toString()`。
pub fn java_big_decimal_to_string(decimal: &str) -> String {
    let (negative, digits, scale) = parse_decimal_parts(decimal);
    let digits = if digits.is_empty() {
        "0"
    } else {
        digits.as_str()
    };
    let precision = digits.len() as i64;
    let adjusted_exponent = -scale + precision - 1;
    let sign = if negative { "-" } else { "" };

    if scale >= 0 && adjusted_exponent >= -6 {
        if scale == 0 {
            return format!("{sign}{digits}");
        }
        if precision > scale {
            let decimal_position = (precision - scale) as usize;
            return format!(
                "{sign}{}.{}",
                &digits[..decimal_position],
                &digits[decimal_position..]
            );
        }
        return format!(
            "{sign}0.{}{}",
            "0".repeat((scale - precision) as usize),
            digits
        );
    }

    let coefficient = if digits.len() == 1 {
        digits.to_string()
    } else {
        format!("{}.{}", &digits[..1], &digits[1..])
    };
    let exponent_sign = if adjusted_exponent > 0 { "+" } else { "" };
    format!("{sign}{coefficient}E{exponent_sign}{adjusted_exponent}")
}

/// 比较两个规范十进制 magnitude：先比较 adjusted exponent，再逐位补零比较。
fn compare_decimal_magnitude(
    digits_a: &str,
    scale_a: i64,
    digits_b: &str,
    scale_b: i64,
) -> Ordering {
    let exponent_a = digits_a.len() as i64 - 1 - scale_a;
    let exponent_b = digits_b.len() as i64 - 1 - scale_b;
    match exponent_a.cmp(&exponent_b) {
        Ordering::Equal => {}
        other => return other,
    }
    let max_len = digits_a.len().max(digits_b.len());
    let mut a = digits_a.bytes();
    let mut b = digits_b.bytes();
    for _ in 0..max_len {
        let da = a.next().unwrap_or(b'0');
        let db = b.next().unwrap_or(b'0');
        match da.cmp(&db) {
            Ordering::Equal => continue,
            other => return other,
        }
    }
    Ordering::Equal
}

/// 将 Java `BigDecimal` 文本拆为 `(负数标记, 规范 unscaled digits, signed scale)`。
fn parse_decimal_parts(dec: &str) -> (bool, String, i64) {
    let trimmed = dec.trim();
    let (negative, body) = match trimmed.strip_prefix('-') {
        Some(rest) => (true, rest),
        None => (false, trimmed.strip_prefix('+').unwrap_or(trimmed)),
    };
    let exponent_index = body.find(['e', 'E']);
    let (coefficient, exponent) = match exponent_index {
        Some(index) => (
            &body[..index],
            body[index + 1..].parse::<i64>().unwrap_or(0),
        ),
        None => (body, 0),
    };
    let (int_part, frac_part) = match coefficient.split_once('.') {
        Some((i, f)) => (i, f),
        None => (coefficient, ""),
    };
    let mut digits: String = int_part
        .chars()
        .chain(frac_part.chars())
        .filter(|c| c.is_ascii_digit())
        .collect();
    let first_non_zero = digits
        .bytes()
        .position(|digit| digit != b'0')
        .unwrap_or(digits.len());
    digits.drain(..first_non_zero);
    let scale = frac_part.chars().filter(|c| c.is_ascii_digit()).count() as i64 - exponent;
    (negative && !digits.is_empty(), digits, scale)
}

#[cfg(test)]
#[path = "number_math_helpers_tests.rs"]
mod tests;
