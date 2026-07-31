//! `instanceof` 操作符。
//!
//! 对应 Java: com.alibaba.qlexpress4.runtime.operator.InstanceOfOperator
//! (@author bingo;右操作数必须是 MetaClass/Class,左操作数 null 时
//! 结果为 false,语义为 `Class.isAssignableFrom`)。

use crate::exception::error_reporter::ErrorReporter;
use crate::exception::QLException;
use crate::ql_precedences;
use crate::runtime::member;
use crate::runtime::qcontext::QContext;
use crate::runtime::value::{DataValue, QValue};

use crate::runtime::operator::base::base_binary_operator::BaseBinaryOperator;
use crate::runtime::operator::binary_operator::BinaryOperator;

/// `instanceof` 操作符。
///
/// 对应 Java: InstanceOfOperator(单例模式,`getInstance()`)。
pub struct InstanceOfOperator;

impl InstanceOfOperator {
    /// Java `getInstance()` 单例获取(无状态,直接构造)。
    pub fn get_instance() -> Self {
        InstanceOfOperator
    }
}

impl BinaryOperator for InstanceOfOperator {
    /// 对应 Java 方法: `execute(Value left, Value right, QRuntime, QLOptions,
    /// ErrorReporter)`。
    ///
    /// 语义要点(逐行对齐 Java):
    /// 1. 右操作数为 null → INVALID_BINARY_OPERAND;
    /// 2. 右操作数是 MetaClass 则取其 clz(Java `instanceof MetaClass`);
    ///    否则(非 Class)→ INVALID_BINARY_OPERAND;
    /// 3. 左操作数为 null → false(Java `null instanceof X` 恒 false);
    /// 4. `targetClass.isAssignableFrom(sourceObject.getClass())`。
    fn execute(
        &self,
        left: &QValue,
        right: &QValue,
        q_context: &mut dyn QContext,
        _ql_options: &crate::ql_options::QLOptions,
        error_reporter: &dyn ErrorReporter,
    ) -> Result<DataValue, QLException> {
        let source_object = left.get();
        let target_class = right.get();
        if target_class.is_null() {
            return Err(BaseBinaryOperator::build_invalid_operand_type_exception(
                "instanceof",
                left,
                right,
                error_reporter,
            ));
        }
        // Java: MetaClass → getClz();非 Class → 非法操作数。
        // Rust SPEC §4 下类字面量只有 MetaClass 一种载体。
        let Some(class_ref) = member::as_meta_class(&target_class) else {
            return Err(BaseBinaryOperator::build_invalid_operand_type_exception(
                "instanceof",
                left,
                right,
                error_reporter,
            ));
        };
        if source_object.is_null() {
            return Ok(DataValue::Bool(false));
        }
        Ok(DataValue::Bool(
            q_context
                .registry()
                .is_value_assignable(&class_ref, &source_object),
        ))
    }

    /// 对应 Java 方法: `getOperator()` —— `"instanceof"`。
    fn operator(&self) -> &str {
        "instanceof"
    }

    /// 对应 Java 方法: `getPriority()` —— QLPrecedences.COMPARE。
    fn priority(&self) -> i32 {
        ql_precedences::COMPARE
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::exception::pure_err_reporter::PureErrReporter;
    use crate::runtime::member::ClassRef;
    use crate::runtime::member::MetaClass;

    #[test]
    fn instanceof_semantics() {
        let int_class = MetaClass::new(ClassRef::from_name("java.lang.Integer")).into_data_value();
        let number_class =
            MetaClass::new(ClassRef::from_name("java.lang.Number")).into_data_value();
        // 1 instanceof Integer == true。
        assert_eq!(
            ctx_assert_check(DataValue::Int(1), int_class.clone()).unwrap(),
            DataValue::Bool(true)
        );
        // Integer instanceof Number == true(父类可赋值)。
        assert_eq!(
            ctx_assert_check(DataValue::Int(1), number_class).unwrap(),
            DataValue::Bool(true)
        );
        // null instanceof X == false。
        assert_eq!(
            ctx_assert_check(DataValue::Null, int_class.clone()).unwrap(),
            DataValue::Bool(false)
        );
        // 右操作数非类 → 报错。
        assert!(ctx_assert_check(DataValue::Int(1), DataValue::Int(2)).is_err());
        // 右操作数为 null → 报错。
        assert!(ctx_assert_check(DataValue::Int(1), DataValue::Null).is_err());
    }

    fn ctx_assert_check(l: DataValue, r: DataValue) -> Result<DataValue, QLException> {
        let op = InstanceOfOperator::get_instance();
        let mut ctx = crate::runtime::delegate_qcontext::DelegateQContext::new(
            std::rc::Rc::new(crate::runtime::qvm_runtime::QvmRuntime::for_test(
                std::rc::Rc::new(crate::runtime::member::NativeRegistry::with_builtins()),
            )),
            crate::runtime::scope::QScope::global(
                crate::runtime::qvm_global_scope::QvmGlobalScope::empty(),
            ),
        );
        op.execute(
            &QValue::Data(l),
            &QValue::Data(r),
            &mut ctx,
            &crate::ql_options::QLOptions::builder().build(),
            &PureErrReporter::INSTANCE,
        )
    }
}
