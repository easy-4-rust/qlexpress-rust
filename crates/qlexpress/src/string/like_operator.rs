//! `like` 操作符,对应 Java
//! `com.alibaba.qlexpress4.runtime.operator.string.LikeOperator`。

use crate::exception::error_codes;
use crate::exception::error_reporter::ErrorReporter;
use crate::exception::QLException;
use crate::ql_options::QLOptions;
use crate::ql_precedences;
use crate::runtime::operator::base::BinaryOperator;
use crate::runtime::qcontext::QContext;
use crate::runtime::value::{DataValue, QValue};

/// `like` 二元操作符(SQL LIKE 风格,仅 `%` 通配)。
///
/// 对应 Java: com.alibaba.qlexpress4.runtime.operator.string.LikeOperator
/// (Author: DQinYuan;语义主体在其父类 `BaseBinaryOperator.like` 与私有方法
/// `matchPattern` 中,本文件自带与 Java 对等的完整逻辑)。
#[derive(Clone, Copy, Debug, Default)]
pub struct LikeOperator;

impl LikeOperator {
    /// 对应 Java `LikeOperator.getInstance()` 单例。
    pub fn get_instance() -> LikeOperator {
        LikeOperator
    }
}

impl BinaryOperator for LikeOperator {
    /// 对应 Java `LikeOperator.execute(Value left, Value right, QRuntime,
    /// QLOptions, ErrorReporter)`:委托给 `like(left, right, errorReporter)`。
    fn execute(
        &self,
        left: &QValue,
        right: &QValue,
        _q_context: &mut dyn QContext,
        _ql_options: &QLOptions,
        error_reporter: &dyn ErrorReporter,
    ) -> Result<DataValue, QLException> {
        Ok(DataValue::Bool(like(
            left,
            right,
            self.operator(),
            error_reporter,
        )?))
    }

    /// 对应 Java `getOperator()`:操作符词素 `"like"`。
    fn operator(&self) -> &str {
        "like"
    }

    /// 对应 Java `getPriority()`:`QLPrecedences.IN_LIKE`。
    fn priority(&self) -> i32 {
        ql_precedences::IN_LIKE
    }
}

/// 对应 Java `BaseBinaryOperator.like(Value, Value, ErrorReporter)`。
///
/// Java 语义要点:
/// - 两侧均为 `null` → `true`;仅一侧为 `null` → `false`;
/// - 任一侧不是 String → 抛 `INVALID_BINARY_OPERAND`;
/// - 否则按 `matchPattern` 做 SQL LIKE 风格匹配(只有 `%` 是通配符,
///   不是正则,`_` 没有特殊含义)。
pub(crate) fn like(
    left: &QValue,
    right: &QValue,
    operator: &str,
    error_reporter: &dyn ErrorReporter,
) -> Result<bool, QLException> {
    let target = left.get();
    let pattern = right.get();
    // Java:target == null && pattern == null → true
    if target.is_null() && pattern.is_null() {
        return Ok(true);
    }
    // Java:target == null || pattern == null → false
    if target.is_null() || pattern.is_null() {
        return Ok(false);
    }

    let (target, pattern) = match (target.as_str(), pattern.as_str()) {
        (Some(t), Some(p)) => (t.to_string(), p.to_string()),
        // Java:!(target instanceof String) || !(pattern instanceof String)
        _ => {
            return Err(build_invalid_operand_type_exception(
                left,
                right,
                operator,
                error_reporter,
            ))
        }
    };
    Ok(match_pattern(&target, &pattern))
}

/// 对应 Java `BaseBinaryOperator.matchPattern(String s, String pattern)`。
///
/// 带回溯的单通配符(`%`)匹配:`%` 匹配任意长度(含 0)字符序列;
/// 其余字符(包括 `_`)按字面量匹配。算法与 Java 逐行一致:
/// 失配时回退到最近一个 `%` 并让其多吞一个字符。
fn match_pattern(s: &str, pattern: &str) -> bool {
    // Java 以 UTF-16 code unit 索引;Rust 用 char 序列,语义一致(BMP 内)。
    let s: Vec<char> = s.chars().collect();
    let pattern: Vec<char> = pattern.chars().collect();
    let mut s_pointer = 0usize;
    let mut p_pointer = 0usize;
    let s_len = s.len();
    let p_len = pattern.len();
    // Java 用 -1 表示无回溯点;此处用 Option 表达。
    let mut s_recall: Option<usize> = None;
    let mut p_recall: Option<usize> = None;
    while s_pointer < s_len {
        if p_pointer < p_len && s[s_pointer] == pattern[p_pointer] {
            s_pointer += 1;
            p_pointer += 1;
        } else if p_pointer < p_len && pattern[p_pointer] == '%' {
            s_recall = Some(s_pointer);
            p_recall = Some(p_pointer);
            p_pointer += 1;
        } else if let (Some(sr), Some(pr)) = (s_recall, p_recall) {
            // 回溯:最近一个 `%` 多吞一个字符
            let next = sr + 1;
            s_recall = Some(next);
            s_pointer = next;
            p_pointer = pr + 1;
        } else {
            return false;
        }
    }
    while p_pointer < p_len && pattern[p_pointer] == '%' {
        p_pointer += 1;
    }
    p_pointer == p_len
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

    // 操作符从不读写 QContext,测试直接驱动核心逻辑函数 `like`。
    fn run(left: DataValue, right: DataValue) -> Result<bool, QLException> {
        like(
            &QValue::from(left),
            &QValue::from(right),
            "like",
            &PureErrReporter::INSTANCE,
        )
    }

    #[test]
    fn like_percent_wildcard() {
        // Java 对齐:`%` 通配任意长度
        assert!(run(DataValue::Str("abc".into()), DataValue::Str("a%c".into())).unwrap());
        assert!(!run(DataValue::Str("abc".into()), DataValue::Str("a%d".into())).unwrap());
        assert!(run(DataValue::Str("abc".into()), DataValue::Str("%".into())).unwrap());
        assert!(run(DataValue::Str("".into()), DataValue::Str("%".into())).unwrap());
        assert!(run(DataValue::Str("abc".into()), DataValue::Str("abc".into())).unwrap());
        // `_` 不是通配符(SQL LIKE 风格,非正则)
        assert!(!run(DataValue::Str("abc".into()), DataValue::Str("a_c".into())).unwrap());
    }

    #[test]
    fn like_null_semantics() {
        assert!(run(DataValue::Null, DataValue::Null).unwrap());
        assert!(!run(DataValue::Null, DataValue::Str("x".into())).unwrap());
        assert!(!run(DataValue::Str("x".into()), DataValue::Null).unwrap());
    }

    #[test]
    fn like_non_string_operand_rejected() {
        let err = run(DataValue::Int(1), DataValue::Str("1".into())).unwrap_err();
        assert_eq!(err.error_code(), error_codes::INVALID_BINARY_OPERAND);
    }
}
