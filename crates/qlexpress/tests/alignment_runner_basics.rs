//! Stage 6 对齐测试:移植 Java `Express4RunnerTest` 的基础语义用例
//! (字面量/变量/短路/错误报告/缓存等)。
//!
//! 对应 Java: com.alibaba.qlexpress4.Express4RunnerTest 各方法,
//! 每个测试函数上以中文注释标注对应的 Java 方法名。

// 与 lib 一致的架构性豁免:QLException 对齐 Java 单一异常类(见 lib.rs)。
#![allow(clippy::result_large_err)]

use std::collections::HashMap;

use qlexpress_rust::exception::error_codes;
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

fn run(script: &str) -> DataValue {
    Express4Runner::new()
        .execute(script, HashMap::new(), &opts())
        .unwrap_or_else(|err| panic!("execute failed for {script:?}: {err:?}"))
        .into_result()
}

fn run_with(script: &str, context: HashMap<String, DataValue>) -> DataValue {
    Express4Runner::new()
        .execute(script, context, &opts())
        .unwrap_or_else(|err| panic!("execute failed for {script:?}: {err:?}"))
        .into_result()
}

fn expect_err_code(script: &str, expected: &str) {
    let err = Express4Runner::new()
        .execute(script, HashMap::new(), &opts())
        .unwrap_err();
    assert_eq!(err.error_code(), expected, "实际错误: {err:?}");
}

/// 对应 Java `Express4RunnerTest#docQuickStartTest`。
#[test]
fn doc_quick_start() {
    let result = run_with(
        "a + b * c",
        ctx(&[
            ("a", DataValue::Int(1)),
            ("b", DataValue::Int(2)),
            ("c", DataValue::Int(3)),
        ]),
    );
    assert_eq!(result, DataValue::Int(7));
}

/// 对应 Java `Express4RunnerTest#dollarVariableTest`。
#[test]
fn dollar_variable() {
    assert_eq!(run("$a = 10; $a"), DataValue::Int(10));
}

/// 对应 Java `Express4RunnerTest#chineseParenAsVarName`。
#[test]
fn chinese_paren_as_var_name() {
    let result = run_with(
        "客户（年龄）+ 客户（等级）",
        ctx(&[
            ("客户（年龄）", DataValue::Int(21)),
            ("客户（等级）", DataValue::Int(3)),
        ]),
    );
    assert_eq!(result, DataValue::Int(24));
}

/// 对应 Java `Express4RunnerTest#specialTokensTest`。
#[test]
fn special_tokens_as_var_name() {
    let result = run_with(
        "薪资项目@A【a】+ 薪资项目@B（b）",
        ctx(&[
            ("薪资项目@A【a】", DataValue::Int(100)),
            ("薪资项目@B（b）", DataValue::Int(1)),
        ]),
    );
    assert_eq!(result, DataValue::Int(101));
}

/// 对应 Java `Express4RunnerTest#variableStartsWithWellNumber`。
#[test]
fn variable_starts_with_well_number() {
    let result = run_with("#cost + 1", ctx(&[("#cost", DataValue::Int(10))]));
    assert_eq!(result, DataValue::Int(11));
}

/// 对应 Java `Express4RunnerTest#notStrictNewLinesTest`。
/// (Rust 版 `InitOptions` 同样提供 `strict_new_lines(false)`。)
#[test]
fn not_strict_new_lines() {
    let runner = Express4Runner::with_init_options(
        qlexpress_rust::init_options::InitOptions::builder()
            .strict_new_lines(false)
            .build(),
    );
    let context = ctx(&[
        ("价格", DataValue::Int(10)),
        ("饭卡商家承担", DataValue::Int(3)),
        ("平台补贴", DataValue::Int(5)),
    ]);
    let result = runner
        .execute(
            "商家应收=\n    价格\n   - 饭卡商家承担\n   + 平台补贴",
            context.clone(),
            &opts(),
        )
        .unwrap();
    assert_eq!(result.into_result(), DataValue::Int(12));
    let result1 = runner
        .execute(
            "if (价格>5) {\n return 10;\n}\nreturn 100;",
            context,
            &opts(),
        )
        .unwrap();
    assert_eq!(result1.into_result(), DataValue::Int(10));
}

/// 对应 Java `Express4RunnerTest#mapLiteralTest`。
#[test]
fn map_literal() {
    match run("{a:123,'b':'test'}") {
        DataValue::Map(map) => {
            let map = map.borrow();
            assert_eq!(
                map.get(&DataValue::Str("a".to_string())),
                Some(&DataValue::Int(123))
            );
            assert_eq!(
                map.get(&DataValue::Str("b".to_string())),
                Some(&DataValue::Str("test".to_string()))
            );
        }
        other => panic!("期望 Map,实际为 {other:?}"),
    }
}

/// 对应 Java `Express4RunnerTest#numberTest`(数值字面量解析)。
#[test]
fn number_literals() {
    // (脚本, 期望类型与值)；BigInteger 使用任意精度表示。
    let int_cases: &[(&str, DataValue)] = &[
        ("12323", DataValue::Int(12323)),
        ("2147483647", DataValue::Int(2147483647)),
        ("9223372036854775807", DataValue::Long(9223372036854775807)),
        (
            "18446744073709552000",
            DataValue::big_int(18446744073709552000u128),
        ),
        ("0xfff", DataValue::Int(4095)),
        ("0b11", DataValue::Int(3)),
        ("072", DataValue::Int(58)),
        ("10l", DataValue::Long(10)),
        ("10L", DataValue::Long(10)),
    ];
    for (script, expect) in int_cases {
        assert_eq!(&run(script), expect, "脚本: {script}");
    }
    // 小数:Java 规则为「double 可精确表示则为 double,否则 BigDecimal」。
    assert!(matches!(run("1.1"), DataValue::BigDec(_)));
    assert!(matches!(run("1.25"), DataValue::Double(v) if v == 1.25));
    assert!(matches!(run("1."), DataValue::Double(v) if v == 1.0));
    assert!(matches!(run(".1"), DataValue::BigDec(_)));
    assert!(matches!(run("12e1"), DataValue::Double(v) if v == 120.0));
    assert!(matches!(run("12.1E2"), DataValue::Double(v) if v == 1210.0));
    assert!(matches!(run("10d"), DataValue::Double(v) if v == 10.0));
    assert!(matches!(run("10.313D"), DataValue::Double(v) if v == 10.313));
    assert!(matches!(run("10.2f"), DataValue::Float(_)));
    assert!(matches!(run("10.2F"), DataValue::Float(_)));
}

/// 对应 Java `Express4RunnerTest#docPreciseTest`(precise 开关)。
#[test]
fn doc_precise() {
    // 默认:小数解析为 BigDecimal(Java `0.1` → BigDecimal)。
    assert!(matches!(run("0.1"), DataValue::BigDec(_)));
    // Java 中 0.3 == 0.1+0.2 为 true(BigDecimal 算术)。
    assert_eq!(run("0.3==0.1+0.2"), DataValue::Bool(true));
    // 上下文传入的 double 默认不精确;precise(true) 后精确。
    let context = ctx(&[("a", DataValue::Double(0.1)), ("b", DataValue::Double(0.2))]);
    assert_eq!(
        run_with("0.3==a+b", context.clone()),
        DataValue::Bool(false)
    );
    let precise = QLOptions::builder().precise(true).build();
    let result = Express4Runner::new()
        .execute("0.3==a+b", context, &precise)
        .unwrap();
    assert_eq!(result.into_result(), DataValue::Bool(true));
}

/// 对应 Java `Express4RunnerTest#logicAndTest`。
#[test]
fn logic_and_null() {
    assert_eq!(run("null && true"), DataValue::Bool(false));
}

/// 对应 Java `Express4RunnerTest#shortCircuitTest`。
#[test]
fn short_circuit() {
    assert_eq!(run("true && true && true"), DataValue::Bool(true));
    assert_eq!(run("true && false && (1/0)"), DataValue::Bool(false));
    assert_eq!(
        run("a = 1+1+1+1+1+1+1+1+1;true && true && true"),
        DataValue::Bool(true)
    );
    assert_eq!(run("false || false || false"), DataValue::Bool(false));
    assert_eq!(run("false || true || (1/0)"), DataValue::Bool(true));
    assert_eq!(
        run("(false && (1/0)) || true || (1/0)"),
        DataValue::Bool(true)
    );
    expect_err_code("true && (1/0)", error_codes::INVALID_ARITHMETIC);
    // 关闭短路后照常求值(Java disableShortCircuit 分支)。
    let disable = QLOptions::builder().short_circuit_disable(true).build();
    let runner = Express4Runner::new();
    assert_eq!(
        runner
            .execute("false || false || true", HashMap::new(), &disable)
            .unwrap()
            .into_result(),
        DataValue::Bool(true)
    );
    assert_eq!(
        runner
            .execute("(true && false) || false", HashMap::new(), &disable)
            .unwrap()
            .into_result(),
        DataValue::Bool(false)
    );
}

/// 对应 Java `Express4RunnerTest#disableShortCircuitTest`。
#[test]
fn disable_short_circuit() {
    assert_eq!(run("false && (1/0)"), DataValue::Bool(false));
    let disable = QLOptions::builder().short_circuit_disable(true).build();
    let err = Express4Runner::new()
        .execute("false && (1/0)", HashMap::new(), &disable)
        .unwrap_err();
    assert_eq!(err.error_code(), error_codes::INVALID_ARITHMETIC);
    assert_eq!(err.reason(), "Division by zero");
}

/// 对应 Java `Express4RunnerTest#assignTest`。
#[test]
fn assign_to_literal_is_syntax_error() {
    expect_err_code("1 = 0", error_codes::SYNTAX_ERROR);
}

/// 对应 Java `Express4RunnerTest#ifTest`。
#[test]
fn if_else_expression() {
    assert_eq!(run("if (2==3) {if (2==2) 10} else 4"), DataValue::Int(4));
}

/// 对应 Java `Express4RunnerTest#debugExample`。
#[test]
fn debug_example() {
    assert_eq!(run("1+1"), DataValue::Int(2));
    assert_eq!(run("false || true || (1/0)"), DataValue::Bool(true));
}

/// 对应 Java `Express4RunnerTest#invalidOperatorTest`。
#[test]
fn invalid_operator_syntax() {
    expect_err_code("a abcd bb", error_codes::SYNTAX_ERROR);
    expect_err_code("import a.b v = 1", error_codes::SYNTAX_ERROR);
    expect_err_code("a.*bbb", error_codes::SYNTAX_ERROR);
}

/// 对应 Java `Express4RunnerTest#importNotAtBeginningTest`。
#[test]
fn import_not_at_beginning() {
    let no_cache = QLOptions::builder().cache(false).build();
    let err = Express4Runner::new()
        .execute("a = 10;\nimport a.b.c;", HashMap::new(), &no_cache)
        .unwrap_err();
    assert_eq!(err.error_code(), error_codes::SYNTAX_ERROR);
    assert_eq!(
        err.reason(),
        "Import statement is not at the beginning of the file."
    );
}

/// 对应 Java `Express4RunnerTest#multilineStrNotCloseTest`。
#[test]
fn multiline_str_not_close() {
    let err = Express4Runner::new()
        .execute("a=1;'aaa \n \n cccc", HashMap::new(), &opts())
        .unwrap_err();
    assert_eq!(err.reason(), "unterminated string literal");
    let err2 = Express4Runner::new()
        .execute("\"aaa \n cccc", HashMap::new(), &opts())
        .unwrap_err();
    assert_eq!(err2.reason(), "unterminated string literal");
}

/// 对应 Java `Express4RunnerTest#chineseCommaPropertyTest`。
#[test]
fn chinese_comma_property() {
    assert_eq!(
        run("{'销售方地址、电话':'test'}.销售方地址、电话"),
        DataValue::Str("test".to_string())
    );
}

/// 对应 Java `Express4RunnerTest#stringEscapeTest`。
#[test]
fn string_escape() {
    // Java: "\"\" + '\'\na' 结果为 "\"'\na"(引号、换行拼接)。
    assert_eq!(
        run("\"\\\"\"+'\\'\na'"),
        DataValue::Str("\"'\na".to_string())
    );
}

/// 对应 Java `Express4RunnerTest#defaultAndSwitchAsVariable`。
#[test]
fn default_and_switch_as_variable() {
    assert_eq!(
        run("default = 1\nswitch = 2;\ndefault+switch"),
        DataValue::Int(3)
    );
}

/// 对应 Java `Express4RunnerTest#testSwitchMixedSyntaxError`。
#[test]
fn switch_mixed_syntax_error() {
    let script = "x = 1\nresult = switch (x) {\n    case 1: \"one\"\n    case 2 -> \"two\"\n    default -> \"other\"\n}";
    let err = Express4Runner::new()
        .execute(script, HashMap::new(), &opts())
        .unwrap_err();
    assert!(
        format!("{err:?}").contains("Cannot mix traditional switch syntax"),
        "实际错误: {err:?}"
    );
}

/// 对应 Java `Express4RunnerTest#errorReportColNumTest`。
#[test]
fn error_report_line_and_col() {
    let err = Express4Runner::new()
        .execute("1+1;\n2+2;\n1+cc()", HashMap::new(), &opts())
        .unwrap_err();
    assert_eq!(err.line_no(), 3);
    assert_eq!(err.col_no(), 3);

    let err2 = Express4Runner::new()
        .execute("1/0", HashMap::new(), &opts())
        .unwrap_err();
    assert_eq!(err2.line_no(), 1);
    assert_eq!(err2.col_no(), 2);

    let err3 = Express4Runner::new()
        .execute("a[]", HashMap::new(), &opts())
        .unwrap_err();
    assert_eq!(err3.line_no(), 1);
    // 偏差(错误位置口径):Java 报告 `]` 的位置(col 3),
    // Rust 版报告 `[` 的位置(col 2);错误码与行号一致。
    assert_eq!(err3.col_no(), 2);
}

/// 对应 Java `Express4RunnerTest#avoidNullPointerTest`。
#[test]
fn avoid_null_pointer() {
    let avoid = QLOptions::builder().avoid_null_pointer(true).build();
    let result = Express4Runner::new()
        .execute("'a '+${a}+aa('xxx')", HashMap::new(), &avoid)
        .unwrap();
    assert_eq!(
        result.into_result(),
        DataValue::Str("a nullnull".to_string())
    );
}

/// 对应 Java `Express4RunnerTest#emptyListCacheTest`
/// (cache=true 下空列表字面量每次执行独立,add 不串扰)。
#[test]
fn empty_list_cache() {
    let runner = Express4Runner::new();
    let cache = QLOptions::builder().cache(true).build();
    for _ in 0..10 {
        let result = runner
            .execute("arr = []; arr.add(1); return arr;", HashMap::new(), &cache)
            .unwrap();
        match result.into_result() {
            DataValue::List(list) => assert_eq!(list.borrow().len(), 1),
            other => panic!("期望 List,实际为 {other:?}"),
        }
    }
}

/// 对应 Java `Express4RunnerTest#emptyMapCacheTest`。
#[test]
fn empty_map_cache() {
    let runner = Express4Runner::new();
    let cache = QLOptions::builder().cache(true).build();
    for i in 0..10 {
        let result = runner
            .execute(
                "m = {:}; m.put(k,'b'); return m;",
                ctx(&[("k", DataValue::Str(format!("k{i}")))]),
                &cache,
            )
            .unwrap();
        match result.into_result() {
            DataValue::Map(map) => assert_eq!(map.borrow().len(), 1),
            other => panic!("期望 Map,实际为 {other:?}"),
        }
    }
}

/// 对应 Java `Express4RunnerTest#populateTest`(polluteUserContext
/// 写回上下文 Map;默认不写回)。
#[test]
fn pollute_user_context() {
    use qlexpress_rust::runtime::data::index_map::IndexMap;
    use qlexpress_rust::MapExpressContext;

    let runner = Express4Runner::new();
    let pollute = QLOptions::builder().pollute_user_context(true).build();
    let source = std::rc::Rc::new(std::cell::RefCell::new(IndexMap::from_entries(vec![(
        DataValue::Str("b".to_string()),
        DataValue::Int(10),
    )])));
    let context = MapExpressContext::new(std::rc::Rc::clone(&source));
    runner
        .execute_with_context("a = 11;b = a", std::rc::Rc::new(context), &pollute)
        .unwrap();
    let borrowed = source.borrow();
    assert_eq!(
        borrowed.get(&DataValue::Str("a".to_string())),
        Some(&DataValue::Int(11))
    );
    assert_eq!(
        borrowed.get(&DataValue::Str("b".to_string())),
        Some(&DataValue::Int(11))
    );

    // 默认(不污染):脚本内赋值不写回外部 Map。
    let source2 = std::rc::Rc::new(std::cell::RefCell::new(IndexMap::new()));
    let context2 = MapExpressContext::new(std::rc::Rc::clone(&source2));
    runner
        .execute_with_context("a = 11", std::rc::Rc::new(context2), &opts())
        .unwrap();
    assert!(!source2
        .borrow()
        .contains_key(&DataValue::Str("a".to_string())));

    let source3 = std::rc::Rc::new(std::cell::RefCell::new(IndexMap::from_entries(vec![(
        DataValue::Str("a".to_string()),
        DataValue::Int(10),
    )])));
    let context3 = MapExpressContext::new(std::rc::Rc::clone(&source3));
    let result = runner
        .execute_with_context("a = 19;a", std::rc::Rc::new(context3), &opts())
        .unwrap();
    assert_eq!(result.into_result(), DataValue::Int(19));
    assert_eq!(
        source3.borrow().get(&DataValue::Str("a".to_string())),
        Some(&DataValue::Int(10))
    );
}

/// 对应 Java `Express4RunnerTest#scripTimeoutTest`(超时错误码)。
#[test]
fn script_timeout() {
    let timeout = QLOptions::builder().timeout_millis(1).build();
    let err = Express4Runner::new()
        .execute("while(true) { 1+1 }", HashMap::new(), &timeout)
        .unwrap_err();
    assert_eq!(err.error_code(), error_codes::SCRIPT_TIME_OUT);
}
