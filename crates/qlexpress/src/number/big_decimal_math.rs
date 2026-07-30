//! BigDecimal 数值域实现(十进制高精度,字符串保精度存储)。
//!
//! 对应 Java: com.alibaba.qlexpress4.runtime.operator.number.BigDecimalMath。
//!
//! 语义要点(SPEC §3.1:BigDec 用字符串保精度存储,运算时解析):
//! - 加/减/乘:精确十进制运算,结果 scale 对齐 BigDecimal
//!   (add/sub 取两边较大 scale,multiply 取 scale 之和);
//! - 除法:先尝试 `BigDecimal.divide`(精确除,不能整除时 Java 抛
//!   ArithmeticException "Non-terminating decimal expansion");失败后按
//!   `precision = max(左精度, 右精度) + DIVISION_EXTRA_PRECISION(10)`
//!   的 MathContext(HALF_UP)重除,再把 scale 收敛到
//!   `max(左scale, 右scale, DIVISION_MIN_SCALE(10))`(HALF_UP);
//! - 除数为零抛 ArithmeticException("Division by zero");
//! - remainder:`BigDecimal.remainder`(向零取整的余数,符号跟被除数);
//! - mod:remainder 为负时无条件加除数（逐字对齐 Java `modImpl`；Java
//!   对负除数不会额外保证非负）。

use super::number_math;
use crate::exception::QLException;
use crate::runtime::data::convert;
use crate::runtime::value::DataValue;

/// Java `DIVISION_EXTRA_PRECISION`(系统属性
/// `qlexpress4.division.extra.precision`,默认 10):非整除除法的额外精度。
pub const DIVISION_EXTRA_PRECISION: usize = 10;

/// Java `DIVISION_MIN_SCALE`(系统属性 `qlexpress4.division.min.scale`,
/// 默认 10):非整除除法结果的最小 scale。
pub const DIVISION_MIN_SCALE: i64 = 10;

/// 对应 Java: BigDecimalMath(单例,Rust 用零大小类型 + 关联函数)。
pub struct BigDecimalMath;

/// 十进制高精度数的内部表示:`(-1)^neg × digits × 10^-scale`。
/// digits 为大端十进制数字(0-9),无前导零;零用空 vec 表示。
#[derive(Clone, Debug)]
struct Decimal {
    neg: bool,
    digits: Vec<u8>,
    scale: i64,
}

impl Decimal {
    fn zero(scale: i64) -> Decimal {
        Decimal {
            neg: false,
            digits: Vec::new(),
            scale,
        }
    }

    fn is_zero(&self) -> bool {
        self.digits.is_empty()
    }

    /// Java `BigDecimal.precision()`:非缩放值的十进制位数(零为 1)。
    fn precision(&self) -> usize {
        self.digits.len().max(1)
    }

    fn negate(&self) -> Decimal {
        Decimal {
            neg: !self.neg,
            digits: self.digits.clone(),
            scale: self.scale,
        }
    }

    fn abs(&self) -> Decimal {
        Decimal {
            neg: false,
            digits: self.digits.clone(),
            scale: self.scale,
        }
    }

    /// 去掉前导零(零规范为空 vec)。
    fn normalized(mut self) -> Decimal {
        let first_nz = self
            .digits
            .iter()
            .position(|&d| d != 0)
            .unwrap_or(self.digits.len());
        self.digits.drain(..first_nz);
        if self.digits.is_empty() {
            self.neg = false;
        }
        self
    }

    /// 按 Java `BigDecimal.toString()` 渲染：
    /// `scale >= 0 && adjustedExponent >= -6` 使用普通记数法，否则使用
    /// 带显式指数的规范科学记数法；尾随零由 unscaled value/scale 保留。
    fn to_java_string(&self) -> String {
        let mut out = String::new();
        if self.neg && !self.is_zero() {
            out.push('-');
        }

        let digits: Vec<u8> = if self.digits.is_empty() {
            vec![0]
        } else {
            self.digits.clone()
        };
        let precision = digits.len() as i64;
        let adjusted_exponent = -self.scale + precision - 1;
        if self.scale >= 0 && adjusted_exponent >= -6 {
            if self.scale == 0 {
                out.extend(digits.iter().map(|d| (b'0' + d) as char));
                return out;
            }
            if precision > self.scale {
                let int_len = (precision - self.scale) as usize;
                out.extend(digits[..int_len].iter().map(|d| (b'0' + d) as char));
                out.push('.');
                out.extend(digits[int_len..].iter().map(|d| (b'0' + d) as char));
            } else {
                out.push_str("0.");
                for _ in 0..(self.scale - precision) {
                    out.push('0');
                }
                out.extend(digits.iter().map(|d| (b'0' + d) as char));
            }
            return out;
        }

        out.push((b'0' + digits[0]) as char);
        if digits.len() > 1 {
            out.push('.');
            out.extend(digits[1..].iter().map(|d| (b'0' + d) as char));
        }
        out.push('E');
        if adjusted_exponent > 0 {
            out.push('+');
        }
        out.push_str(&adjusted_exponent.to_string());
        out
    }
}

/// 解析 Java `BigDecimal(String)` 接受的十进制/指数文本并保留 signed scale。
fn parse_dec(s: &str) -> Decimal {
    let trimmed = s.trim();
    let (neg, body) = match trimmed.strip_prefix('-') {
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
    let mut digits: Vec<u8> = int_part
        .chars()
        .chain(frac_part.chars())
        .filter(|c| c.is_ascii_digit())
        .map(|c| (c as u8) - b'0')
        .collect();
    let scale =
        frac_part.chars().filter(|c| c.is_ascii_digit()).count() as i64 - exponent;
    let first_nz = digits.iter().position(|&d| d != 0).unwrap_or(digits.len());
    digits.drain(..first_nz);
    Decimal {
        neg: neg && !digits.is_empty(),
        digits,
        scale,
    }
}

/// Java `NumberMath.toBigDecimal(n)` 后取内部表示。
fn dec_of(v: &DataValue) -> Decimal {
    parse_dec(&convert::to_big_dec_string(v))
}

// ---------- 数字串(幅值)运算:大端 vec<u8> ----------

fn cmp_mag(a: &[u8], b: &[u8]) -> std::cmp::Ordering {
    a.len().cmp(&b.len()).then_with(|| a.cmp(b))
}

fn add_mag(a: &[u8], b: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(a.len().max(b.len()) + 1);
    let mut carry = 0u8;
    for i in 0..a.len().max(b.len()) {
        let da = if i < a.len() { a[a.len() - 1 - i] } else { 0 };
        let db = if i < b.len() { b[b.len() - 1 - i] } else { 0 };
        let s = da + db + carry;
        out.push(s % 10);
        carry = s / 10;
    }
    if carry > 0 {
        out.push(carry);
    }
    out.reverse();
    out
}

/// a - b,要求 a >= b。
fn sub_mag(a: &[u8], b: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(a.len());
    let mut borrow = 0i8;
    for i in 0..a.len() {
        let da = a[a.len() - 1 - i] as i8;
        let db = if i < b.len() {
            b[b.len() - 1 - i] as i8
        } else {
            0
        };
        let mut d = da - db - borrow;
        if d < 0 {
            d += 10;
            borrow = 1;
        } else {
            borrow = 0;
        }
        out.push(d as u8);
    }
    out.reverse();
    let first_nz = out.iter().position(|&d| d != 0).unwrap_or(out.len());
    out.drain(..first_nz);
    out
}

fn mul_mag(a: &[u8], b: &[u8]) -> Vec<u8> {
    if a.is_empty() || b.is_empty() {
        return Vec::new();
    }
    let mut out = vec![0u16; a.len() + b.len()];
    for (i, &da) in a.iter().rev().enumerate() {
        for (j, &db) in b.iter().rev().enumerate() {
            out[i + j] += da as u16 * db as u16;
        }
    }
    let mut carry = 0u16;
    for slot in out.iter_mut() {
        let v = *slot + carry;
        *slot = v % 10;
        carry = v / 10;
    }
    let mut digits: Vec<u8> = out.into_iter().map(|v| v as u8).collect();
    digits.reverse();
    let first_nz = digits.iter().position(|&d| d != 0).unwrap_or(digits.len());
    digits.drain(..first_nz);
    digits
}

fn mul_pow10(a: &[u8], n: usize) -> Vec<u8> {
    if a.is_empty() {
        return Vec::new();
    }
    let mut out = a.to_vec();
    out.extend(std::iter::repeat_n(0, n));
    out
}

/// 长除法:q = a / b(向下取整),r = a % b;b 必须非零。
fn divmod_mag(a: &[u8], b: &[u8]) -> (Vec<u8>, Vec<u8>) {
    debug_assert!(!b.is_empty());
    if cmp_mag(a, b) == std::cmp::Ordering::Less {
        return (Vec::new(), a.to_vec());
    }
    let mut quotient = Vec::with_capacity(a.len());
    let mut rem: Vec<u8> = Vec::new();
    for &digit in a {
        rem.push(digit);
        let first_nz = rem.iter().position(|&d| d != 0).unwrap_or(rem.len());
        rem.drain(..first_nz);
        // 试商:十进制下逐位二分 0..=9。
        let mut lo = 0u8;
        let mut hi = 9u8;
        while lo < hi {
            let mid = (lo + hi).div_ceil(2);
            if cmp_mag(&mul_mag(b, &[mid]), &rem) != std::cmp::Ordering::Greater {
                lo = mid;
            } else {
                hi = mid - 1;
            }
        }
        if lo > 0 {
            rem = sub_mag(&rem, &mul_mag(b, &[lo]));
        }
        quotient.push(lo);
    }
    let first_nz = quotient
        .iter()
        .position(|&d| d != 0)
        .unwrap_or(quotient.len());
    quotient.drain(..first_nz);
    (quotient, rem)
}

// ---------- Decimal 层运算 ----------

/// 同号幅值对齐后相加/相减,产出 Decimal。
fn add_sub(l: &Decimal, r: &Decimal, subtract: bool) -> Decimal {
    let scale = l.scale.max(r.scale);
    let lm = mul_pow10(&l.digits, (scale - l.scale) as usize);
    let rm = mul_pow10(&r.digits, (scale - r.scale) as usize);
    let r_neg = r.neg ^ subtract;
    if l.neg == r_neg {
        // 同号:幅值相加。
        Decimal {
            neg: l.neg,
            digits: add_mag(&lm, &rm),
            scale,
        }
        .normalized()
    } else {
        match cmp_mag(&lm, &rm) {
            std::cmp::Ordering::Equal => Decimal::zero(scale),
            std::cmp::Ordering::Greater => Decimal {
                neg: l.neg,
                digits: sub_mag(&lm, &rm),
                scale,
            }
            .normalized(),
            std::cmp::Ordering::Less => Decimal {
                neg: r_neg,
                digits: sub_mag(&rm, &lm),
                scale,
            }
            .normalized(),
        }
    }
}

fn mul_dec(l: &Decimal, r: &Decimal) -> Decimal {
    Decimal {
        neg: l.neg != r.neg,
        digits: mul_mag(&l.digits, &r.digits),
        scale: l.scale + r.scale,
    }
    .normalized()
}

/// 精确除法(Java `BigDecimal.divide`):能整除时返回精确商
/// (scale = max(左scale - 右scale, 恰好整除所需的最小 scale));
/// 否则返回 None(Java 抛 "Non-terminating decimal expansion")。
fn divide_exact(l: &Decimal, r: &Decimal) -> Option<Decimal> {
    if r.is_zero() {
        return None; // 调用方先判零,这里防御
    }
    if l.is_zero() {
        // Java `BigDecimal.divide` 的零结果保留 preferred scale。
        return Some(Decimal::zero(l.scale - r.scale));
    }
    // 结果 scale 从 preferred scale(s1 - s2) 起步,逐步放大直到整除。
    let preferred_scale = l.scale - r.scale;
    let mut scale = preferred_scale;
    // 十进制 n 位整数若约分后只含 2/5，终止 scale 最多约 3.322n
    // (`2^k` 是最坏情形)；四倍总位数是严格安全的有限搜索上界。
    let max_extra_digits = (r.digits.len() + l.digits.len())
        .saturating_mul(4)
        .saturating_add(2);
    for _ in 0..=max_extra_digits {
        // 被除数幅值乘 10^(scale + s2 - s1)。
        let exp = (scale - preferred_scale) as usize;
        let num = mul_pow10(&l.digits, exp);
        let (q, rem) = divmod_mag(&num, &r.digits);
        if rem.is_empty() {
            return Some(
                Decimal {
                    neg: l.neg != r.neg,
                    digits: q,
                    scale,
                }
                .normalized(),
            );
        }
        scale += 1;
    }
    None
}

/// 按 MathContext(precision)(HALF_UP)除法:商保留 precision 位有效数字。
fn divide_with_precision(l: &Decimal, r: &Decimal, precision: usize) -> Decimal {
    // 求 e 使 floor(|m1| × 10^e / |m2|) 恰有 precision 位。
    let base = l.digits.len() as i64 - r.digits.len() as i64;
    let mut e = precision as i64 - base;
    let (mut q, rem);
    loop {
        let num = if e >= 0 {
            mul_pow10(&l.digits, e as usize)
        } else {
            l.digits.clone()
        };
        let den = if e >= 0 {
            r.digits.clone()
        } else {
            mul_pow10(&r.digits, (-e) as usize)
        };
        let (qq, rr) = divmod_mag(&num, &den);
        let len = qq.len().max(1);
        if len > precision {
            e -= 1;
        } else if len < precision {
            e += 1;
        } else {
            q = qq;
            rem = rr;
            break;
        }
    }
    // HALF_UP:余数 ×2 >= 除数则进位。注意 e<0 时除数已放大,比较口径一致。
    let den_for_round = if e >= 0 {
        r.digits.clone()
    } else {
        mul_pow10(&r.digits, (-e) as usize)
    };
    if cmp_mag(&mul_mag(&rem, &[2]), &den_for_round) != std::cmp::Ordering::Less {
        q = add_mag(&q, &[1]);
    }
    // 进位导致位数溢出(999..→1000..):砍掉一位,scale 相应减一。
    let mut extra_carry = false;
    if q.len() > precision {
        q.truncate(q.len() - 1);
        extra_carry = true;
    }
    // 商 q 的实际 scale:value = q × 10^(s2-s1-e) ⇒ scale = s1 - s2 + e
    // (+ 进位修正)。
    let mut scale = l.scale - r.scale + e;
    if extra_carry {
        scale -= 1;
    }
    Decimal {
        neg: l.neg != r.neg,
        digits: q,
        scale,
    }
    .normalized()
}

/// Java `setScale(newScale, RoundingMode.HALF_UP)`(仅缩 scale 场景)。
fn set_scale_half_up(d: &Decimal, new_scale: i64) -> Decimal {
    if d.scale <= new_scale {
        return d.clone();
    }
    let shift = (d.scale - new_scale) as usize;
    let divisor = mul_pow10(&[1], shift);
    let (q, rem) = divmod_mag(&d.digits, &divisor);
    let rounded = if cmp_mag(&mul_mag(&rem, &[2]), &divisor) != std::cmp::Ordering::Less {
        add_mag(&q, &[1])
    } else {
        q
    };
    Decimal {
        neg: d.neg,
        digits: rounded,
        scale: new_scale,
    }
    .normalized()
}

impl BigDecimalMath {
    /// 返回当前数值域的绝对值。
    /// 参数：`number`；返回：`Result<DataValue, QLException>`。
    /// 对应或承接 Java 源文件：`com/alibaba/qlexpress4/runtime/operator/number/BigDecimalMath.java`，方法 `absImpl`；Rust 侧按所有权与 `Result` 语义适配。
    /// Java `absImpl`。
    /// 对应 Java: com.alibaba.qlexpress4.runtime.operator.number.BigDecimalMath#absImpl。
    pub fn abs_impl(number: &DataValue) -> Result<DataValue, QLException> {
        Ok(DataValue::BigDec(dec_of(number).abs().to_java_string()))
    }

    /// Java `addImpl`(精确,scale 取两边较大)。
    /// 对应 Java: com.alibaba.qlexpress4.runtime.operator.number.BigDecimalMath#addImpl。
    pub fn add_impl(left: &DataValue, right: &DataValue) -> Result<DataValue, QLException> {
        Ok(DataValue::BigDec(
            add_sub(&dec_of(left), &dec_of(right), false).to_java_string(),
        ))
    }

    /// 在当前数值域执行减法。
    /// 参数：`left`、`right`；返回：`Result<DataValue, QLException>`。
    /// 对应或承接 Java 源文件：`com/alibaba/qlexpress4/runtime/operator/number/BigDecimalMath.java`，方法 `subtractImpl`；Rust 侧按所有权与 `Result` 语义适配。
    /// Java `subtractImpl`。
    /// 对应 Java: com.alibaba.qlexpress4.runtime.operator.number.BigDecimalMath#subtractImpl。
    pub fn subtract_impl(left: &DataValue, right: &DataValue) -> Result<DataValue, QLException> {
        Ok(DataValue::BigDec(
            add_sub(&dec_of(left), &dec_of(right), true).to_java_string(),
        ))
    }

    /// Java `multiplyImpl`(精确,scale 相加)。
    /// 对应 Java: com.alibaba.qlexpress4.runtime.operator.number.BigDecimalMath#multiplyImpl。
    pub fn multiply_impl(left: &DataValue, right: &DataValue) -> Result<DataValue, QLException> {
        Ok(DataValue::BigDec(
            mul_dec(&dec_of(left), &dec_of(right)).to_java_string(),
        ))
    }

    /// Java `divideImpl`(精度与舍入语义见文件头注释)。
    /// 对应 Java: com.alibaba.qlexpress4.runtime.operator.number.BigDecimalMath#divideImpl。
    pub fn divide_impl(left: &DataValue, right: &DataValue) -> Result<DataValue, QLException> {
        let big_left = dec_of(left);
        let big_right = dec_of(right);
        // Java `bigLeft.divide(bigRight)`:除数为零抛 ArithmeticException。
        if big_right.is_zero() {
            return Err(number_math::arithmetic_exception("Division by zero"));
        }
        match divide_exact(&big_left, &big_right) {
            Some(exact) => Ok(DataValue::BigDec(exact.to_java_string())),
            None => {
                // Java catch(ArithmeticException):非终止小数按默认精度重除。
                let precision =
                    big_left.precision().max(big_right.precision()) + DIVISION_EXTRA_PRECISION;
                let result = divide_with_precision(&big_left, &big_right, precision);
                let scale = big_left.scale.max(big_right.scale).max(DIVISION_MIN_SCALE);
                let result = if result.scale > scale {
                    set_scale_half_up(&result, scale)
                } else {
                    result
                };
                Ok(DataValue::BigDec(result.to_java_string()))
            }
        }
    }

    /// Java `compareToImpl`(忽略 scale:`1.0 == 1.00`)。
    /// 对应 Java: com.alibaba.qlexpress4.runtime.operator.number.BigDecimalMath#compareToImpl。
    pub fn compare_to_impl(left: &DataValue, right: &DataValue) -> i32 {
        match convert::big_dec_compare(
            &convert::to_big_dec_string(left),
            &convert::to_big_dec_string(right),
        ) {
            std::cmp::Ordering::Less => -1,
            std::cmp::Ordering::Equal => 0,
            std::cmp::Ordering::Greater => 1,
        }
    }

    /// 返回当前数值取相反数后的结果。
    /// 参数：`left`；返回：`Result<DataValue, QLException>`。
    /// 对应或承接 Java 源文件：`com/alibaba/qlexpress4/runtime/operator/number/BigDecimalMath.java`，方法 `unaryMinusImpl`；Rust 侧按所有权与 `Result` 语义适配。
    /// Java `unaryMinusImpl`。
    /// 对应 Java: com.alibaba.qlexpress4.runtime.operator.number.BigDecimalMath#unaryMinusImpl。
    pub fn unary_minus_impl(left: &DataValue) -> Result<DataValue, QLException> {
        Ok(DataValue::BigDec(dec_of(left).negate().to_java_string()))
    }

    /// 返回当前数值的一元正号结果。
    /// 参数：`left`；返回：`Result<DataValue, QLException>`。
    /// 对应或承接 Java 源文件：`com/alibaba/qlexpress4/runtime/operator/number/BigDecimalMath.java`，方法 `unaryPlusImpl`；Rust 侧按所有权与 `Result` 语义适配。
    /// Java `unaryPlusImpl`。
    /// 对应 Java: com.alibaba.qlexpress4.runtime.operator.number.BigDecimalMath#unaryPlusImpl。
    pub fn unary_plus_impl(left: &DataValue) -> Result<DataValue, QLException> {
        Ok(DataValue::BigDec(dec_of(left).to_java_string()))
    }

    /// Java `remainderImpl`:`BigDecimal.remainder` = 被除数 - 商(向零取整)×除数。
    /// 对应 Java: com.alibaba.qlexpress4.runtime.operator.number.BigDecimalMath#remainderImpl。
    pub fn remainder_impl(left: &DataValue, right: &DataValue) -> Result<DataValue, QLException> {
        let l = dec_of(left);
        let r = dec_of(right);
        if r.is_zero() {
            return Err(number_math::arithmetic_exception("Division by zero"));
        }
        // 统一放大到同一 scale 的整数域:q = trunc(lm / rm) 即向零取整商。
        let scale = l.scale.max(r.scale);
        let lm = mul_pow10(&l.digits, (scale - l.scale) as usize);
        let rm = mul_pow10(&r.digits, (scale - r.scale) as usize);
        let (q, _) = divmod_mag(&lm, &rm);
        // rem = lm - q × rm(同 scale 下的幅值余数)。
        let prod = mul_mag(&q, &rm);
        let rem_mag = if cmp_mag(&lm, &prod) == std::cmp::Ordering::Less {
            // 防御:理论上 lm >= prod。
            Vec::new()
        } else {
            sub_mag(&lm, &prod)
        };
        // 余数符号跟被除数(BigDecimal.remainder 语义)。
        Ok(DataValue::BigDec(
            Decimal {
                neg: l.neg,
                digits: rem_mag,
                scale,
            }
            .normalized()
            .to_java_string(),
        ))
    }

    /// Java `modImpl`:remainder 为负则无条件加除数。
    /// 对应 Java: com.alibaba.qlexpress4.runtime.operator.number.BigDecimalMath#modImpl。
    pub fn mod_impl(self_value: &DataValue, divisor: &DataValue) -> Result<DataValue, QLException> {
        let remainder = Self::remainder_impl(self_value, divisor)?;
        if let DataValue::BigDec(rem_str) = &remainder {
            let rem = parse_dec(rem_str);
            if rem.neg && !rem.is_zero() {
                // remainder.signum() < 0 → remainder + divisor。
                return Self::add_impl(
                    &DataValue::BigDec(rem.to_java_string()),
                    &DataValue::BigDec(convert::to_big_dec_string(divisor)),
                );
            }
        }
        Ok(remainder)
    }
}

#[cfg(test)]
mod tests {
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
        // 7 / 2 = 3.5(精确)。
        assert_eq!(div("7", "2"), DataValue::BigDec("3.5".to_string()));
        // 2.00 / 2 = 1.00(保留 preferred scale)。
        assert_eq!(div("2.00", "2"), DataValue::BigDec("1.00".to_string()));
        // 1 / 8 = 0.125。
        assert_eq!(div("1", "8"), DataValue::BigDec("0.125".to_string()));
    }

    #[test]
    fn non_terminating_division_uses_default_precision_and_half_up() {
        // Java 语义要点:1 / 3 → MathContext(11) 后收敛 scale 10,HALF_UP。
        assert_eq!(div("1", "3"), DataValue::BigDec("0.3333333333".to_string()));
        // 2 / 3 → 末位进位。
        assert_eq!(div("2", "3"), DataValue::BigDec("0.6666666667".to_string()));
        // 10 / 3 → 3.3333333333。
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
            DataValue::BigDec("2.00".to_string())
        );
        assert_eq!(
            BigDecimalMath::subtract_impl(
                &DataValue::BigDec("0.3".into()),
                &DataValue::BigDec("0.1".into())
            )
            .unwrap(),
            DataValue::BigDec("0.2".to_string())
        );
        assert_eq!(
            BigDecimalMath::multiply_impl(
                &DataValue::BigDec("1.5".into()),
                &DataValue::BigDec("0.02".into())
            )
            .unwrap(),
            DataValue::BigDec("0.030".to_string())
        );
        // 负数。
        assert_eq!(
            BigDecimalMath::add_impl(
                &DataValue::BigDec("-1.5".into()),
                &DataValue::BigDec("0.5".into())
            )
            .unwrap(),
            DataValue::BigDec("-1.0".to_string())
        );
    }

    #[test]
    fn remainder_and_mod_sign_rules() {
        // BigDecimal.remainder:符号跟被除数。
        assert_eq!(
            BigDecimalMath::remainder_impl(
                &DataValue::BigDec("-7".into()),
                &DataValue::BigDec("3".into())
            )
            .unwrap(),
            DataValue::BigDec("-1".to_string())
        );
        // mod:负余数加除数 → 非负。
        assert_eq!(
            BigDecimalMath::mod_impl(
                &DataValue::BigDec("-7".into()),
                &DataValue::BigDec("3".into())
            )
            .unwrap(),
            DataValue::BigDec("2".to_string())
        );
        // Java 实现对负除数仍是“负余数 + 除数”，不会强制结果非负。
        assert_eq!(
            BigDecimalMath::mod_impl(
                &DataValue::BigDec("-7".into()),
                &DataValue::BigDec("-3".into())
            )
            .unwrap(),
            DataValue::BigDec("-4".to_string())
        );
    }

    #[test]
    fn integer_operands_also_flow_through() {
        // Java: IntegerMath.divideImpl 委托到这里,int 也能直接除。
        assert_eq!(
            BigDecimalMath::divide_impl(&DataValue::Int(1), &DataValue::Long(4)).unwrap(),
            DataValue::BigDec("0.25".to_string())
        );
    }

    /// SOURCE_PARITY: Java `BigDecimal(String)` 保留 signed scale，
    /// `toString()` 在 adjusted exponent 小于 -6 或 scale 为负时使用科学记数法。
    #[test]
    fn exponent_input_and_java_to_string_preserve_signed_scale() {
        let cases = [
            ("1.0E20", "1.0E+20", -19, 2),
            ("1E+2", "1E+2", -2, 1),
            ("0E+7", "0E+7", -7, 1),
            ("0E-7", "0E-7", 7, 1),
            ("0.000001", "0.000001", 6, 1),
            ("0.0000001", "1E-7", 7, 1),
        ];
        for (source, expected, scale, precision) in cases {
            let decimal = parse_dec(source);
            assert_eq!(decimal.to_java_string(), expected, "source={source}");
            assert_eq!(decimal.scale, scale, "source={source}");
            assert_eq!(decimal.precision(), precision, "source={source}");
        }
    }

    /// SOURCE_PARITY: signed scale 必须贯穿 add/multiply/exact divide 和
    /// QLExpress 的非终止除法回退路径。
    #[test]
    fn arithmetic_with_negative_scale_matches_jdk_oracle() {
        assert_eq!(
            BigDecimalMath::add_impl(
                &DataValue::BigDec("1.0E+2".into()),
                &DataValue::BigDec("1".into()),
            )
            .unwrap(),
            DataValue::BigDec("101".into())
        );
        assert_eq!(
            BigDecimalMath::multiply_impl(
                &DataValue::BigDec("1.0E+2".into()),
                &DataValue::BigDec("2.00".into()),
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
                &DataValue::BigDec("3E+1".into()),
            )
            .unwrap(),
            DataValue::BigDec("20".into())
        );
    }
}
