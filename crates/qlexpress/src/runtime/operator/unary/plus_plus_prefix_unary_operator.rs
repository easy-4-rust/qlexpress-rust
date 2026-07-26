//! 前缀 `++` 操作符,对应 Java
//! `com.alibaba.qlexpress4.runtime.operator.unary.PlusPlusPrefixUnaryOperator`。

use crate::exception::error_codes;
use crate::exception::error_reporter::ErrorReporter;
use crate::exception::QLException;
use crate::ql_precedences;
use crate::runtime::data::convert::to_f64;
use crate::runtime::operator::base::UnaryOperator;
use crate::runtime::value::{DataValue, QValue};

/// 前缀 `++` 操作符(先自增,后取值)。
///
/// 对应 Java: com.alibaba.qlexpress4.runtime.operator.unary.PlusPlusPrefixUnaryOperator
/// (@author bingo;执行体委托 `NumberMath.add(operand, 1)`,错误构造继承自
/// `BaseUnaryOperator`)。
#[derive(Clone, Copy, Debug, Default)]
pub struct PlusPlusPrefixUnaryOperator;

impl PlusPlusPrefixUnaryOperator {
    /// 对应 Java `PlusPlusPrefixUnaryOperator.getInstance()` 单例。
    pub fn get_instance() -> PlusPlusPrefixUnaryOperator {
        PlusPlusPrefixUnaryOperator
    }
}

impl UnaryOperator for PlusPlusPrefixUnaryOperator {
    /// 对应 Java `PlusPlusPrefixUnaryOperator.execute(Value value,
    /// ErrorReporter)`:
    /// ```java
    /// Object operand = value.get();
    /// if (!(operand instanceof Number)) throw ...;
    /// Number result = NumberMath.add((Number)operand, 1);
    /// if (value instanceof LeftValue) ((LeftValue)value).set(result, errorReporter);
    /// return result;
    /// ```
    fn execute(
        &self,
        value: &QValue,
        error_reporter: &dyn ErrorReporter,
    ) -> Result<DataValue, QLException> {
        let operand = value.get();
        // Java:!(operand instanceof Number) → buildInvalidOperandTypeException
        if !operand.is_number() {
            return Err(build_invalid_operand_type_exception(
                value,
                self.operator(),
                error_reporter,
            ));
        }

        // Java:NumberMath.add(operand, 1)
        let result = number_add_one(&operand);
        // Java:value instanceof LeftValue → set(result, errorReporter)
        // 写穿变量槽(声明类型不兼容时 set 内抛
        // INCOMPATIBLE_ASSIGNMENT_TYPE);前缀与后缀的差异仅在返回值。
        if let Some(left_value) = value.as_left() {
            left_value
                .borrow_mut()
                .set(result.clone(), error_reporter)?;
        }
        // Java 前缀:return result(自增后的新值)
        Ok(result)
    }

    /// 对应 Java `getOperator()`:操作符词素 `"++"`。
    fn operator(&self) -> &str {
        "++"
    }

    /// 对应 Java `getPriority()`:`QLPrecedences.UNARY`。
    fn priority(&self) -> i32 {
        ql_precedences::UNARY
    }
}

/// 对应 Java `NumberMath.add(operand, 1)`(右操作数为 int 常量 1,
/// 经 `getMath(left, right)` 提升矩阵分发):
/// - `IntegerMath.addImpl`:`intValue() + 1` → **Byte/Short/Int 结果为 Int**;
/// - `LongMath.addImpl`:Long + 1;
/// - `BigIntegerMath.addImpl`:BigInteger + 1;
/// - `BigDecimalMath.addImpl`:BigDecimal + 1(十进制字符串实现);
/// - `FloatingPointMath.addImpl`:`doubleValue() + 1` → **Float 结果为
///   Double**。
///
/// `pub(crate)`:Java 中该逻辑经 `NumberMath` 被前缀/后缀 `++` 共享,
/// Rust 侧由后缀操作符复用本函数。
pub(crate) fn number_add_one(operand: &DataValue) -> DataValue {
    match operand {
        DataValue::Byte(v) => DataValue::Int((*v as i32).wrapping_add(1)),
        DataValue::Short(v) => DataValue::Int((*v as i32).wrapping_add(1)),
        DataValue::Int(v) => DataValue::Int(v.wrapping_add(1)),
        DataValue::Long(v) => DataValue::Long(v.wrapping_add(1)),
        DataValue::BigInt(v) => DataValue::BigInt(v + 1),
        DataValue::BigDec(v) => DataValue::BigDec(big_dec_add_one(v)),
        DataValue::Float(_) | DataValue::Double(_) => DataValue::Double(to_f64(operand) + 1.0),
        // 调用点已保证 is_number,其余类型不可达
        _ => unreachable!("++ on non-number"),
    }
}

/// 十进制字符串加一,对应 Java `BigDecimal.add(BigDecimal.ONE)` 在字符串
/// 存储上的等价实现。
///
/// 要点:±1 只影响整数部分(小数位不变),唯一特殊情形是负值且整数部分
/// 为 0(如 `-0.25 + 1 = 0.75`),需要按 `1 - 0.frac` 补位计算。
fn big_dec_add_one(dec: &str) -> String {
    let (negative, int_part, frac_part) = split_dec(dec);
    // 零值 + 1 = 1(覆盖 "-0.000" 等写法)
    if int_part == "0" && frac_part.bytes().all(|b| b == b'0') {
        return render_dec(false, "1".to_string(), frac_part);
    }
    if !negative {
        render_dec(false, incr_digits(&int_part), frac_part)
    } else if int_part == "0" {
        // 值 ∈ (-1, 0):+1 后 = 1 - 0.frac = 0.(逐位对 9 取补,末位 +1)
        render_dec(false, "0".to_string(), one_minus_point_frac(&frac_part))
    } else {
        // -(I.F) + 1 = -((I-1).F)
        render_dec(true, decr_digits(&int_part), frac_part)
    }
}

/// 拆十进制字符串为 (是否负, 整数部分, 小数部分),容忍前导 `+` 与前导零。
fn split_dec(dec: &str) -> (bool, String, String) {
    let (negative, body) = match dec.strip_prefix('-') {
        Some(body) => (true, body),
        None => (false, dec),
    };
    let body = body.strip_prefix('+').unwrap_or(body);
    let (int_part, frac_part) = match body.split_once('.') {
        Some((i, f)) => (i, f),
        None => (body, ""),
    };
    let int_trimmed = int_part.trim_start_matches('0');
    let int_norm = if int_trimmed.is_empty() {
        "0"
    } else {
        int_trimmed
    };
    (negative, int_norm.to_string(), frac_part.to_string())
}

/// 渲染 (符号, 整数, 小数) 回十进制字符串;零值不带负号。
fn render_dec(negative: bool, int_part: String, frac_part: String) -> String {
    let is_zero = int_part == "0" && frac_part.bytes().all(|b| b == b'0');
    let mut out = String::new();
    if negative && !is_zero {
        out.push('-');
    }
    out.push_str(&int_part);
    if !frac_part.is_empty() {
        out.push('.');
        out.push_str(&frac_part);
    }
    out
}

/// 数字字符串加一(十进制进位)。
fn incr_digits(digits: &str) -> String {
    let mut buf: Vec<u8> = digits.bytes().map(|b| b - b'0').collect();
    let mut i = buf.len();
    loop {
        if i == 0 {
            buf.insert(0, 1);
            break;
        }
        i -= 1;
        if buf[i] == 9 {
            buf[i] = 0;
        } else {
            buf[i] += 1;
            break;
        }
    }
    buf.iter().map(|d| (d + b'0') as char).collect()
}

/// 数字字符串减一(十进制借位,要求数值 ≥ 1)。
fn decr_digits(digits: &str) -> String {
    let mut buf: Vec<u8> = digits.bytes().map(|b| b - b'0').collect();
    let mut i = buf.len();
    loop {
        i -= 1;
        if buf[i] == 0 {
            buf[i] = 9;
        } else {
            buf[i] -= 1;
            break;
        }
    }
    let s: String = buf.iter().map(|d| (d + b'0') as char).collect();
    let trimmed = s.trim_start_matches('0');
    if trimmed.is_empty() {
        "0".to_string()
    } else {
        trimmed.to_string()
    }
}

/// 计算 `1 - 0.frac` 的小数部分(等长补位:`10^n - frac`)。
fn one_minus_point_frac(frac: &str) -> String {
    let n = frac.len();
    let mut buf: Vec<u8> = frac.bytes().map(|b| b - b'0').collect();
    // 从末位起:第一个非零位取 10-d,其左各位取 9-d,其右保持 0
    let mut i = n;
    while i > 0 {
        i -= 1;
        if buf[i] != 0 {
            buf[i] = 10 - buf[i];
            break;
        }
    }
    for d in buf.iter_mut().take(i) {
        *d = 9 - *d;
    }
    let s: String = buf.iter().map(|d| (d + b'0') as char).collect();
    // 末尾的零只是借位产物,裁掉(数值不变)
    let trimmed = s.trim_end_matches('0');
    trimmed.to_string()
}

/// 对应 Java `BaseUnaryOperator.buildInvalidOperandTypeException`:
/// 错误码 `INVALID_UNARY_OPERAND`,参数为操作符、类型名与值。
fn build_invalid_operand_type_exception(
    value: &QValue,
    operator: &str,
    error_reporter: &dyn ErrorReporter,
) -> QLException {
    error_reporter.report_format(
        error_codes::INVALID_UNARY_OPERAND,
        error_codes::error_msg(error_codes::INVALID_UNARY_OPERAND),
        &[
            operator.to_string(),
            value.type_name().to_string(),
            value.get().string_value_of(),
        ],
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::exception::pure_err_reporter::PureErrReporter;
    use crate::runtime::data::assignable_data_value::AssignableDataValue;
    use crate::runtime::value::Value;
    use std::cell::RefCell;
    use std::rc::Rc;

    fn run(value: QValue) -> Result<DataValue, QLException> {
        PlusPlusPrefixUnaryOperator::get_instance().execute(&value, &PureErrReporter::INSTANCE)
    }

    #[test]
    fn prefix_plus_plus_returns_incremented_and_writes_back() {
        // Java:++a → 槽内变 2,表达式值也是 2(前缀返回 result)
        let slot = Rc::new(RefCell::new(AssignableDataValue::new(
            "a",
            DataValue::Int(1),
        )));
        let result = run(QValue::Left(slot.clone())).unwrap();
        assert_eq!(result, DataValue::Int(2));
        assert_eq!(slot.borrow().get(), DataValue::Int(2));
    }

    #[test]
    fn prefix_plus_plus_promotes_byte_to_int() {
        // Java IntegerMath.addImpl:intValue() + 1
        let slot = Rc::new(RefCell::new(AssignableDataValue::new(
            "a",
            DataValue::Byte(1),
        )));
        assert_eq!(run(QValue::Left(slot.clone())).unwrap(), DataValue::Int(2));
        assert_eq!(slot.borrow().get(), DataValue::Int(2));
        // Long 保持 Long
        assert_eq!(
            run(QValue::from(DataValue::Long(1))).unwrap(),
            DataValue::Long(2)
        );
        // Float 经 FloatingPointMath → Double
        assert_eq!(
            run(QValue::from(DataValue::Float(0.5))).unwrap(),
            DataValue::Double(1.5)
        );
    }

    #[test]
    fn prefix_plus_plus_big_decimal_add_one() {
        // Java BigDecimalMath:add(ONE)
        assert_eq!(
            run(QValue::from(DataValue::BigDec("1.50".into()))).unwrap(),
            DataValue::BigDec("2.50".into())
        );
        assert_eq!(
            run(QValue::from(DataValue::BigDec("-1.5".into()))).unwrap(),
            DataValue::BigDec("-0.5".into())
        );
        assert_eq!(
            run(QValue::from(DataValue::BigDec("-0.25".into()))).unwrap(),
            DataValue::BigDec("0.75".into())
        );
        assert_eq!(
            run(QValue::from(DataValue::BigDec("9".into()))).unwrap(),
            DataValue::BigDec("10".into())
        );
    }

    #[test]
    fn prefix_plus_plus_rejects_non_number() {
        let err = run(QValue::from(DataValue::Str("a".into()))).unwrap_err();
        assert_eq!(err.error_code(), error_codes::INVALID_UNARY_OPERAND);
    }
}
