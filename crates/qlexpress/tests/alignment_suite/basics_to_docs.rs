/// 确保 Java 独立语言脚本既作为 Rust fixture 保存，也确实被本对齐套件的
/// 可执行测试源码逐项采用。
///
/// Java 的 `TestSuiteRunner#suiteTest` 执行 `testsuite/independent` 的 151 个
/// 脚本；本文件保留逐用例的断言和宿主配置。此检查阻止两份资产此后发生静默漂移。
#[test]
fn vendored_independent_fixtures_match_executed_alignment_scripts() {
    let fixture_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/java-testsuite/independent");
    let source = concat!(
        include_str!("basics_to_docs.rs"),
        include_str!("loops_and_functions.rs"),
        include_str!("control_and_maps.rs"),
        include_str!("numeric_operators.rs"),
        include_str!("operators_and_scope.rs"),
        include_str!("collections_and_control.rs"),
    );
    let mut stack = vec![fixture_root];
    let mut checked = 0_usize;

    while let Some(path) = stack.pop() {
        for entry in std::fs::read_dir(path).expect("read vendored Java fixture directory") {
            let entry = entry.expect("read vendored Java fixture entry");
            let entry_path = entry.path();
            if entry_path.is_dir() {
                stack.push(entry_path);
                continue;
            }
            if entry_path.extension().and_then(std::ffi::OsStr::to_str) != Some("ql") {
                continue;
            }
            let script = std::fs::read_to_string(&entry_path).expect("read vendored Java script");
            let normalized_script = script.trim_end_matches('\n');
            assert!(
                source.contains(normalized_script),
                "fixture is not embedded in an executable alignment test: {}",
                entry_path.display()
            );
            checked += 1;
        }
    }

    assert_eq!(
        checked, 151,
        "Java independent testsuite fixture count changed"
    );
}

/// 逐项对应 Java `TestSuiteRunner#assertTest`，验证测试路径附件、
/// `BIZ_EXCEPTION` 错误码/原因，以及函数名和变量名可共存。
#[test]
fn suite_runner_assert_contract() {
    let runner = alignment_util::suite_runner();
    let mut attachments = std::collections::HashMap::new();
    attachments.insert("TEST_PATH".to_string(), DataValue::Str("a/b.ql".into()));
    let options = QLOptions::builder().attachments(attachments).build();

    runner
        .execute("assert(true)", std::collections::HashMap::new(), &options)
        .expect("true assertion");

    let default_error = runner
        .execute("assert(false)", std::collections::HashMap::new(), &options)
        .expect_err("false assertion");
    assert_eq!(default_error.error_code(), "BIZ_EXCEPTION");
    assert_eq!(default_error.reason(), "a/b.ql: assert fail");

    let custom_error = runner
        .execute(
            "assert(false, 'my test')",
            std::collections::HashMap::new(),
            &options,
        )
        .expect_err("false assertion with message");
    assert_eq!(custom_error.error_code(), "BIZ_EXCEPTION");
    assert_eq!(custom_error.reason(), "a/b.ql: my test");

    runner
        .execute(
            "assert = 4;assert(assert == 4)",
            std::collections::HashMap::new(),
            &QLOptions::builder().build(),
        )
        .expect("variable may share function name");
}

// Java source: TestSuiteRunner#suiteTestReportsAllFailures
// ADAPTED: Java AssertionError.suppressed 在 Rust 中表示为错误 Vec；汇总顺序、
// 总数、路径、错误码和 reason 仍逐项保持一致。
#[test]
fn suite_test_reports_all_failures() {
    let cases = [
        ("/00_success.ql", "assert(true)"),
        ("/01_first_failure.ql", "assert(false, 'first failure')"),
        ("/02_second_failure.ql", "assert(false, 'second failure')"),
    ];
    let mut passed = 0_usize;
    let mut failures = Vec::new();
    for (path, script) in cases {
        let runner = alignment_util::suite_runner();
        let options = QLOptions::builder()
            .attachments(std::collections::HashMap::from([(
                "TEST_PATH".to_string(),
                DataValue::Str(path.into()),
            )]))
            .build();
        match runner.execute(script, std::collections::HashMap::new(), &options) {
            Ok(_) => passed += 1,
            Err(error) => failures.push((path, error)),
        }
    }

    let mut summary = format!(
        "Test suite completed: total {}, passed {passed}, failed {}",
        passed + failures.len(),
        failures.len()
    );
    summary.push_str("\nFailed QL test files:");
    for (index, (path, error)) in failures.iter().enumerate() {
        summary.push_str(&format!(
            "\n\n{}) {path} - QLException:\n{}",
            index + 1,
            error
        ));
    }

    assert_eq!(failures.len(), 2);
    assert!(summary.starts_with("Test suite completed: total 3, passed 1, failed 2"));
    assert!(summary.contains("Failed QL test files:"));
    assert!(summary.contains("/01_first_failure.ql"));
    assert!(summary.contains("first failure"));
    assert!(summary.contains("/02_second_failure.ql"));
    assert!(summary.contains("second failure"));
    assert_eq!(failures[0].1.error_code(), "BIZ_EXCEPTION");
    assert_eq!(
        failures[0].1.reason(),
        "/01_first_failure.ql: first failure"
    );
    assert_eq!(
        failures[1].1.reason(),
        "/02_second_failure.ql: second failure"
    );
}

// Java source: TestSuiteRunner#featureDebug
#[test]
fn feature_debug_executes_switch_fallthrough_with_debug_output() {
    let debug_lines = std::rc::Rc::new(std::cell::RefCell::new(Vec::new()));
    let captured = std::rc::Rc::clone(&debug_lines);
    let init_options = InitOptions::builder()
        .class_supplier(std::rc::Rc::new(alignment_util::JdkClassSupplier))
        .security_strategy(QLSecurityStrategy::open())
        .debug(true)
        .debug_info_consumer(std::rc::Rc::new(move |line| {
            captured.borrow_mut().push(line);
        }))
        .build();
    let runner = alignment_util::suite_runner_with_init_options(init_options);
    let script = r#"/*
{
  "noReturn": true
}
*/
int x = 10;
String result;

switch (x) {
  case 10:
  case 9:
    result = "A";
    break;
  case 8:
    result = "B";
    break;
  default:
    result = "F";
}

assert(result == "A");

function stest(y) {
    switch (y) {
      case 10:
      case 9:
        return "A";
      case 8:
        return "B";
      default:
        return "F";
    }
}

assert(stest(9) == "A");
assert(stest(10) == "A");"#;
    let options = QLOptions::builder()
        .attachments(std::collections::HashMap::from([(
            "TEST_PATH".to_string(),
            DataValue::Str("/independent/switch/switch_fallthrough.ql".into()),
        )]))
        .build();

    let result = runner
        .execute(script, std::collections::HashMap::new(), &options)
        .expect("debug suite file");
    assert!(result.result().is_null());
    let debug_lines = debug_lines.borrow();
    assert!(!debug_lines.is_empty());
    assert!(debug_lines
        .iter()
        .any(|line| line.contains("Compile consume time")));
    assert!(debug_lines
        .iter()
        .any(|line| line.contains("Execute consume time")));
}

/// 对应 Java testsuite 脚本 `testsuite/independent/array/array_index_out_of_bound.ql`。
#[test]
fn array_array_index_out_of_bound() {
    const SCRIPT: &str = r#"/*
{
  "errCode": "INDEX_OUT_BOUND"
}
*/
a = [];
a[1]"#;
    expect_err_code(SCRIPT, "INDEX_OUT_BOUND");
}

/// 对应 Java testsuite 脚本 `testsuite/independent/array/array_literal.ql`。
#[test]
fn array_array_literal() {
    const SCRIPT: &str = r#"a = [1,2,3, "123"];
assert(a == [1,2,3, "123"]);
assert(a != [1,2,3, "125"]);
assert(a[0] == 1 && a[3] == "123");
assert(a.length == 4);
assert(a[-1] == a[3]);"#;
    expect_ok(SCRIPT);
}

/// 对应 Java testsuite 脚本 `testsuite/independent/array/float_index.ql`。
#[test]
fn array_float_index() {
    const SCRIPT: &str = r"a = [1,2,3,4];
assert(a[2.8] == 3);";
    expect_ok(SCRIPT);
}

/// 对应 Java testsuite 脚本 `testsuite/independent/array/invalid_index.ql`。
#[test]
fn array_invalid_index() {
    const SCRIPT: &str = r#"/*
{
  "errCode": "INVALID_INDEX"
}
*/
a = [1];
a["aaa"] = 2;"#;
    expect_err_code(SCRIPT, "INVALID_INDEX");
}

/// 对应 Java testsuite 脚本 `testsuite/independent/array/max_arr_len.ql`。
#[test]
fn array_max_arr_len() {
    const SCRIPT: &str = r#"/*
{
  "qlOptions": QLOptions.builder().maxArrLength(10)
}
*/

try {
    a = new int[10]
    a = new int[1][2][10][10][9]
} catch(o) {
    assert(false);
}

assertErrorCode(() -> new int[11], "EXCEED_MAX_ARR_LENGTH")
assertErrorCode(() -> new int[1][13][3], "EXCEED_MAX_ARR_LENGTH")"#;
    expect_ok_with(SCRIPT, &QLOptions::builder().max_arr_length(10).build());
}

/// 对应 Java testsuite 脚本 `testsuite/independent/array/miss_comma_between_elements.ql`。
#[test]
fn array_miss_comma_between_elements() {
    const SCRIPT: &str = r#"/*
{
  "errCode": "SYNTAX_ERROR"
}
*/
[123 334]"#;
    expect_err_code(SCRIPT, "SYNTAX_ERROR");
}

/// 对应 Java testsuite 脚本 `testsuite/independent/array/no_rbrack_to_match.ql`。
#[test]
fn array_no_rbrack_to_match() {
    const SCRIPT: &str = r#"/*
{
  "errCode": "SYNTAX_ERROR"
}
*/
a = [1223,34,34"#;
    expect_err_code(SCRIPT, "SYNTAX_ERROR");
}

/// 对应 Java testsuite 脚本 `testsuite/independent/array/slice.ql`。
#[test]
fn array_slice() {
    const SCRIPT: &str = r"a = [1,2,3,4,5,6];
assert(a[3:] == [4,5,6]);
assert(a[:2] == [1,2]);
assert(a[2:4] == [3,4]);
assert(a[4:10] == [5, 6]);
assert(a[-88:100] == [1,2,3,4,5,6])
assert(a[3:-1] == [4,5])";
    expect_ok(SCRIPT);
}

/// 对应 Java testsuite 脚本 `testsuite/independent/array/unindexable.ql`。
#[test]
fn array_unindexable() {
    const SCRIPT: &str = r#"/*
{
  "errCode": "NONINDEXABLE_OBJECT"
}
*/
a = new HashSet();
a[1]"#;
    expect_err_code(SCRIPT, "NONINDEXABLE_OBJECT");
}

/// 对应 Java testsuite 脚本 `testsuite/independent/avoidnullpointer/avoid_null_pointer.ql`。
#[test]
fn avoidnullpointer_avoid_null_pointer() {
    const SCRIPT: &str = r#"/*
{
  "qlOptions": QLOptions.builder().avoidNullPointer(true)
}
*/
assert(a.b == null);
assert(a.b.c == null);
assert(a.b() == null);
assert(a.b().c.d() == null);
assert(a::b == null);
assert(a.b.c.mm() == null);
assert(mmm() == null)
assert(a.n.c[2]==null)
assert(a.n.c[1:4]==null)"#;
    expect_ok_with(
        SCRIPT,
        &QLOptions::builder().avoid_null_pointer(true).build(),
    );
}

/// 对应 Java testsuite 脚本 `testsuite/independent/avoidnullpointer/can_not_find_function.ql`。
#[test]
fn avoidnullpointer_can_not_find_function() {
    const SCRIPT: &str = r#"/*{
  "errCode": "FUNCTION_NOT_FOUND"
}*/
mmm()"#;
    expect_err_code(SCRIPT, "FUNCTION_NOT_FOUND");
}

/// 对应 Java testsuite 脚本 `testsuite/independent/avoidnullpointer/get_from_null.ql`。
#[test]
fn avoidnullpointer_get_from_null() {
    const SCRIPT: &str = r#"/*
{
  "errCode": "NULL_FIELD_ACCESS"
}
*/
a.b"#;
    expect_err_code(SCRIPT, "NULL_FIELD_ACCESS");
}

/// 对应 Java testsuite 脚本 `testsuite/independent/avoidnullpointer/get_method_from_null.ql`。
#[test]
fn avoidnullpointer_get_method_from_null() {
    const SCRIPT: &str = r#"/*{
  "errCode": "NULL_METHOD_ACCESS"
}*/
a::b"#;
    expect_err_code(SCRIPT, "NULL_METHOD_ACCESS");
}

/// 对应 Java testsuite 脚本 `testsuite/independent/block/block_as_expr.ql`。
#[test]
fn block_block_as_expr() {
    const SCRIPT: &str = r"a = {
  1 + 1
} + 1;
assert(a == 3);
b = {
  String c = 'ccc';
  String d = 'ddd';
  c + '-' + d
};
assert(b == 'ccc-ddd');
f = {
  int e = 10;
  if (a > 5) {
    a + e
  } else {
    a * 2
  }
};
assert(f == 6);";
    expect_ok(SCRIPT);
}

/// 对应 Java testsuite 脚本 `testsuite/independent/block/block_at_if.ql`。
#[test]
fn block_block_at_if() {
    const SCRIPT: &str = r"a = 1;
b = if (a < 5) {
  {
    a + 10
  }
} else {
  {
    a * 10
  }
};
assert(b == 11);

";
    expect_ok(SCRIPT);
}

/// 对应 Java testsuite 脚本 `testsuite/independent/block/lambda_with_block.ql`。
#[test]
fn block_lambda_with_block() {
    const SCRIPT: &str = r"f = (x) -> {
   int e = 10;
   if (x > 5) {
     x + e
   } else {
     x * 2
   }
};
assert(f(6) == 16);
assert(f(3) == 6);";
    expect_ok(SCRIPT);
}

/// 对应 Java testsuite 脚本 `testsuite/independent/block/missing_rbrace.ql`。
#[test]
fn block_missing_rbrace() {
    const SCRIPT: &str = r#"/*
{
  "errCode": "SYNTAX_ERROR"
}
*/
{
  1+1
"#;
    expect_err_code(SCRIPT, "SYNTAX_ERROR");
}

/// 对应 Java testsuite 脚本 `testsuite/independent/block/return_at_block.ql`。
#[test]
fn block_return_at_block() {
    const SCRIPT: &str = r"function returnAtBlock(a) {
    int i = if (a > 10) {
      {
        return 100;
      }
    } else {
      if (a < 5) {
        {return 1000;}
      }
      {return 101;}
    };
    10000
}

assert(returnAtBlock(11) == 100);
assert(returnAtBlock(5) == 101);
assert(returnAtBlock(-5) == 1000)";
    expect_ok(SCRIPT);
}

/// 对应 Java testsuite 脚本 `testsuite/independent/bool/bool_literal.ql`。
#[test]
fn bool_bool_literal() {
    const SCRIPT: &str = r#"assert(true == true);
assert(false == false);
assert(true != false);
assert(true != "true");"#;
    expect_ok(SCRIPT);
}

/// 对应 Java testsuite 脚本 `testsuite/independent/bool/bool_literal_is_keyword.ql`。
#[test]
fn bool_bool_literal_is_keyword() {
    const SCRIPT: &str = r#"/*
{
  "errCode": "SYNTAX_ERROR"
}
*/
true = 1;"#;
    expect_err_code(SCRIPT, "SYNTAX_ERROR");
}

/// 对应 Java testsuite 脚本 `testsuite/independent/bool/short_circuit.ql`。
#[test]
fn bool_short_circuit() {
    const SCRIPT: &str = r"function a(int value, boolean b) {
  a = value;
  return b;
}

c = a(1, false) && a(10, true);
assert(c == false);
assert(a == 1);

c = a(100, true) || a(1, false);
assert(c == true);
assert(a == 100);

d = a(1000, true) && a(10000, false);
assert(d == false);
assert(a == 10000);

e = a(11, false) or a(111, true);
assert(e == true);
assert(a == 111);

f = a(2, true) || a(3, false) && a(5, false);
assert(f == true);
assert(a == 2);";
    expect_ok(SCRIPT);
}

/// 对应 Java testsuite 脚本 `testsuite/independent/bool/short_circuit_with_block.ql`。
#[test]
fn bool_short_circuit_with_block() {
    const SCRIPT: &str = r#"{
  assert(true || Integer.parseInt("0") + Integer.parseInt("0") < 0);
}


"#;
    expect_ok(SCRIPT);
}

/// 对应 Java testsuite 脚本 `testsuite/independent/cast/cast_express.ql`。
#[test]
fn cast_cast_express() {
    const SCRIPT: &str = r"a = int;
assert(a == int);
b = 12L;
c = (int) b;
assert(c.class == a.class);
d = (int) 100.12d;
assert(d == 100);";
    expect_ok(SCRIPT);
}

/// 对应 Java testsuite 脚本 `testsuite/independent/cast/null_cast.ql`。
#[test]
fn cast_null_cast() {
    const SCRIPT: &str = r"Integer a = null;
assert(!(boolean) a);
assert((int) a == null);";
    expect_ok(SCRIPT);
}

/// 对应 Java testsuite 脚本 `testsuite/independent/comment/comment.ql`。
#[test]
fn comment_comment() {
    const SCRIPT: &str = r"// in-line comment
/*
multiline comment
*/

//";
    expect_ok(SCRIPT);
}

/// 对应 Java testsuite 脚本 `testsuite/independent/doc/convenient_syntax_elements.ql`。
#[test]
fn doc_convenient_syntax_elements() {
    const SCRIPT: &str = r#"// list
l = [1,2,3]
assert(l[0]==1)
assert(l[-1]==3)
// Underlying data type of list is ArrayList in Java
assert(l instanceof ArrayList)
// map
m = {
  "aa": 10,
  "bb": {
    "cc": "cc1",
    "dd": "dd1"
  }
}
assert(m['aa']==10)
// Underlying data type of map is LinkedHashMap in Java
assert(m instanceof LinkedHashMap)
// empty map
emMap = {:}
emMap['haha']='huhu'
assert(emMap['haha']=='huhu')"#;
    expect_ok(SCRIPT);
}

/// 对应 Java testsuite 脚本 `testsuite/independent/doc/dynamic_string.ql`。
#[test]
fn doc_dynamic_string() {
    const SCRIPT: &str = r#"a = 123
assert("hello,${a-1}" == "hello,122")

// escape $ with \$
assert("hello,\${a-1}" == "hello,\${a-1}")

b = "test"
assert("m xx ${
  if (b like 't%') {
      'YYY'
  }
}" == "m xx YYY")"#;
    expect_ok(SCRIPT);
}

/// 对应 Java testsuite 脚本 `testsuite/independent/doc/dynamic_typing.ql`。
#[test]
fn doc_dynamic_typing() {
    const SCRIPT: &str = r#"// Dynamic Typeing
a = 1;
a = "1";
// Static Typing
int b = 2;
// throw QLException with error code INCOMPATIBLE_ASSIGNMENT_TYPE when assign with incompatible type String
assertErrorCode(() -> b = "1", "INCOMPATIBLE_ASSIGNMENT_TYPE")

"#;
    expect_ok(SCRIPT);
}

/// 对应 Java testsuite 脚本 `testsuite/independent/doc/for.ql`。
#[test]
fn doc_for() {
    const SCRIPT: &str = r"l = [];
for (int i = 3; i < 6; i++) {
  l.add(i);
}
assert(l==[3,4,5])";
    expect_ok(SCRIPT);
}

/// 对应 Java testsuite 脚本 `testsuite/independent/doc/for_each.ql`。
#[test]
fn doc_for_each() {
    const SCRIPT: &str = r"sum = 0;
for (i: [0,1,2,3,4]) {
  if (i == 2) {
    continue;
  }
  sum += i;
}
assert(sum==8)";
    expect_ok(SCRIPT);
}

/// 对应 Java testsuite 脚本 `testsuite/independent/doc/function.ql`。
#[test]
fn doc_function() {
    const SCRIPT: &str = r"function sub(a, b) {
    return a-b;
}
assert(sub(3,1)==2)";
    expect_ok(SCRIPT);
}

/// 对应 Java testsuite 脚本 `testsuite/independent/doc/if.ql`。
#[test]
fn doc_if() {
    const SCRIPT: &str = r"a = 11;
// if ... else ...
assert(if (a >= 0 && a < 5) {
  true
} else if (a >= 5 && a < 10) {
  false
} else if (a >= 10 && a < 15) {
  true
} == true)

// if ... then ... else ...
r = if (a == 11) then true else false
assert(r == true)";
    expect_ok(SCRIPT);
}

/// 对应 Java testsuite 脚本 `testsuite/independent/doc/if_as_expr.ql`。
#[test]
fn doc_if_as_expr() {
    const SCRIPT: &str = r"assert(if (11 == 11) {
  10
} else {
  20 + 2
} + 1 == 11)";
    expect_ok(SCRIPT);
}

/// 对应 Java testsuite 脚本 `testsuite/independent/doc/if_then.ql`。
#[test]
fn doc_if_then() {
    const SCRIPT: &str = r"a = 11;

assert(if (a >= 0 && a < 5) then true else false == false)";
    expect_ok(SCRIPT);
}

/// 对应 Java testsuite 脚本 `testsuite/independent/doc/lambda.ql`。
#[test]
fn doc_lambda() {
    const SCRIPT: &str = r"add = (a, b) -> {
  return a + b;
}
assert(add(1,2)==3)";
    expect_ok(SCRIPT);
}

/// 对应 Java testsuite 脚本 `testsuite/independent/doc/list_map_filter.ql`。
#[test]
fn doc_list_map_filter() {
    const SCRIPT: &str = r#"l = ["a-111", "a-222", "b-333", "c-888"]
assert(l.filter(i -> i.startsWith("a-"))
        .map(i -> i.split("-")[1]) == ["111", "222"])"#;
    expect_ok(SCRIPT);
}

/// 对应 Java testsuite 脚本 `testsuite/independent/doc/switch.ql`。
#[test]
fn doc_switch() {
    const SCRIPT: &str = r#"int day = 3;
String dayName;
switch (day) {
  case 1:
    dayName = "Monday"
    break
  case 2:
    dayName = "Tuesday"
    break
  case 3:
    dayName = "Wednesday"
    break
  default:
    dayName = "Unknown"
}
assert(dayName == "Wednesday")
"#;
    expect_ok(SCRIPT);
}

/// 对应 Java testsuite 脚本 `testsuite/independent/doc/try_catch.ql`。
#[test]
fn doc_try_catch() {
    const SCRIPT: &str = r"assert(try {
    100 + 1/0
} catch(e) {
    // Throw a zero-division exception
    11
} == 11)";
    expect_ok(SCRIPT);
}

/// 对应 Java testsuite 脚本 `testsuite/independent/doc/try_catch_as_expr.ql`。
#[test]
fn doc_try_catch_as_expr() {
    const SCRIPT: &str = r"assert(1 + try {
    100 + 1/0
} catch(e) {
    // Throw a zero-division exception
    11
} == 12)";
    expect_ok(SCRIPT);
}

/// 对应 Java testsuite 脚本 `testsuite/independent/doc/while.ql`。
#[test]
fn doc_while() {
    const SCRIPT: &str = r"i = 0;
while (i < 5) {
  if (++i == 2) {
    break;
  }
}
assert(i==2)";
    expect_ok(SCRIPT);
}
