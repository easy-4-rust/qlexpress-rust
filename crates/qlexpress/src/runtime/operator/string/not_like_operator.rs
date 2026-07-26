//! `not_like` 操作符,对应 Java
//! `com.alibaba.qlexpress4.runtime.operator.string.NotLikeOperator`。

use crate::exception::error_reporter::ErrorReporter;
use crate::exception::QLException;
use crate::ql_options::QLOptions;
use crate::ql_precedences;
use crate::runtime::operator::base::BinaryOperator;
use crate::runtime::qcontext::QContext;
use crate::runtime::value::{DataValue, QValue};

/// `not_like` 二元操作符,`like` 的逻辑取反。
///
/// 对应 Java: com.alibaba.qlexpress4.runtime.operator.string.NotLikeOperator
/// (@author bingo;Java 实现为 `!like(left, right, errorReporter)`,匹配算法
/// 继承自 `BaseBinaryOperator`)。
#[derive(Clone, Copy, Debug, Default)]
pub struct NotLikeOperator;

impl NotLikeOperator {
    /// 对应 Java `NotLikeOperator.getInstance()` 单例。
    pub fn get_instance() -> NotLikeOperator {
        NotLikeOperator
    }
}

impl BinaryOperator for NotLikeOperator {
    /// 对应 Java `NotLikeOperator.execute(Value left, Value right, QRuntime,
    /// QLOptions, ErrorReporter)`:返回 `!like(left, right, errorReporter)`。
    fn execute(
        &self,
        left: &QValue,
        right: &QValue,
        _q_context: &mut dyn QContext,
        _ql_options: &QLOptions,
        error_reporter: &dyn ErrorReporter,
    ) -> Result<DataValue, QLException> {
        // Java 语义要点:与 LikeOperator 共用同一套 null/类型检查与匹配
        // 逻辑(Java 继承自 BaseBinaryOperator.like),仅结果取反;操作符名
        // 传 "not_like",保证报错信息与 Java 一致。
        Ok(DataValue::Bool(!super::like_operator::like(
            left,
            right,
            self.operator(),
            error_reporter,
        )?))
    }

    /// 对应 Java `getOperator()`:操作符词素 `"not_like"`。
    fn operator(&self) -> &str {
        "not_like"
    }

    /// 对应 Java `getPriority()`:`QLPrecedences.IN_LIKE`。
    fn priority(&self) -> i32 {
        ql_precedences::IN_LIKE
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::exception::error_codes;
    use crate::exception::pure_err_reporter::PureErrReporter;

    // 操作符从不读写 QContext,测试直接驱动 BinaryOperator 共用的 like 逻辑。
    fn run(left: DataValue, right: DataValue) -> Result<bool, QLException> {
        super::super::like_operator::like(
            &QValue::from(left),
            &QValue::from(right),
            "not_like",
            &PureErrReporter::INSTANCE,
        )
        .map(|matched| !matched)
    }

    #[test]
    fn not_like_is_negated_like() {
        assert!(run(DataValue::Str("abc".into()), DataValue::Str("a%d".into())).unwrap());
        assert!(!run(DataValue::Str("abc".into()), DataValue::Str("a%c".into())).unwrap());
        // null 语义同样取反:null not_like null → false
        assert!(!run(DataValue::Null, DataValue::Null).unwrap());
        assert!(run(DataValue::Null, DataValue::Str("x".into())).unwrap());
    }

    #[test]
    fn not_like_non_string_operand_rejected() {
        let err = run(DataValue::Int(1), DataValue::Str("1".into())).unwrap_err();
        assert_eq!(err.error_code(), error_codes::INVALID_BINARY_OPERAND);
    }
}
