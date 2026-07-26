//! Stage 6 对齐测试:字符串插值与模板引擎用例。
//!
//! 对应 Java: com.alibaba.qlexpress4.Express4RunnerTest 的
//! `interpolationTest` / `templateEngineTest` / `docTryCatchTest`。

// 与 lib 一致的架构性豁免:QLException 对齐 Java 单一异常类(见 lib.rs)。
#![allow(clippy::result_large_err)]

use std::collections::HashMap;

use qlexpress_rust::aparser::interpolation_mode::InterpolationMode;
use qlexpress_rust::init_options::InitOptions;
use qlexpress_rust::ql_options::QLOptions;
use qlexpress_rust::runtime::value::DataValue;
use qlexpress_rust::Express4Runner;

fn opts() -> QLOptions {
    QLOptions::builder().build()
}

fn ctx(pairs: &[(&str, DataValue)]) -> HashMap<String, DataValue> {
    pairs
        .iter()
        .map(|(k, v)| (k.to_string(), v.clone()))
        .collect()
}

fn run_with(script: &str, context: HashMap<String, DataValue>) -> DataValue {
    Express4Runner::new()
        .execute(script, context, &opts())
        .unwrap_or_else(|err| panic!("execute failed for {script:?}: {err:?}"))
        .into_result()
}

/// 对应 Java `Express4RunnerTest#interpolationTest`(插值基本行为)。
#[test]
fn interpolation_basic() {
    let result = run_with("\"Hello,${a+1}\"", ctx(&[("a", DataValue::Int(1))]));
    assert_eq!(result, DataValue::Str("Hello,2".to_string()));
}

/// 对应 Java `Express4RunnerTest#interpolationTest` 的
/// disableInterpolation 分支(关闭插值后 `${...}` 原样输出)。
/// `InterpolationMode::Disable` 仅抑制 `${...}` 表达式插值;字符串字面量内部
/// 的转义(`\n` `\b`)仍按 Java 字符串字面量规则解析,因此期望里写
/// 真实的换行字符 `"\n"` 和退格 `"\u{8}"`,而不是字面的反斜杠。
#[test]
fn interpolation_disable() {
    let runner = Express4Runner::with_init_options(
        InitOptions::builder()
            .interpolation_mode(InterpolationMode::Disable)
            .build(),
    );
    let context = ctx(&[("a", DataValue::Int(1))]);
    // `${ ... }` 在 disable 模式下原样保留。
    assert_eq!(
        runner
            .execute("\"Hello,${ a + 1 }\"", context.clone(), &opts())
            .unwrap()
            .into_result(),
        DataValue::Str("Hello,${ a + 1 }".to_string())
    );
    // 字符串内未闭合的 `${` 也原样保留。
    assert_eq!(
        runner
            .execute("\"Hello,${lll\"", context.clone(), &opts())
            .unwrap()
            .into_result(),
        DataValue::Str("Hello,${lll".to_string())
    );
    // disable 模式下脚本本身含 `\n \b`:Java 端会按转义解析为真换行 + 真退格,
    // Rust 端同样处理。期望写真实字符。
    assert_eq!(
        runner
            .execute(r#""Hello,aaa $ lll\"\n\b""#, context, &opts())
            .unwrap()
            .into_result(),
        DataValue::Str("Hello,aaa $ lll\"\n\u{8}".to_string())
    );
}

/// 对应 Java `Express4RunnerTest#templateEngineTest`。
#[test]
fn template_engine() {
    let runner = Express4Runner::new();
    let context = ctx(&[
        ("a", DataValue::Int(1)),
        ("b", DataValue::Int(2)),
        ("c", DataValue::Str("test".to_string())),
    ]);
    assert_eq!(
        runner
            .execute_template("a ${a};b ${b+2}", context.clone(), &opts())
            .unwrap()
            .into_result(),
        DataValue::Str("a 1;b 4".to_string())
    );
    assert_eq!(
        runner
            .execute_template(
                "m xx ${\n  if (c like 't%') {\n      'YYY'\n  }\n}",
                context.clone(),
                &opts()
            )
            .unwrap()
            .into_result(),
        DataValue::Str("m xx YYY".to_string())
    );
    assert_eq!(
        runner
            .execute_template("m\n ${a}\n c", context.clone(), &opts())
            .unwrap()
            .into_result(),
        DataValue::Str("m\n 1\n c".to_string())
    );
    assert_eq!(
        runner
            .execute_template("m \n\"haha\" d\"", context, &opts())
            .unwrap()
            .into_result(),
        DataValue::Str("m \n\"haha\" d\"".to_string())
    );
}

/// 对应 Java `Express4RunnerTest#docTryCatchTest`(try-catch 作为表达式)。
#[test]
fn doc_try_catch_as_expr() {
    let result = Express4Runner::new()
        .execute(
            "1 + try {\n  100 + 1/0\n} catch(e) {\n  11\n}",
            HashMap::new(),
            &opts(),
        )
        .unwrap();
    assert_eq!(result.into_result(), DataValue::Int(12));
}
