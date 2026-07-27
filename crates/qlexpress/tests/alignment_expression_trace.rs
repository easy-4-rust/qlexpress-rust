//! 表达式追踪对齐测试。
//!
//! 用例来源：
//! - Java `Express4RunnerTest#getExpressionTracePointsTest`
//! - Java `Express4RunnerTest#expressionTraceTest`
//! - Java `SerializableParseCacheTest#tracePointsAreOptionalAndRoundTripWhenExported`

use std::collections::HashMap;
use std::rc::Rc;

use qlexpress::api::parsecache::SerializableParseCache;
use qlexpress::init_options::InitOptions;
use qlexpress::ql_options::QLOptions;
use qlexpress::runtime::context::EmptyContext;
use qlexpress::runtime::value::DataValue;
use qlexpress::Express4Runner;

fn trace_runner() -> Express4Runner {
    Express4Runner::with_init_options(InitOptions::builder().trace_expression(true).build())
}

fn trace_options() -> QLOptions {
    QLOptions::builder()
        .trace_expression(true)
        .cache(true)
        .build()
}

#[test]
fn static_trace_tree_matches_java_examples() {
    let runner = Express4Runner::new();
    let arithmetic = runner
        .get_expression_trace_points("1+3+5*ab+9")
        .expect("静态追踪解析成功");
    assert_eq!(1, arithmetic.len());
    assert_eq!(
        concat!(
            "OPERATOR +\n",
            "  | OPERATOR +\n",
            "      | OPERATOR +\n",
            "          | VALUE 1\n",
            "          | VALUE 3\n",
            "      | OPERATOR *\n",
            "          | VALUE 5\n",
            "          | VARIABLE ab\n",
            "  | VALUE 9\n",
        ),
        arithmetic[0].to_pretty_string(0)
    );

    let function = runner
        .get_expression_trace_points("ab && (myTest(1,2) || false)")
        .expect("函数追踪解析成功");
    assert_eq!(
        concat!(
            "OPERATOR &&\n",
            "  | VARIABLE ab\n",
            "  | OPERATOR ||\n",
            "      | FUNCTION myTest\n",
            "          | VALUE 1\n",
            "          | VALUE 2\n",
            "      | VALUE false\n",
        ),
        function[0].to_pretty_string(0)
    );
}

#[test]
fn runtime_trace_preserves_short_circuit_and_resets_cached_execution() {
    let runner = trace_runner();
    let options = trace_options();

    let first = runner
        .execute("false && true", HashMap::new(), &options)
        .expect("第一次执行成功");
    assert_eq!(&DataValue::Bool(false), first.result());
    assert_eq!(1, first.expression_traces().len());
    let first_root = &first.expression_traces()[0];
    assert!(first_root.is_evaluated());
    assert!(first_root.children()[0].is_evaluated());
    assert!(!first_root.children()[1].is_evaluated());

    let second = runner
        .execute("true && true", HashMap::new(), &options)
        .expect("第二次执行成功");
    let second_root = &second.expression_traces()[0];
    assert!(second_root.children()[0].is_evaluated());
    assert!(second_root.children()[1].is_evaluated());
}

#[test]
fn trace_points_round_trip_through_json_parse_cache() {
    let producer = trace_runner();
    let exported = producer
        .export_parse_cache("false && true")
        .expect("导出带追踪点的缓存");
    assert!(exported
        .trace_points
        .as_ref()
        .is_some_and(|points| !points.is_empty()));

    let json = serde_json::to_string(&exported).expect("序列化缓存");
    let parsed: SerializableParseCache = serde_json::from_str(&json).expect("反序列化缓存");

    let consumer = trace_runner();
    let result = consumer
        .execute_with_cache(
            &parsed,
            Rc::new(EmptyContext),
            &QLOptions::builder().trace_expression(true).build(),
        )
        .expect("执行导入缓存");
    assert_eq!(1, result.expression_traces().len());
    assert!(!result.expression_traces()[0].children()[1].is_evaluated());

    let plain = Express4Runner::new()
        .export_parse_cache("false && true")
        .expect("导出不带追踪点的缓存");
    assert!(plain.trace_points.is_none());
    let plain_result = consumer
        .execute_with_cache(
            &plain,
            Rc::new(EmptyContext),
            &QLOptions::builder().trace_expression(true).build(),
        )
        .expect("执行无追踪点缓存");
    assert!(plain_result.expression_traces().is_empty());
}
