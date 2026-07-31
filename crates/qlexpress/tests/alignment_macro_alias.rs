//! Stage 6 对齐测试:宏、别名与特殊函数名用例。
//!
//! 对应 Java: com.alibaba.qlexpress4.Express4RunnerTest 的
//! `addMacroTest` / `addAliasTest` / `inTest` / `atFunctionTest`。

// 与 lib 一致的架构性豁免:QLException 对齐 Java 单一异常类(见 lib.rs)。
#![allow(clippy::result_large_err)]

use std::collections::HashMap;

use qlexpress::ql_options::QLOptions;
use qlexpress::runtime::value::DataValue;
use qlexpress::Express4Runner;

fn opts() -> QLOptions {
    QLOptions::builder().build()
}

fn ctx(pairs: &[(&str, DataValue)]) -> HashMap<String, DataValue> {
    pairs
        .iter()
        .map(|(k, v)| (k.to_string(), v.clone()))
        .collect()
}

/// 对应 Java `Express4RunnerTest#addMacroTest`(addMacro +
/// addOrReplaceMacro 替换宏)。
#[test]
fn add_macro_and_replace() {
    let runner = Express4Runner::new();
    runner.add_macro("rename", "name='haha-'+name").unwrap();
    let context = ctx(&[("name", DataValue::Str("wuli".into()))]);
    assert_eq!(
        runner
            .execute("rename", context.clone(), &opts())
            .unwrap()
            .into_result(),
        DataValue::Str("haha-wuli".into())
    );
    // 替换宏定义
    runner
        .add_or_replace_macro("rename", "name='huhu-'+name")
        .unwrap();
    assert_eq!(
        runner
            .execute("rename", context, &opts())
            .unwrap()
            .into_result(),
        DataValue::Str("huhu-wuli".into())
    );
}

/// 对应 Java `Express4RunnerTest#addAliasTest`(关键字/操作符/函数别名)。
#[test]
fn add_alias() {
    let mut runner = Express4Runner::new();
    // 自定义函数 zero
    runner.add_function(
        "zero",
        |_ctx: &mut dyn qlexpress::runtime::qcontext::QContext,
         _params: &qlexpress::runtime::parameters::Parameters|
         -> Result<_, _> { Ok(DataValue::Int(0)) },
    );
    // 关键字别名
    assert!(runner.add_alias("如果", "if"));
    assert!(runner.add_alias("则", "then"));
    assert!(runner.add_alias("否则", "else"));
    assert!(runner.add_alias("返回", "return"));
    // 操作符别名
    assert!(runner.add_alias("大于", ">"));
    // 函数别名
    assert!(runner.add_alias("零", "zero"));

    let context = ctx(&[
        ("语文", DataValue::Int(90)),
        ("数学", DataValue::Int(90)),
        ("英语", DataValue::Int(90)),
    ]);
    let result = runner
        .execute(
            "如果 (语文 + 数学 + 英语 大于 270) 则 {返回 1;} 否则 {返回 零();}",
            context,
            &opts(),
        )
        .unwrap();
    assert_eq!(result.into_result(), DataValue::Int(0));
}

/// 对应 Java `Express4RunnerTest#inTest`(in 操作符别名)。
#[test]
fn in_operator_alias() {
    let mut runner = Express4Runner::new();
    runner.add_alias("属于", "in");
    assert_eq!(
        runner
            .execute("1 属于 [1,2]", HashMap::new(), &opts())
            .unwrap()
            .into_result(),
        DataValue::Bool(true)
    );
    assert_eq!(
        runner
            .execute("1 属于 [3,2]", HashMap::new(), &opts())
            .unwrap()
            .into_result(),
        DataValue::Bool(false)
    );
}

/// 对应 Java `Express4RunnerTest#atFunctionTest`(`@` 可作为函数名)。
#[test]
fn at_function_name() {
    let runner = Express4Runner::new();
    runner.add_function(
        "@",
        |_ctx: &mut dyn qlexpress::runtime::qcontext::QContext,
         params: &qlexpress::runtime::parameters::Parameters|
         -> Result<_, _> {
            match params.get_value(0) {
                DataValue::Str(s) => Ok(DataValue::string(format!("{s},{s}"))),
                other => Ok(other),
            }
        },
    );
    assert_eq!(
        runner
            .execute("@('a')", HashMap::new(), &opts())
            .unwrap()
            .into_result(),
        DataValue::Str("a,a".into())
    );
}
