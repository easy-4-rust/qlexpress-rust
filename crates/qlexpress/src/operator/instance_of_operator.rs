//! `instanceof` 操作符。
//!
//! 对应 Java: com.alibaba.qlexpress4.runtime.operator.InstanceOfOperator
//! (@author bingo;右操作数必须是 MetaClass/Class,左操作数 null 时
//! 结果为 false,语义为 `Class.isAssignableFrom`)。

use crate::exception::error_reporter::ErrorReporter;
use crate::exception::QLException;
use crate::ql_precedences;
use crate::runtime::member::{self, ClassRef};
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
        _q_context: &mut dyn QContext,
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
        Ok(DataValue::Bool(is_assignable_from(
            &class_ref,
            &source_object,
        )))
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

/// Java `Class.isAssignableFrom(source.getClass())` 的内建类型近似:
/// 同名类、`java.lang.Object`、`java.lang.Number` 接收各数值装箱类型、
/// `java.lang.Comparable`/`java.io.Serializable` 接收装箱类型与字符串;
/// 宿主对象按 `native_type_name` 精确匹配(对齐 cast.rs 的既有口径)。
fn is_assignable_from(target: &ClassRef, source: &DataValue) -> bool {
    let target_name = target.java_name();
    if target_name == "java.lang.Object" {
        return true;
    }
    let source_name = match source {
        DataValue::Object(obj) => obj.borrow().native_type_name().to_string(),
        other => other.data_type_name().to_string(),
    };
    if target_name == source_name {
        return true;
    }
    let is_boxed_number = matches!(
        source_name.as_str(),
        "java.lang.Byte"
            | "java.lang.Short"
            | "java.lang.Integer"
            | "java.lang.Long"
            | "java.lang.Float"
            | "java.lang.Double"
            | "java.math.BigInteger"
            | "java.math.BigDecimal"
    );
    match target_name {
        "java.lang.Number" => is_boxed_number,
        "java.lang.Comparable" | "java.io.Serializable" => {
            is_boxed_number
                || matches!(
                    source_name.as_str(),
                    "java.lang.String" | "java.lang.Boolean" | "java.lang.Character"
                )
        }
        // Java 集合接口层级:`new ArrayList() instanceof List` 为 true。
        // (对齐测试 extensionfunction/extension_function.ql 发现。)
        "java.util.List" | "java.util.Collection" | "java.lang.Iterable" => matches!(
            source_name.as_str(),
            "java.util.ArrayList" | "java.util.LinkedList" | "java.util.List"
        ),
        "java.util.Set" => matches!(
            source_name.as_str(),
            "java.util.HashSet" | "java.util.LinkedHashSet" | "java.util.TreeSet"
        ),
        "java.util.Map" => matches!(
            source_name.as_str(),
            "java.util.HashMap" | "java.util.LinkedHashMap" | "java.util.TreeMap"
        ),
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::exception::pure_err_reporter::PureErrReporter;
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
