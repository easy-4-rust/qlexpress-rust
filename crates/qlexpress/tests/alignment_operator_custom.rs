//! Stage 6 对齐测试:自定义操作符与自定义函数用例。
//!
//! 对应 Java: com.alibaba.qlexpress4.Express4RunnerTest 的
//! `addOperatorTest` / `docAddFunctionAndOperatorTest`。

// 与 lib 一致的架构性豁免:QLException 对齐 Java 单一异常类(见 lib.rs)。
#![allow(clippy::result_large_err)]

use std::collections::HashMap;
use std::rc::Rc;

use qlexpress::exception::error_codes;
use qlexpress::exception::ql_exception_kind::QLExceptionKind;
use qlexpress::exception::QLException;
use qlexpress::ql_options::QLOptions;
use qlexpress::runtime::value::{DataValue, QValue};
use qlexpress::Express4Runner;

fn opts_no_cache() -> QLOptions {
    QLOptions::builder().cache(false).build()
}

/// 对应 Java `Express4RunnerTest#docAddFunctionAndOperatorTest`
/// (varargs 函数 + 二元操作符同名注册)。
#[test]
fn doc_add_function_and_operator() {
    let mut runner = Express4Runner::new();
    // 自定义 varargs 函数 join
    runner.add_varargs_function(
        "join",
        |params: &[qlexpress::runtime::value::DataValue]| -> Result<_, _> {
            let joined = (0..params.len())
                .map(|i| params[i].string_value_of())
                .collect::<Vec<_>>()
                .join(",");
            Ok(DataValue::string(joined))
        },
    );
    assert_eq!(
        runner
            .execute("join(1,2,3)", HashMap::new(), &opts_no_cache())
            .unwrap()
            .into_result(),
        DataValue::Str("1,2,3".into())
    );
    // 自定义二元操作符 join(与函数同名)
    runner.add_operator_bi("join", |left: DataValue, right: DataValue| {
        DataValue::string(format!(
            "{},{}",
            left.string_value_of(),
            right.string_value_of()
        ))
    });
    assert_eq!(
        runner
            .execute("1 join 2 join 3", HashMap::new(), &opts_no_cache())
            .unwrap()
            .into_result(),
        DataValue::Str("1,2,3".into())
    );
}

/// 对应 Java `Express4RunnerTest#addOperatorTest` 的
/// replaceDefaultOperator 分支(替换默认 `+` 为数值相加)。
#[test]
fn replace_default_operator() {
    let mut runner = Express4Runner::new();
    // 默认:字符串拼接
    assert_eq!(
        runner
            .execute("'1.2'+'2.3'", HashMap::new(), &opts_no_cache())
            .unwrap()
            .into_result(),
        DataValue::Str("1.22.3".into())
    );
    let replaced = runner.replace_operator(
        "+",
        Rc::new(|left: &QValue, right: &QValue| {
            let l: f64 = left.get().string_value_of().parse().unwrap_or(f64::NAN);
            let r: f64 = right.get().string_value_of().parse().unwrap_or(f64::NAN);
            Ok(DataValue::Double(l + r))
        }),
    );
    assert!(replaced);
    assert_eq!(
        runner
            .execute("'1.2'+'2.3'", HashMap::new(), &opts_no_cache())
            .unwrap()
            .into_result(),
        DataValue::Double(3.5)
    );
}

/// 对应 Java `OperatorManager#adapt2BinOp` 的 `UserDefineException` 分支：
/// 参数错误不得被错误包装成 `OPERATOR_INNER_EXCEPTION`。
#[test]
fn custom_operator_invalid_argument_keeps_user_defined_error_category() {
    let mut runner = Express4Runner::new();
    assert!(runner.add_operator(
        "badarg",
        Rc::new(|_left: &QValue, _right: &QValue| {
            Err(QLException::for_test(
                QLExceptionKind::Runtime,
                "bad input",
                error_codes::INVALID_ARGUMENT,
            ))
        }),
    ));

    let error = runner
        .execute("1 badarg 2", HashMap::new(), &opts_no_cache())
        .expect_err("custom argument failure");
    assert_eq!(error.error_code(), error_codes::INVALID_ARGUMENT);
    assert_eq!(error.reason(), "bad input");
}

/// 对应 Java `OperatorManager#adapt2BinOp` 的默认 `UserDefineException`
/// 分支：业务错误同样不能落入内部异常包装。
#[test]
fn custom_operator_business_error_keeps_user_defined_error_category() {
    let mut runner = Express4Runner::new();
    assert!(runner.add_operator(
        "badbiz",
        Rc::new(|_left: &QValue, _right: &QValue| {
            Err(QLException::for_test(
                QLExceptionKind::Runtime,
                "business rejected",
                error_codes::BIZ_EXCEPTION,
            ))
        }),
    ));

    let error = runner
        .execute("1 badbiz 2", HashMap::new(), &opts_no_cache())
        .expect_err("custom business failure");
    assert_eq!(error.error_code(), error_codes::BIZ_EXCEPTION);
    assert_eq!(error.reason(), "business rejected");
}

/// 对应 Java `OperatorManager#adapt2BinOp` 的非 `UserDefineException` 分支：
/// 宿主异常必须在当前位置包装为 `OPERATOR_INNER_EXCEPTION`，并保留 cause。
#[test]
fn custom_operator_host_failure_is_wrapped_and_keeps_cause() {
    let mut runner = Express4Runner::new();
    assert!(runner.add_operator(
        "hostfail",
        Rc::new(|_left: &QValue, _right: &QValue| {
            Err(QLException::host_error(
                QLExceptionKind::Runtime,
                "host exploded",
                "HOST_FAILURE",
            ))
        }),
    ));

    let error = runner
        .execute("1 hostfail 2", HashMap::new(), &opts_no_cache())
        .expect_err("host custom operator failure");
    assert_eq!(error.error_code(), "OPERATOR_INNER_EXCEPTION");
    let cause = error.cause().expect("host error must remain the cause");
    assert_eq!(cause.error_code(), "HOST_FAILURE");
    assert_eq!(cause.reason(), "host exploded");
}

/// Java `ThrowUtils.wrapThrowable` 会把未包装的原生算术异常改报当前
/// 操作符的内部异常，不能以用户定义错误原样泄漏。
#[test]
fn custom_operator_arithmetic_failure_is_wrapped() {
    let mut runner = Express4Runner::new();
    assert!(runner.add_operator(
        "arithfail",
        Rc::new(|_left: &QValue, _right: &QValue| {
            Err(QLException::for_test(
                QLExceptionKind::Runtime,
                "division by zero",
                "ARITHMETIC_EXCEPTION",
            ))
        }),
    ));

    let error = runner
        .execute("1 arithfail 2", HashMap::new(), &opts_no_cache())
        .expect_err("arithmetic custom operator failure");
    assert_eq!(error.error_code(), "OPERATOR_INNER_EXCEPTION");
    assert!(error.cause().is_none());
}

/// 对应 Java `Express4RunnerTest#addOperatorTest` 的
/// addOperator("join") 与 addOperator(".*", GROUP) 分支。
#[test]
fn add_operator_with_group_precedence() {
    let mut runner = Express4Runner::new();
    runner.add_operator_bi("join", |left: DataValue, right: DataValue| {
        DataValue::string(format!(
            "{}{}",
            left.string_value_of(),
            right.string_value_of()
        ))
    });
    assert_eq!(
        runner
            .execute("1.2 join 2", HashMap::new(), &opts_no_cache())
            .unwrap()
            .into_result(),
        DataValue::Str("1.22".into())
    );

    // `.*` 投影操作符:取列表中每个 map 的指定字段。
    runner.add_operator_with_precedence(
        ".*",
        Rc::new(|left: &QValue, right: &QValue| {
            let field = right.get().string_value_of();
            match left.get() {
                DataValue::List(list) => {
                    let projected: Vec<DataValue> = list
                        .borrow()
                        .iter()
                        .map(|item| match item {
                            DataValue::Map(map) => map
                                .borrow()
                                .get(&DataValue::string(field.clone()))
                                .cloned()
                                .unwrap_or(DataValue::Null),
                            _ => DataValue::Null,
                        })
                        .collect();
                    Ok(DataValue::list(projected))
                }
                _ => Ok(DataValue::Null),
            }
        }),
        qlexpress::ql_precedences::GROUP,
    );
    match runner
        .execute("[{a:1}, {a:5}].*a", HashMap::new(), &opts_no_cache())
        .unwrap()
        .into_result()
    {
        DataValue::List(list) => {
            assert_eq!(*list.borrow(), vec![DataValue::Int(1), DataValue::Int(5)]);
        }
        other => panic!("期望 List,实际为 {other:?}"),
    }
    // 投影结果支持切片。
    match runner
        .execute(
            "[{a:1}, {a:5}, {a:10}, {a:20}].*a[1:-1]",
            HashMap::new(),
            &QLOptions::builder().build(),
        )
        .unwrap()
        .into_result()
    {
        DataValue::List(list) => {
            assert_eq!(*list.borrow(), vec![DataValue::Int(5), DataValue::Int(10)]);
        }
        other => panic!("期望 List,实际为 {other:?}"),
    }
}
