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
use qlexpress::runtime::parameters::Parameters;
use qlexpress::runtime::qcontext::QContext;
use qlexpress::runtime::trace::TraceType;
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
/// Java `Express4RunnerTest#getExpressionTracePointsTest`。
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

    let in_operator = runner
        .get_expression_trace_points("'ab' in ['cc', 'dd', 'ff']")
        .expect("in 操作符追踪解析成功");
    assert_eq!(
        concat!(
            "OPERATOR in\n",
            "  | VALUE 'ab'\n",
            "  | LIST [\n",
            "      | VALUE 'cc'\n",
            "      | VALUE 'dd'\n",
            "      | VALUE 'ff'\n",
        ),
        in_operator[0].to_pretty_string(0)
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

#[test]
fn static_trace_covers_java_visitor_statement_and_expression_shapes() {
    let runner = Express4Runner::new();
    let cases = [
        ("throw 'boom';", TraceType::Statement),
        ("int value = 1;", TraceType::Statement),
        ("while (true) { break; }", TraceType::Statement),
        (
            "for (int i = 0; i < 1; i++) { continue; }",
            TraceType::Statement,
        ),
        ("for (item : [1]) { item; }", TraceType::Statement),
        (
            "function plusOne(x) { return x + 1; }",
            TraceType::DefineFunction,
        ),
        ("macro one { 1 }", TraceType::DefineMacro),
        ("return 1;", TraceType::Return),
        (";", TraceType::Statement),
        ("target = source + 1", TraceType::Operator),
        ("true ? 1 : 2", TraceType::Operator),
        ("++value", TraceType::Operator),
        ("[1, value, 3]", TraceType::List),
        ("{'a': 1}", TraceType::Map),
        ("{ 1; 2; }", TraceType::Block),
        ("if (true) { 1 } else { 2 }", TraceType::If),
        (
            "switch (value) { case 1 -> 10 default -> 20 }",
            TraceType::Switch,
        ),
        ("try { 1 } catch (error) { 2 }", TraceType::Primary),
        ("${ selected-value }", TraceType::Primary),
        ("(int)value", TraceType::Variable),
        ("(value)", TraceType::Variable),
        ("new int[2]", TraceType::Primary),
        ("new int[]{1, 2}", TraceType::Primary),
        ("service.call(1)[0].name", TraceType::Field),
    ];

    for (script, expected_type) in cases {
        let points = runner
            .get_expression_trace_points(script)
            .unwrap_or_else(|error| panic!("trace parse failed for {script:?}: {error:?}"));
        assert_eq!(
            points.len(),
            1,
            "one top-level trace point expected for {script:?}: {points:?}"
        );
        assert_eq!(
            points[0].trace_type(),
            expected_type,
            "unexpected trace type for {script:?}: {}",
            points[0].to_pretty_string(0)
        );
        assert!(!points[0].token().is_empty(), "token for {script:?}");
        assert!(points[0].line() >= 1, "line for {script:?}");
        assert!(points[0].position() >= 0, "position for {script:?}");
    }
}

/// 完整复刻 Java `Express4RunnerTest#expressionTraceTest` 的运行时断言。
#[test]
fn java_expression_trace_test_complete_contract() {
    let runner = trace_runner();
    assert!(runner.add_function(
        "myTest",
        |_context: &mut dyn QContext, parameters: &Parameters| {
            Ok(DataValue::Bool(matches!(
                parameters.get_value(0),
                DataValue::Int(value) if value > 10
            )))
        }
    ));
    let options = QLOptions::builder().trace_expression(true).build();

    let result = runner
        .execute(
            "a && (!myTest(11) || false)",
            HashMap::from([("a".to_string(), DataValue::Bool(true))]),
            &options,
        )
        .expect("primary trace");
    assert_eq!(result.result(), &DataValue::Bool(false));
    assert_eq!(result.expression_traces().len(), 1);
    assert_eq!(
        result.expression_traces()[0].to_pretty_string(0),
        concat!(
            "OPERATOR && false\n",
            "  | VARIABLE a true\n",
            "  | OPERATOR || false\n",
            "      | OPERATOR ! false\n",
            "          | FUNCTION myTest true\n",
            "              | VALUE 11 11\n",
            "      | VALUE false false\n",
        )
    );

    let short = runner
        .execute(
            "(a && true) && (!myTest(11) || false)",
            HashMap::from([("a".to_string(), DataValue::Bool(false))]),
            &options,
        )
        .expect("short-circuit trace");
    assert_eq!(
        short.expression_traces()[0].to_pretty_string(0),
        concat!(
            "OPERATOR && false\n",
            "  | OPERATOR && false\n",
            "      | VARIABLE a false\n",
            "      | VALUE true \n",
            "  | OPERATOR || \n",
            "      | OPERATOR ! \n",
            "          | FUNCTION myTest \n",
            "              | VALUE 11 \n",
            "      | VALUE false \n",
        )
    );
    assert!(short.expression_traces()[0].children()[0].is_evaluated());
    assert!(!short.expression_traces()[0].children()[1].is_evaluated());

    let in_result = runner
        .execute("'ab' in ['cc', 'dd', 'ff']", HashMap::new(), &options)
        .expect("in trace");
    assert_eq!(in_result.result(), &DataValue::Bool(false));
    assert_eq!(
        in_result.expression_traces()[0].to_pretty_string(0),
        concat!(
            "OPERATOR in false\n",
            "  | VALUE 'ab' ab\n",
            "  | LIST [ [cc, dd, ff]\n",
            "      | VALUE 'cc' cc\n",
            "      | VALUE 'dd' dd\n",
            "      | VALUE 'ff' ff\n",
        )
    );

    let ternary = runner
        .execute("true? 2: 1;false? 2: 1", HashMap::new(), &options)
        .expect("ternary traces");
    assert_eq!(
        ternary.expression_traces()[0].to_pretty_string(0),
        "OPERATOR ? 2\n  | VALUE true true\n  | VALUE 2 2\n  | VALUE 1 \n"
    );
    assert_eq!(
        ternary.expression_traces()[1].to_pretty_string(0),
        "OPERATOR ? 1\n  | VALUE false false\n  | VALUE 2 \n  | VALUE 1 1\n"
    );

    let if_result = runner
        .execute("if(true) {11} else {13}", HashMap::new(), &options)
        .expect("if trace");
    assert_eq!(
        if_result.expression_traces()[0].to_pretty_string(0),
        concat!(
            "IF if 11\n",
            "  | VALUE true true\n",
            "  | BLOCK { 11\n",
            "      | VALUE 11 11\n",
            "  | BLOCK { \n",
            "      | VALUE 13 \n",
        )
    );

    let assign = runner
        .execute("aab = 11", HashMap::new(), &options)
        .expect("new assignment trace");
    assert_eq!(
        assign.expression_traces()[0].to_pretty_string(0),
        "OPERATOR = 11\n  | VARIABLE aab null\n  | VALUE 11 11\n"
    );
    let assign_change = runner
        .execute(
            "aab = 111",
            HashMap::from([("aab".to_string(), DataValue::Int(100))]),
            &options,
        )
        .expect("existing assignment trace");
    assert_eq!(
        assign_change.expression_traces()[0].to_pretty_string(0),
        "OPERATOR = 111\n  | VARIABLE aab 100\n  | VALUE 111 111\n"
    );

    let function_field = runner
        .execute(
            "m = {bbb:6};aaa = () -> m;aaa().bbb=10;m.bbb",
            HashMap::new(),
            &options,
        )
        .expect("function field assignment trace");
    assert_eq!(function_field.result(), &DataValue::Int(10));
    assert_eq!(
        function_field.expression_traces()[0].to_pretty_string(0),
        "OPERATOR = {bbb=10}\n  | VARIABLE m null\n  | MAP { {bbb=10}\n"
    );
    assert_eq!(
        function_field.expression_traces()[2].to_pretty_string(0),
        concat!(
            "OPERATOR = 10\n",
            "  | FIELD bbb 6\n",
            "      | FUNCTION aaa \n",
            "  | VALUE 10 10\n",
        )
    );

    let block = runner
        .execute("a = {m=10;m+11}", HashMap::new(), &options)
        .expect("block trace");
    assert_eq!(
        block.expression_traces()[0].to_pretty_string(0),
        concat!(
            "OPERATOR = 21\n",
            "  | VARIABLE a null\n",
            "  | BLOCK { 21\n",
            "      | OPERATOR = 10\n",
            "          | VARIABLE m null\n",
            "          | VALUE 10 10\n",
            "      | OPERATOR + 21\n",
            "          | VARIABLE m 10\n",
            "          | VALUE 11 11\n",
        )
    );

    let nested_if = runner
        .execute(
            "if(false) {11} else if (1>10) {15} else {}",
            HashMap::new(),
            &options,
        )
        .expect("nested if trace");
    assert_eq!(
        nested_if.expression_traces()[0].to_pretty_string(0),
        concat!(
            "IF if null\n",
            "  | VALUE false false\n",
            "  | BLOCK { \n",
            "      | VALUE 11 \n",
            "  | IF if null\n",
            "      | OPERATOR > false\n",
            "          | VALUE 1 1\n",
            "          | VALUE 10 10\n",
            "      | BLOCK { \n",
            "          | VALUE 15 \n",
            "      | BLOCK { null\n",
        )
    );

    let statement_cases = [
        ("int a = 1;", "STATEMENT int null\n"),
        ("while(false) {m=10}", "STATEMENT while null\n"),
        ("for(int i=0; i<3; i++) {i}", "STATEMENT for null\n"),
        ("for(int item : [1,2,3]) {item}", "STATEMENT for null\n"),
        (
            "function testFunc() {return 10}",
            "DEFINE_FUNCTION testFunc null\n",
        ),
        (
            "macro testMacro {return 20}",
            "DEFINE_MACRO testMacro null\n",
        ),
        ("break", "STATEMENT break null\n"),
        ("continue", "STATEMENT continue null\n"),
    ];
    for (script, expected) in statement_cases {
        let traced = runner
            .execute(script, HashMap::new(), &options)
            .unwrap_or_else(|error| panic!("{script:?} failed: {error}"));
        assert_eq!(traced.expression_traces()[0].to_pretty_string(0), expected);
    }

    let throw_branch = runner
        .execute("if (true) 10 else throw 1", HashMap::new(), &options)
        .expect("throw branch trace");
    assert_eq!(
        throw_branch.expression_traces()[0].to_pretty_string(0),
        "IF if 10\n  | VALUE true true\n  | VALUE 10 10\n  | STATEMENT throw \n"
    );

    let function_condition = runner
        .execute("if (myTest(11)) 10 else 1", HashMap::new(), &options)
        .expect("function condition trace");
    assert_eq!(
        function_condition.expression_traces()[0].to_pretty_string(0),
        concat!(
            "IF if 10\n",
            "  | FUNCTION myTest true\n",
            "      | VALUE 11 11\n",
            "  | VALUE 10 10\n",
            "  | VALUE 1 \n",
        )
    );

    let returned = runner
        .execute("return 1+1", HashMap::new(), &options)
        .expect("return trace");
    assert_eq!(
        returned.expression_traces()[0].to_pretty_string(0),
        concat!(
            "RETURN return 2\n",
            "  | OPERATOR + 2\n",
            "      | VALUE 1 1\n",
            "      | VALUE 1 1\n",
        )
    );

    let empty = runner
        .execute(";;;;", HashMap::new(), &options)
        .expect("empty statement trace");
    assert_eq!(empty.expression_traces().len(), 1);
    assert_eq!(
        empty.expression_traces()[0].to_pretty_string(0),
        "STATEMENT ; \n"
    );
    assert_eq!(
        runner
            .execute("a=1;;;;", HashMap::new(), &options)
            .expect("filtered empty statements")
            .expression_traces()
            .len(),
        1
    );

    let switched = runner
        .execute(
            concat!(
                "switch (a + b) {\n",
                "  case 30:\n",
                "    result = a * 2;\n",
                "    break;\n",
                "  default:\n",
                "    result = 0;\n",
                "}\nreturn result;"
            ),
            HashMap::from([
                ("a".to_string(), DataValue::Int(1)),
                ("b".to_string(), DataValue::Int(29)),
            ]),
            &options,
        )
        .expect("switch trace");
    assert_eq!(switched.result(), &DataValue::Int(2));
    assert_eq!(
        switched.expression_traces()[0].to_pretty_string(0),
        concat!(
            "SWITCH switch null\n",
            "  | OPERATOR + 30\n",
            "      | VARIABLE a 1\n",
            "      | VARIABLE b 29\n",
            "  | VALUE 30 30\n",
            "  | BLOCK result null\n",
            "      | OPERATOR = 2\n",
            "          | VARIABLE result null\n",
            "          | OPERATOR * 2\n",
            "              | VARIABLE a 1\n",
            "              | VALUE 2 2\n",
            "      | STATEMENT break null\n",
            "  | BLOCK result \n",
            "      | OPERATOR = \n",
            "          | VARIABLE result \n",
            "          | VALUE 0 \n",
        )
    );
}
