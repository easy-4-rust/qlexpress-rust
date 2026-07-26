//! Stage 6 对齐测试:自定义操作符与自定义函数用例。
//!
//! 对应 Java: com.alibaba.qlexpress4.Express4RunnerTest 的
//! `addOperatorTest` / `docAddFunctionAndOperatorTest`。

// 与 lib 一致的架构性豁免:QLException 对齐 Java 单一异常类(见 lib.rs)。
#![allow(clippy::result_large_err)]

use std::collections::HashMap;
use std::rc::Rc;

use qlexpress_rust::ql_options::QLOptions;
use qlexpress_rust::runtime::value::{DataValue, QValue};
use qlexpress_rust::Express4Runner;

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
        |params: &[qlexpress_rust::runtime::value::DataValue]| -> Result<_, _> {
            let joined = (0..params.len())
                .map(|i| params[i].string_value_of())
                .collect::<Vec<_>>()
                .join(",");
            Ok(DataValue::Str(joined))
        },
    );
    assert_eq!(
        runner
            .execute("join(1,2,3)", HashMap::new(), &opts_no_cache())
            .unwrap()
            .into_result(),
        DataValue::Str("1,2,3".to_string())
    );
    // 自定义二元操作符 join(与函数同名)
    runner.add_operator_bi("join", |left: DataValue, right: DataValue| {
        DataValue::Str(format!(
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
        DataValue::Str("1,2,3".to_string())
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
        DataValue::Str("1.22.3".to_string())
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

/// 对应 Java `Express4RunnerTest#addOperatorTest` 的
/// addOperator("join") 与 addOperator(".*", GROUP) 分支。
#[test]
fn add_operator_with_group_precedence() {
    let mut runner = Express4Runner::new();
    runner.add_operator_bi("join", |left: DataValue, right: DataValue| {
        DataValue::Str(format!(
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
        DataValue::Str("1.22".to_string())
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
                                .get(&DataValue::Str(field.clone()))
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
        qlexpress_rust::ql_precedences::GROUP,
    );
    match runner
        .execute("[{a:1}, {a:5}].*a", HashMap::new(), &opts_no_cache())
        .unwrap()
        .into_result()
    {
        DataValue::List(list) => {
            assert_eq!(
                *list.borrow(),
                vec![DataValue::Int(1), DataValue::Int(5)]
            );
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
            assert_eq!(
                *list.borrow(),
                vec![DataValue::Int(5), DataValue::Int(10)]
            );
        }
        other => panic!("期望 List,实际为 {other:?}"),
    }
}
