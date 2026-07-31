//! 可调用脚本 Lambda 值,对应 Java `com.alibaba.qlexpress4.runtime.QLambda`。
//! 职责:脚本 Lambda 的统一调用契约(`call` / `getFunctionDefined`)。
//! Java 为接口 + 实现类(`QLambdaEmpty`、`QLambdaInner`、方法引用 Lambda
//! `QLambdaMethod`);Rust 以枚举变体聚合这些实现类,变体语义与原类一一对应。
//! 本文件由 `qlambda.rs` 拆分而来(SPEC §5.5 一类一文件),仅移动代码与补充中文注释,行为完全一致。

use std::collections::HashMap;
use std::fmt;
use std::rc::Rc;

use crate::exception::QLException;
use crate::exception::error_codes;
use crate::exception::error_reporter::ErrorReporter;
use crate::exception::pure_err_reporter::PureErrReporter;
use crate::runtime::data::lambda::QLambdaMethod;
use crate::runtime::function::CustomFunction;
use crate::runtime::q_result::QResult;
use crate::runtime::qlambda_definition::QLambdaDefinition;
use crate::runtime::qlambda_empty::QLambdaEmpty;
use crate::runtime::qlambda_inner::QLambdaInner;
use crate::runtime::value::DataValue;

/// 可调用脚本 Lambda 值。对应 Java: com.alibaba.qlexpress4.runtime.QLambda(接口;
/// Rust 以枚举变体聚合其实现类 `QLambdaEmpty`/`QLambdaInner`/`QLambdaMethod`)
///
/// A callable script lambda value, mirroring the Java `QLambda` interface
/// and its implementations (`QLambdaEmpty`, `QLambdaInner`, and the
/// method-reference lambda `QLambdaMethod`).
pub enum QLambda {
    /// 空 Lambda:调用返回 `QResult.NEXT_INSTRUCTION`。
    /// 对应 Java `QLambdaEmpty.INSTANCE`(负载为其 Rust 对应类型 [`QLambdaEmpty`])。
    Empty(QLambdaEmpty),
    /// 指令序列 Lambda。对应 Java `QLambdaInner`。
    Inner(QLambdaInner),
    /// 对象方法作为 Lambda。对应 Java `data/lambda/QLambdaMethod`。
    Method(QLambdaMethod),
}

impl QLambda {
    /// 调用 Lambda。对应 Java 方法 `QLambda.call(Object... params)`。
    /// Java `QLambda.call(Object... params)`.
    pub fn call(&self, params: &[DataValue]) -> Result<QResult, QLException> {
        match self {
            QLambda::Empty(_) => Ok(QResult::NEXT_INSTRUCTION),
            QLambda::Inner(inner) => inner.call(params),
            QLambda::Method(method) => method.call(params),
        }
    }

    /// 调用 Lambda 并返回其内部定义的函数表。对应 Java 方法
    /// `QLambda.getFunctionDefined(Object... params)`。
    /// Java `QLambda.getFunctionDefined(Object... params)`.
    pub fn function_defined(
        &self,
        params: &[DataValue],
    ) -> Result<HashMap<String, Rc<dyn CustomFunction>>, QLException> {
        match self {
            QLambda::Inner(inner) => inner.function_defined(params),
            _ => Ok(HashMap::new()),
        }
    }

    /// 无参数调用 Lambda 并返回其结果值。
    ///
    /// 对应 Java：`QLambda#get()`。
    ///
    /// # 返回值
    /// 返回 Lambda 的结果；不携带结果的控制流返回 `null`。
    ///
    /// # 错误
    /// 返回 Lambda 执行期间产生的原始错误。
    pub fn get(&self) -> Result<DataValue, QLException> {
        Ok(self.call(&[])?.value())
    }

    /// 使用一个参数调用 Lambda，并丢弃结果值。
    ///
    /// 对应 Java：`QLambda#accept(Object)`。
    ///
    /// # 参数
    /// - `o`：传给 Lambda 的参数。
    ///
    /// # 错误
    /// 返回 Lambda 执行期间产生的原始错误。
    pub fn accept(&self, o: &DataValue) -> Result<(), QLException> {
        self.call(std::slice::from_ref(o))?;
        Ok(())
    }

    /// 无参数调用 Lambda，并丢弃结果值。
    ///
    /// 对应 Java：`QLambda#run()`。
    ///
    /// # 错误
    /// 返回 Lambda 执行期间产生的原始错误。
    pub fn run(&self) -> Result<(), QLException> {
        self.call(&[])?;
        Ok(())
    }

    /// 使用一个参数调用 Lambda，并将结果强制读取为布尔值。
    ///
    /// 对应 Java：`QLambda#test(Object)`；Java 对非 `Boolean` 结果执行强制
    /// 类型转换并抛出 `ClassCastException`，Rust 以稳定的类型转换错误返回。
    ///
    /// # 参数
    /// - `o`：传给 Lambda 的参数。
    ///
    /// # 返回值
    /// 返回 Lambda 产生的布尔值。
    ///
    /// # 错误
    /// - [`QLException`]：Lambda 执行失败，或结果不是布尔值。
    pub fn test(&self, o: &DataValue) -> Result<bool, QLException> {
        let value = self.call(std::slice::from_ref(o))?.value();
        value.as_bool().ok_or_else(|| {
            PureErrReporter::INSTANCE.report_format(
                error_codes::INCOMPATIBLE_TYPE_CAST,
                error_codes::error_msg(error_codes::INCOMPATIBLE_TYPE_CAST),
                &[
                    value.data_type_name().to_string(),
                    "java.lang.Boolean".to_string(),
                ],
            )
        })
    }

    /// 使用一个参数调用 Lambda 并返回结果值。
    ///
    /// 对应 Java：`QLambda#apply(Object)`。
    ///
    /// # 参数
    /// - `o`：传给 Lambda 的参数。
    ///
    /// # 返回值
    /// 返回 Lambda 的结果；不携带结果的控制流返回 `null`。
    ///
    /// # 错误
    /// 返回 Lambda 执行期间产生的原始错误。
    pub fn apply(&self, o: &DataValue) -> Result<DataValue, QLException> {
        Ok(self.call(std::slice::from_ref(o))?.value())
    }
}

impl fmt::Debug for QLambda {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            QLambda::Empty(_) => write!(f, "QLambdaEmpty"),
            QLambda::Inner(inner) => f
                .debug_struct("QLambdaInner")
                .field("name", &inner.lambda_definition.name())
                .field("params", &inner.lambda_definition.params_type())
                .field("new_env", &inner.new_env)
                .finish(),
            QLambda::Method(method) => write!(f, "QLambdaMethod({})", method.method_name()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runtime::member::NativeRegistry;

    fn method_lambda(method_name: &str, bean: DataValue) -> QLambda {
        QLambda::Method(QLambdaMethod::new(
            method_name,
            Rc::new(NativeRegistry::with_builtins()),
            bean,
        ))
    }

    /// `SOURCE_PARITY`：Java `QLambda#get()` 与 `QLambda#run()` 都执行一次
    /// 无参数调用；前者返回值，后者只丢弃返回值。
    #[test]
    fn supplier_and_runnable_defaults_preserve_java_call_contract() {
        let lambda = method_lambda("isEmpty", DataValue::string(String::new()));

        assert_eq!(lambda.get().expect("supplier call"), DataValue::Bool(true));
        lambda.run().expect("runnable call");
    }

    /// `SOURCE_PARITY`：Java `Consumer.accept` 丢弃结果，而
    /// `Function.apply` 返回同一次单参数调用的结果。
    #[test]
    fn consumer_and_function_defaults_preserve_java_result_contract() {
        let lambda = method_lambda("equals", DataValue::string("same"));
        let argument = DataValue::string("same");

        lambda.accept(&argument).expect("consumer call");
        assert_eq!(
            lambda.apply(&argument).expect("function call"),
            DataValue::Bool(true)
        );
    }

    /// `SOURCE_PARITY`：Java `Predicate.test` 只接受 `Boolean` 返回值。
    #[test]
    fn predicate_default_returns_boolean_and_rejects_other_types() {
        let predicate = method_lambda("equals", DataValue::string("same"));
        assert!(
            predicate
                .test(&DataValue::string("same"))
                .expect("boolean predicate")
        );
        assert!(
            !predicate
                .test(&DataValue::string("other"))
                .expect("false predicate")
        );

        let non_boolean = method_lambda("substring", DataValue::string("value"));
        let error = non_boolean
            .test(&DataValue::Int(1))
            .expect_err("predicate result must be boolean");
        assert_eq!(error.error_code(), error_codes::INCOMPATIBLE_TYPE_CAST);
        assert_eq!(
            error.reason(),
            "incompatible cast from type: java.lang.String to type: java.lang.Boolean"
        );
    }

    /// `RUST_OBLIGATION`：Rust `Result` 适配必须保留底层 Lambda 的精确错误，
    /// 不得把它吞掉或替换为成功的 `null`。
    #[test]
    fn functional_defaults_propagate_lambda_errors() {
        let lambda = method_lambda("methodThatDoesNotExist", DataValue::string("value"));
        let error = lambda.get().expect_err("missing method must propagate");
        assert_eq!(error.error_code(), error_codes::INVALID_ARGUMENT);
    }
}
