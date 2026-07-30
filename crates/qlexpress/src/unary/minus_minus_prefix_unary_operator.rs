//! 前缀 `--` 操作符,对应 Java
//! `com.alibaba.qlexpress4.runtime.operator.unary.MinusMinusPrefixUnaryOperator`。

use crate::exception::error_codes;
use crate::exception::error_reporter::ErrorReporter;
use crate::exception::QLException;
use crate::ql_precedences;
use crate::runtime::data::convert::to_f64;
use crate::runtime::operator::base::UnaryOperator;
use crate::runtime::value::{DataValue, QValue};

/// 前缀 `--` 操作符。
///
/// 对应 Java: com.alibaba.qlexpress4.runtime.operator.unary.MinusMinusPrefixUnaryOperator
/// (@author bingo;执行体委托 `NumberMath.subtract(operand, 1)`,错误构造
/// 继承自 `BaseUnaryOperator`)。
///
/// 注意:Java 原文的返回值是**自减前的 `operand`**(`return operand;`,
/// 与后缀 `--` 的写法互换,疑似 Java 版笔误)。按「以 Java 源码为唯一语义
/// 参照」原样保留该行为。
#[derive(Clone, Copy, Debug, Default)]
pub struct MinusMinusPrefixUnaryOperator;

impl MinusMinusPrefixUnaryOperator {
    /// 对应 Java `MinusMinusPrefixUnaryOperator.getInstance()` 单例。
    pub fn get_instance() -> MinusMinusPrefixUnaryOperator {
        MinusMinusPrefixUnaryOperator
    }
}

impl UnaryOperator for MinusMinusPrefixUnaryOperator {
    /// 对应 Java `MinusMinusPrefixUnaryOperator.execute(Value value,
    /// ErrorReporter)`:
    /// ```java
    /// Object operand = value.get();
    /// if (!(operand instanceof Number)) throw ...;
    /// if (value instanceof LeftValue)
    ///     ((LeftValue)value).set(NumberMath.subtract((Number)operand, 1), errorReporter);
    /// return operand;
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

        // Java:value instanceof LeftValue → set(NumberMath.subtract(operand, 1))
        if let Some(left_value) = value.as_left() {
            left_value
                .borrow_mut()
                .set(number_sub_one(&operand), error_reporter)?;
        }
        // Java 原文:return operand(自减前的原值——与 ++ 前缀不同,见类注释)
        Ok(operand)
    }

    /// 对应 Java `getOperator()`:操作符词素 `"--"`。
    fn operator(&self) -> &str {
        "--"
    }

    /// 对应 Java `getPriority()`:`QLPrecedences.UNARY`。
    fn priority(&self) -> i32 {
        ql_precedences::UNARY
    }
}

/// 对应 Java `NumberMath.subtract(operand, 1)`(右操作数为 int 常量 1,
/// 经 `getMath(left, right)` 提升矩阵分发):
/// - `IntegerMath.subtractImpl`:`intValue() - 1` → **Byte/Short/Int 结果
///   为 Int**;
/// - `LongMath.subtractImpl`:Long - 1;
/// - `BigIntegerMath.subtractImpl`:BigInteger - 1;
/// - `BigDecimalMath.subtractImpl`:BigDecimal - 1(十进制字符串实现);
/// - `FloatingPointMath.subtractImpl`:`doubleValue() - 1` → **Float 结果
///   为 Double**。
///
/// `pub(crate)`:Java 中该逻辑经 `NumberMath` 被前缀/后缀 `--` 共享,
/// Rust 侧由后缀操作符复用本函数。
/// 对应 Java: com.alibaba.qlexpress4.runtime.operator.unary.MinusMinusPrefixUnaryOperator#numberSubOne。
pub(crate) fn number_sub_one(operand: &DataValue) -> DataValue {
    match operand {
        DataValue::Byte(v) => DataValue::Int((*v as i32).wrapping_sub(1)),
        DataValue::Short(v) => DataValue::Int((*v as i32).wrapping_sub(1)),
        DataValue::Int(v) => DataValue::Int(v.wrapping_sub(1)),
        DataValue::Long(v) => DataValue::Long(v.wrapping_sub(1)),
        DataValue::BigInt(v) => DataValue::BigInt(v - 1),
        DataValue::BigDec(v) => DataValue::BigDec(big_dec_sub_one(v)),
        DataValue::Float(_) | DataValue::Double(_) => DataValue::Double(to_f64(operand) - 1.0),
        // 调用点已保证 is_number,其余类型不可达
        _ => unreachable!("-- on non-number"),
    }
}

/// 十进制字符串减一,对应 Java `BigDecimal.subtract(BigDecimal.ONE)` 在
/// 字符串存储上的等价实现:`x - 1 = -(-x + 1)`。
fn big_dec_sub_one(dec: &str) -> String {
    negate_dec(&add_one_dec(&negate_dec(dec)))
}

/// 十进制字符串取负(零值不带负号)。
fn negate_dec(dec: &str) -> String {
    let (negative, int_part, frac_part) = split_dec(dec);
    let is_zero = int_part == "0" && frac_part.bytes().all(|b| b == b'0');
    if is_zero {
        return render_dec(false, int_part, frac_part);
    }
    render_dec(!negative, int_part, frac_part)
}

/// 十进制字符串加一(算法同 `++` 操作符中的实现):
/// ±1 只影响整数部分(小数位不变),唯一特殊情形是负值且整数部分为 0
/// (如 `-0.25 + 1 = 0.75`),按 `1 - 0.frac` 补位计算。
fn add_one_dec(dec: &str) -> String {
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
        MinusMinusPrefixUnaryOperator::get_instance().execute(&value, &PureErrReporter::INSTANCE)
    }

    #[test]
    fn prefix_minus_minus_writes_back_but_returns_original() {
        // Java 原文 return operand:--a 槽内变 1,但表达式值仍是自减前的 2
        let slot = Rc::new(RefCell::new(AssignableDataValue::new(
            "a",
            DataValue::Int(2),
        )));
        let result = run(QValue::Left(slot.clone())).unwrap();
        assert_eq!(result, DataValue::Int(2));
        assert_eq!(slot.borrow().get(), DataValue::Int(1));
    }

    #[test]
    fn prefix_minus_minus_promotes_byte_to_int() {
        // Java IntegerMath.subtractImpl:intValue() - 1
        let slot = Rc::new(RefCell::new(AssignableDataValue::new(
            "a",
            DataValue::Byte(1),
        )));
        run(QValue::Left(slot.clone())).unwrap();
        assert_eq!(slot.borrow().get(), DataValue::Int(0));
        // Long 保持 Long(写回值)
        let slot = Rc::new(RefCell::new(AssignableDataValue::new(
            "a",
            DataValue::Long(1),
        )));
        run(QValue::Left(slot.clone())).unwrap();
        assert_eq!(slot.borrow().get(), DataValue::Long(0));
    }

    #[test]
    fn prefix_minus_minus_big_decimal_sub_one() {
        // Java BigDecimalMath:subtract(ONE);写回值为减一结果
        let slot = Rc::new(RefCell::new(AssignableDataValue::new(
            "a",
            DataValue::BigDec("1.50".into()),
        )));
        run(QValue::Left(slot.clone())).unwrap();
        assert_eq!(slot.borrow().get(), DataValue::BigDec("0.50".into()));
        let slot = Rc::new(RefCell::new(AssignableDataValue::new(
            "a",
            DataValue::BigDec("0.25".into()),
        )));
        run(QValue::Left(slot.clone())).unwrap();
        assert_eq!(slot.borrow().get(), DataValue::BigDec("-0.75".into()));
        let slot = Rc::new(RefCell::new(AssignableDataValue::new(
            "a",
            DataValue::BigDec("-0.5".into()),
        )));
        run(QValue::Left(slot.clone())).unwrap();
        assert_eq!(slot.borrow().get(), DataValue::BigDec("-1.5".into()));
    }

    #[test]
    fn prefix_minus_minus_rejects_non_number() {
        let err = run(QValue::from(DataValue::Str("a".into()))).unwrap_err();
        assert_eq!(err.error_code(), error_codes::INVALID_UNARY_OPERAND);
    }
}
