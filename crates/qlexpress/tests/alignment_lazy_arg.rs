//! Java 惰性参数函数对齐测试。
//!
//! 来源：
//! - `Express4RunnerTest#testLazyArgCustomFunction`
//! - `Express4RunnerTest#testLazyArgCustomFunctionNoArgs`
//! - Java 修复 `f02f9d4f`：同一脚本内每次惰性调用必须使用唯一作用域名。

#![allow(clippy::result_large_err)]

use std::collections::HashMap;

use qlexpress::exception::error_codes;
use qlexpress::exception::ql_exception::QLExceptionKind;
use qlexpress::exception::QLException;
use qlexpress::ql_options::QLOptions;
use qlexpress::runtime::function::{CustomFunction, LazyArgCustomFunction};
use qlexpress::runtime::parameters::Parameters;
use qlexpress::runtime::qcontext::QContext;
use qlexpress::runtime::qvm_runtime::current_time_millis;
use qlexpress::runtime::value::DataValue;
use qlexpress::Express4Runner;

struct LazyIf;

impl LazyIf {
    fn evaluate(value: DataValue) -> Result<DataValue, QLException> {
        match value {
            DataValue::Lambda(lambda) => Ok(lambda.call(&[])?.value()),
            value => Ok(value),
        }
    }
}

impl CustomFunction for LazyIf {
    fn call(
        &self,
        _q_context: &mut dyn QContext,
        parameters: &Parameters,
    ) -> Result<DataValue, QLException> {
        if parameters.size() != 3 {
            return Err(QLException::for_test(
                QLExceptionKind::Runtime,
                "Invalid number of arguments",
                error_codes::INVALID_ARGUMENT,
            ));
        }
        let condition = Self::evaluate(parameters.get_value(0))?;
        match condition {
            DataValue::Bool(true) => Self::evaluate(parameters.get_value(1)),
            DataValue::Bool(false) => Self::evaluate(parameters.get_value(2)),
            _ => Err(QLException::for_test(
                QLExceptionKind::Runtime,
                "Argument 1 must be a boolean",
                error_codes::INVALID_ARGUMENT,
            )),
        }
    }

    fn as_lazy_arg(&self) -> Option<&dyn LazyArgCustomFunction> {
        Some(self)
    }
}

impl LazyArgCustomFunction for LazyIf {
    fn is_lazy_arg(&self, arg_index: usize) -> bool {
        arg_index == 1 || arg_index == 2
    }
}

struct CurrentTime;

impl CustomFunction for CurrentTime {
    fn call(
        &self,
        _q_context: &mut dyn QContext,
        _parameters: &Parameters,
    ) -> Result<DataValue, QLException> {
        Ok(DataValue::Long(current_time_millis()))
    }

    fn as_lazy_arg(&self) -> Option<&dyn LazyArgCustomFunction> {
        Some(self)
    }
}

impl LazyArgCustomFunction for CurrentTime {}

fn runner() -> Express4Runner {
    let runner = Express4Runner::new();
    assert!(runner.add_function("IF", LazyIf));
    runner
}

fn execute(
    runner: &Express4Runner,
    script: &str,
    context: HashMap<String, DataValue>,
) -> DataValue {
    runner
        .execute(script, context, &QLOptions::builder().build())
        .unwrap_or_else(|error| panic!("script failed: {script}: {error}"))
        .into_result()
}

#[test]
fn lazy_if_evaluates_only_selected_branch() {
    let runner = runner();
    let context = HashMap::from([
        ("a".to_string(), DataValue::Long(10_000)),
        ("b".to_string(), DataValue::Long(0)),
        ("c".to_string(), DataValue::Long(20)),
    ]);

    assert_eq!(
        execute(&runner, "IF(b == 0, 0, a / b)", context.clone()),
        DataValue::Long(0)
    );
    assert_eq!(
        execute(&runner, "IF(c != 0, a / c, 0)", context.clone()),
        DataValue::Long(500)
    );
    assert_eq!(
        execute(&runner, "IF(false, 0, IF(true, 1, 0))", context),
        DataValue::Long(1)
    );
}

#[test]
fn repeated_and_nested_lazy_calls_have_independent_scopes() {
    let runner = runner();
    let context = HashMap::from([
        ("a".to_string(), DataValue::Null),
        ("b".to_string(), DataValue::Long(0)),
    ]);

    assert_eq!(
        execute(
            &runner,
            "IF(false, 0, IF(true, IF(true, IF(true, IF(true, 1, 0), 0), 0), 0))",
            context.clone(),
        ),
        DataValue::Long(1)
    );
    assert_eq!(
        execute(
            &runner,
            "IF(true, 1, 0) + IF(true, 1, 0) + IF(false, IF(true, 1, 0), 0)",
            context.clone(),
        ),
        DataValue::Long(2)
    );
    assert_eq!(
        execute(
            &runner,
            "function func(x){ x++; return x+b; } IF(true, func(0), 0)",
            context,
        ),
        DataValue::Long(1)
    );
}

#[test]
fn lazy_function_with_no_arguments_executes_normally() {
    let runner = Express4Runner::new();
    assert!(runner.add_function("CURRENT_TIME", CurrentTime));
    let value = execute(&runner, "CURRENT_TIME()", HashMap::new());
    assert!(matches!(value, DataValue::Long(timestamp) if timestamp > 0));
}
