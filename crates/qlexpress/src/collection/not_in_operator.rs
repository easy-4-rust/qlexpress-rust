//! `not_in` 操作符,对应 Java
//! `com.alibaba.qlexpress4.runtime.operator.collection.NotInOperator`。

use crate::exception::error_reporter::ErrorReporter;
use crate::exception::QLException;
use crate::ql_options::QLOptions;
use crate::ql_precedences;
use crate::runtime::operator::base::BinaryOperator;
use crate::runtime::qcontext::QContext;
use crate::runtime::value::{DataValue, QValue};

/// `not_in` 二元操作符,`in` 的逻辑取反。
///
/// 对应 Java: com.alibaba.qlexpress4.runtime.operator.collection.NotInOperator
/// (@author bingo;Java 实现为 `!in(left, right, errorReporter)`,包含判定
/// 继承自 `BaseBinaryOperator`)。
#[derive(Clone, Copy, Debug, Default)]
pub struct NotInOperator;

impl NotInOperator {
    /// 对应 Java `NotInOperator.getInstance()` 单例。
    pub fn get_instance() -> NotInOperator {
        NotInOperator
    }
}

impl BinaryOperator for NotInOperator {
    /// 对应 Java `NotInOperator.execute(Value left, Value right, QRuntime,
    /// QLOptions, ErrorReporter)`:返回 `!in(left, right, errorReporter)`。
    fn execute(
        &self,
        left: &QValue,
        right: &QValue,
        _q_context: &mut dyn QContext,
        _ql_options: &QLOptions,
        error_reporter: &dyn ErrorReporter,
    ) -> Result<DataValue, QLException> {
        // Java 语义要点:与 InOperator 共用同一套包含判定(Java 继承自
        // BaseBinaryOperator.in),仅结果取反;操作符名传 "not_in",
        // 保证报错信息与 Java 一致。
        Ok(DataValue::Bool(!super::in_operator::op_in(
            left,
            right,
            self.operator(),
            error_reporter,
        )?))
    }

    /// 对应 Java `getOperator()`:操作符词素 `"not_in"`。
    fn operator(&self) -> &str {
        "not_in"
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
    use crate::runtime::data::index_map::IndexMap;

    // 操作符从不读写 QContext,测试直接驱动 BinaryOperator 共用的 in 逻辑。
    fn run(left: DataValue, right: DataValue) -> Result<bool, QLException> {
        super::super::in_operator::op_in(
            &QValue::from(left),
            &QValue::from(right),
            "not_in",
            &PureErrReporter::INSTANCE,
        )
        .map(|contained| !contained)
    }

    #[test]
    fn not_in_is_negated_in() {
        let list = DataValue::list(vec![DataValue::Int(1), DataValue::Int(2)]);
        assert!(run(DataValue::Int(3), list.clone()).unwrap());
        assert!(!run(DataValue::Long(1), list).unwrap());
        // 字符串子串取反
        assert!(run(DataValue::Str("z".into()), DataValue::Str("abcd".into())).unwrap());
        // null 语义同样取反:null not_in null → false
        assert!(!run(DataValue::Null, DataValue::Null).unwrap());
    }

    #[test]
    fn not_in_map_is_invalid_operand() {
        // Java:Map 不是 Collection,与 in 一样抛 INVALID_BINARY_OPERAND
        let map = DataValue::map(IndexMap::from_entries(vec![(
            DataValue::Int(1),
            DataValue::Int(2),
        )]));
        let err = run(DataValue::Int(1), map).unwrap_err();
        assert_eq!(err.error_code(), error_codes::INVALID_BINARY_OPERAND);
    }
}
