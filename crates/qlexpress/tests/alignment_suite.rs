//! Stage 6 对齐测试:移植 Java `TestSuiteRunner.suiteTest` 的 testsuite 脚本用例。
//!
//! 每个 `#[test]` 对应 `src/test/resources/testsuite/independent/` 下的一个
//! `.ql` 脚本(脚本字符串与 Java 版保持一致,含选项注释头),通过
//! `alignment_util` 中复刻的 assert/assertFalse/assertErrorCode/println 执行。
//!
//! 对应 Java: com.alibaba.qlexpress4.TestSuiteRunner#suiteTest

// 与 lib 一致的架构性豁免:QLException 对齐 Java 单一异常类(见 lib.rs)。
#![allow(clippy::result_large_err)]

mod alignment_util;

use alignment_util::{
    expect_err_code, expect_err_code_with, expect_null, expect_ok, expect_ok_with,
};
use qlexpress::init_options::InitOptions;
use qlexpress::ql_options::QLOptions;
use qlexpress::runtime::value::DataValue;
use qlexpress::security::ql_security_strategy::QLSecurityStrategy;

/// 逐项对应 Java `TestSuiteRunner#assertTest`，验证测试路径附件、
/// `BIZ_EXCEPTION` 错误码/原因，以及函数名和变量名可共存。
#[test]
fn suite_runner_assert_contract() {
    let runner = alignment_util::suite_runner();
    let mut attachments = std::collections::HashMap::new();
    attachments.insert(
        "TEST_PATH".to_string(),
        DataValue::Str("a/b.ql".to_string()),
    );
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
                DataValue::Str(path.to_string()),
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
            DataValue::Str("/independent/switch/switch_fallthrough.ql".to_string()),
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

/// 对应 Java testsuite 脚本 `testsuite/independent/extensionfunction/extension_function.ql`。
#[test]
fn extensionfunction_extension_function() {
    const SCRIPT: &str = r#"a = [1,2,3,4].filter(i -> i > 2)
assert(a == [3,4])
assert(a instanceof List)

assert([1,2].map(i -> i+2) == a)

assertErrorCode(() -> {'a':1}.filter(en -> en.value > 10), "METHOD_NOT_FOUND")"#;
    expect_ok(SCRIPT);
}

/// 对应 Java testsuite 脚本 `testsuite/independent/for/break_continue.ql`。
#[test]
fn for_break_continue() {
    const SCRIPT: &str = r"for (i = 0; i < 5; i++) {
  if (i == 2) {
    break;
  }
}
assert(i == 2);

sum = 0;
for (i = 0; i < 5; i++) {
  if (i == 2) {
    continue;
  }
  sum += i;
}
assert(sum == 8);

sum = 0;
for (i = 0; i < 5; i++) {
  if (i == 2) {
    if (i == 2) {
      continue;
    }
  }
  sum += i;
}
assert(sum == 8);

";
    expect_ok(SCRIPT);
}

/// 对应 Java testsuite 脚本 `testsuite/independent/for/c_for.ql`。
#[test]
fn for_c_for() {
    const SCRIPT: &str = r"l = [];
for (int i = 3; i < 6; i++) {
  l.add(i);
}
assert(l == [3,4,5]);
assert(i == null);

l1 = [];
for (j = 10; j > 8; j--) {
  l1.add(j);
}
assert(l1 == [10, 9]);
assert(j == 8);

// scope test; h not in for condition scope
for (m = 0; m < 5 && h == null; m++) {
  int h = 5;
}
assert(m == 5);";
    expect_ok(SCRIPT);
}

/// 对应 Java testsuite 脚本 `testsuite/independent/for/condition_not_bool.ql`。
#[test]
fn for_condition_not_bool() {
    const SCRIPT: &str = r#"/*
{
  "errCode": "FOR_CONDITION_BOOL_REQUIRED"
}
*/
for (i = 0; 1+1; false) {}"#;
    expect_err_code(SCRIPT, "FOR_CONDITION_BOOL_REQUIRED");
}

/// 对应 Java testsuite 脚本 `testsuite/independent/for/for_each.ql`。
#[test]
fn for_for_each() {
    const SCRIPT: &str = r"i = 0;
l = [1,2,3];
for (ele : l) {
  assert(l[i++] == ele);
}

j = 0;
for (int ele : l) {
  assert(l[j++] == ele);
}";
    expect_ok(SCRIPT);
}

/// 对应 Java testsuite 脚本 `testsuite/independent/for/for_each_break_continue.ql`。
#[test]
fn for_for_each_break_continue() {
    const SCRIPT: &str = r"globalI = 0;
for (int i: [0,1,2,3,4]) {
  globalI = i;
  if (i == 2) {
    break;
  }
}
assert(globalI == 2);

sum = 0;
for (i: [0,1,2,3,4]) {
  if (i == 2) {
    continue;
  }
  sum += i;
}
assert(sum == 8);

sum = 0;
for (i: [0,1,2,3,4]) {
  if (i == 2) {
    if (i == 2) {
      continue;
    }
  }
  sum += i;
}
assert(sum == 8);

";
    expect_ok(SCRIPT);
}

/// 对应 Java testsuite 脚本 `testsuite/independent/for/for_each_invalid_type.ql`。
#[test]
fn for_for_each_invalid_type() {
    const SCRIPT: &str = r#"/*
{
  "errCode": "FOR_EACH_TYPE_MISMATCH"
}
*/
a = [1,2,"abc"];

for (int b : a) {
    c = 2
}"#;
    expect_err_code(SCRIPT, "FOR_EACH_TYPE_MISMATCH");
}

/// 对应 Java testsuite 脚本 `testsuite/independent/for/for_each_not_iterable.ql`。
#[test]
fn for_for_each_not_iterable() {
    const SCRIPT: &str = r#"/*
{
  "errCode": "FOR_EACH_ITERABLE_REQUIRED"
}
*/
for (c : 100) {
}"#;
    expect_err_code(SCRIPT, "FOR_EACH_ITERABLE_REQUIRED");
}

/// 对应 Java testsuite 脚本 `testsuite/independent/for/infinite_loop.ql`。
#[test]
fn for_infinite_loop() {
    const SCRIPT: &str = r"i = 0;
for (;;) {
  if (i > 3) {
    break;
  }
  i++;
}
assert(i == 4);

for (j = 0; ; j++) {
  if (j > 3) {
    break;
  }
}
assert(j == 4);";
    expect_ok(SCRIPT);
}

/// 对应 Java testsuite 脚本 `testsuite/independent/for/missing_lparen_at_for.ql`。
#[test]
fn for_missing_lparen_at_for() {
    const SCRIPT: &str = r#"/*
{
  "errCode": "SYNTAX_ERROR"
}
*/
for int i;;;)"#;
    expect_err_code(SCRIPT, "SYNTAX_ERROR");
}

/// 对应 Java testsuite 脚本 `testsuite/independent/for/missing_rparen_after_for_update.ql`。
#[test]
fn for_missing_rparen_after_for_update() {
    const SCRIPT: &str = r#"/*
{
  "errCode": "SYNTAX_ERROR"
}
*/
for (int i = 0; i < 10; i++ {}"#;
    expect_err_code(SCRIPT, "SYNTAX_ERROR");
}

/// 对应 Java testsuite 脚本 `testsuite/independent/for/missing_rparen_at_for_each.ql`。
#[test]
fn for_missing_rparen_at_for_each() {
    const SCRIPT: &str = r#"/*
{
  "errCode": "SYNTAX_ERROR"
}
*/
for (a :[1,2,3] {
}"#;
    expect_err_code(SCRIPT, "SYNTAX_ERROR");
}

/// 对应 Java testsuite 脚本 `testsuite/independent/for/missing_semi_after_for_init.ql`。
#[test]
fn for_missing_semi_after_for_init() {
    const SCRIPT: &str = r#"/*
{
  "errCode": "SYNTAX_ERROR"
}
*/
for (i = 0 i < 10; i++) {}"#;
    expect_err_code(SCRIPT, "SYNTAX_ERROR");
}

/// 对应 Java testsuite 脚本 `testsuite/independent/for/return_from_for.ql`。
#[test]
fn for_return_from_for() {
    const SCRIPT: &str = r#"function test(l) {
  for (o:l) {
    if (o == 10) {
      return "find" + o;
    }
  }
}

r1 = test([3,4,10]);
assert(r1 == 'find10');

r2 = test([3,4,11]);
assert(r2 == null);"#;
    expect_ok(SCRIPT);
}

/// 对应 Java testsuite 脚本 `testsuite/independent/function/complex_parameters.ql`。
#[test]
fn function_complex_parameters() {
    const SCRIPT: &str = r"// Test functions with various parameter types and complexity
assert(processData(5, true) == 15)
assert(calculateComplex(1.5, 2.5, 10) == 160.0)

// Test function calls with complex expressions as parameters
assert(mathOperations(add(2, 3), multiply(2, 2)) == 625) // 5^4 = 625

function processData(int count, boolean flag) {
    int result = count * 2;
    if (flag) {
        result = result + 5;
    }
    return result;
}

function calculateComplex(double x, double y, int multiplier) {
    double base = x + y;
    return base * base * multiplier;
}

function mathOperations(int a, int b) {
    return power(a, b);
}

function add(int x, int y) {
    return x + y;
}

function multiply(int x, int y) {
    return x * y;
}

function power(int base, int exp) {
    if (exp == 0) {
        return 1;
    }
    int result = 1;
    for (int i = 0; i < exp; i++) {
        result *= base;
    }
    return result;
}

// Test functions with no parameters
assert(getConstant() == 42)
assert(generateRandom() > 0)

function getConstant() {
    return 42;
}

function generateRandom() {
    // Simple pseudo-random using current execution context
    return 123; // For deterministic testing
}

// Test function overloading-like behavior with different parameter counts
assert(calculate(5) == 25)
assert(calculate2(5, 3) == 39) // 5*5 + 3*3 + 5 = 25 + 9 + 5 = 39

function calculate(int x) {
    return x * x;
}

function calculate2(int x, int y) {
    return x * x + y * y + 5;
}";
    expect_ok(SCRIPT);
}

/// 对应 Java testsuite 脚本 `testsuite/independent/function/edge_cases.ql`。
#[test]
fn function_edge_cases() {
    const SCRIPT: &str = r#"// Test edge cases for function hoisting
// Test function called immediately at global level before definition
int immediateResult = callImmediately();
assert(immediateResult == 42)

// Test function in conditional blocks called before definition  
if (true) {
    assert(conditionalFunction(5) == 10)
}

// Test function in loop called before definition
for (int i = 0; i < 2; i++) {
    assert(loopFunction(i) == i * 3)
}

// Test function with same name as built-in (should work)
assert(toString(123) == "custom_123")

// Test function that returns function call result
assert(chainReturn() == 999)

// Function definitions (after all calls)
function callImmediately() {
    return 42;
}

function conditionalFunction(int x) {
    return x * 2;
}

function loopFunction(int x) {
    return x * 3;
}

function toString(int value) {
    return "custom_" + value;
}

function chainReturn() {
    return getSpecialValue();
}

function getSpecialValue() {
    return 999;
}

// Test empty function
assert(emptyFunction() == null)

function emptyFunction() {
    // Empty body
}

// Test function that just returns constant
assert(constantFunction() == "CONSTANT")

function constantFunction() {
    return "CONSTANT";
}

// Test functions with early returns
assert(earlyReturnFunction(true) == 1)
assert(earlyReturnFunction(false) == 2)

function earlyReturnFunction(boolean condition) {
    if (condition) {
        return 1;
    }
    return 2;
}

// Test function calling itself indirectly (through another function)
assert(indirectSelfCall(3) == 6)

function indirectSelfCall(int n) {
    if (n <= 0) {
        return 0;
    }
    return n + helperForIndirect(n - 1);
}

function helperForIndirect(int n) {
    return indirectSelfCall(n);
}

// Test function with multiple return types based on conditions
function dynamicReturn(int choice) {
    if (choice == 1) {
        return 100;
    } else if (choice == 2) {
        return "text";
    } else {
        return true;
    }
}

// Test the dynamic returns
assert(dynamicReturn(1) == 100)
assert(dynamicReturn(2) == "text")
assert(dynamicReturn(3) == true)"#;
    expect_ok(SCRIPT);
}

/// 对应 Java testsuite 脚本 `testsuite/independent/function/function_call.ql`。
#[test]
fn function_function_call() {
    const SCRIPT: &str = r#"function add(int a, int b) {
    return a+b;
}

assert(add(1,1)==2)

function sub(a, b) {
    return a-b;
}

assert(sub(3,1)==2)

assertErrorCode(() -> {add(1, "2")}, "INVALID_ARGUMENT")

assert(check(3,1)==false)

function check(a, b) {
    return a < b;
}
"#;
    expect_ok(SCRIPT);
}

/// 对应 Java testsuite 脚本 `testsuite/independent/function/function_scoping.ql`。
#[test]
fn function_function_scoping() {
    const SCRIPT: &str = r"// Test function scoping and parameter shadowing
int outerVariable = 10;

// Test function with same parameter name as global variable
assert(testScoping(5) == 15)

// Test that global variable is not affected
assert(outerVariable == 10)

function testScoping(int outerVariable) {
    // Parameter shadows global variable
    return outerVariable + 10;
}

// Test nested scoping with local variables
assert(nestedScoping(3) == 18)

function nestedScoping(int x) {
    int localVar = x * 2; // 6
    if (localVar > 5) {
        int innerVar = localVar * 2; // 12
        return innerVar + 6; // 18
    }
    return localVar;
}

// Test functions that call other functions with same parameter names
assert(chainedScoping(2) == 14)

function chainedScoping(int value) {
    return helperFunction(value + 1);
}

function helperFunction(int value) {
    // Different 'value' parameter
    return value * 4 + 2; // (2+1) * 4 + 2 = 14
}

// Test function with multiple parameters having local scope
assert(multipleParams(1, 2, 3) == 12)

function multipleParams(int a, int b, int c) {
    int sum = a + b + c;
    int doubled = sum * 2;
    return doubled;
}

// Test function that modifies parameters (local copies)
int originalValue = 5;
assert(modifyParameter(originalValue) == 25)
assert(originalValue == 5) // Original should remain unchanged

function modifyParameter(int param) {
    param = param * 5; // Modifying local copy
    return param;
}

// Test function with conditional blocks and local variables
assert(conditionalScoping(true, 10) == 30)
assert(conditionalScoping(false, 10) == 0) // 10 - 10 = 0

function conditionalScoping(boolean condition, int base) {
    int result = base;
    if (condition) {
        int bonus = 20;
        result = result + bonus;
    } else {
        int penalty = 10;
        result = result - penalty;
    }
    return result;
}

// Test loops with local scope
assert(loopScoping(3) == 6)

function loopScoping(int count) {
    int total = 0;
    for (int i = 1; i <= count; i++) {
        int squared = i; // Local to loop iteration
        total += squared;
    }
    return total; // 1 + 2 + 3 = 6
}";
    expect_ok(SCRIPT);
}

/// 对应 Java testsuite 脚本 `testsuite/independent/function/invalid_argument.ql`。
#[test]
fn function_invalid_argument() {
    const SCRIPT: &str = r#"/*
{
  "errCode": "INVALID_ARGUMENT"
}
*/
function add(a, int b) {
  return a + b;
}

add("aaa", "ffff");"#;
    expect_err_code(SCRIPT, "INVALID_ARGUMENT");
}

/// 对应 Java testsuite 脚本 `testsuite/independent/function/mixed_declarations.ql`。
#[test]
fn function_mixed_declarations() {
    const SCRIPT: &str = r"// Test mixed function and variable declarations with forward references
int globalVar = computeInitialValue();
int message = formatMessage(42, getValue());

assert(globalVar == 100)
assert(message == 92) // 42 + 50 = 92

// Variables referencing functions before they're defined
int result1 = doubleValue(25);
assert(result1 == 50)

function computeInitialValue() {
    return 100;
}

function formatMessage(int prefix, int value) {
    return prefix + value;
}

function getValue() {
    return 50;
}

function doubleValue(int x) {
    return x * 2;
}

// Test functions that modify and return based on global state
int counter = 0;

function increment() {
    counter = counter + 1;
    return counter;
}

function getCounterValue() {
    return counter;
}

// Test the counter functions
int first = increment();   // Should be 1
int second = increment();  // Should be 2
int current = getCounterValue(); // Should be 2

assert(first == 1)
assert(second == 2) 
assert(current == 2)

// Test simple calculation with constants
function simpleCalculation() {
    return 10 * 30; // 300
}

assert(simpleCalculation() == 300)";
    expect_ok(SCRIPT);
}

/// 对应 Java testsuite 脚本 `testsuite/independent/function/multi_call.ql`。
#[test]
fn function_multi_call() {
    const SCRIPT: &str = r"a = {:};

function a() {
    return a;
}

a().b = 10;

assert(a.b==10);";
    expect_ok(SCRIPT);
}

/// 对应 Java testsuite 脚本 `testsuite/independent/function/nested_function_calls.ql`。
#[test]
fn function_nested_function_calls() {
    const SCRIPT: &str = r"// Test nested function calls with forward declarations
assert(outerFunc(5) == 25)

assert(calculateArea(3, 4) == 26) // calls getPerimeter inside - 2*(3+4) + 3*4 = 14 + 12 = 26

function outerFunc(x) {
    return innerFunc(x) * 5;
}

function innerFunc(x) {
    return x;
}

function calculateArea(width, height) {
    int perimeter = getPerimeter(width, height);
    return perimeter + (width * height);
}

function getPerimeter(w, h) {
    return 2 * (w + h);
}

// Test deeply nested calls
assert(level1(2) == 14) // level4(2)*3-1+2*2 = 6-1+2*2 = 5+2*2 = 7*2 = 14

function level1(x) {
    return level2(x) * 2;
}

function level2(x) {
    return level3(x) + 2;
}

function level3(x) {
    return level4(x) - 1;
}

function level4(x) {
    return x * 3;
}";
    expect_ok(SCRIPT);
}

/// 对应 Java testsuite 脚本 `testsuite/independent/function/recursive_functions.ql`。
#[test]
fn function_recursive_functions() {
    const SCRIPT: &str = r"// Test recursive functions with forward declarations
assert(factorial(5) == 120)
assert(fibonacci(6) == 8)
assert(gcd(48, 18) == 6)

// Forward references to recursive functions
int result1 = factorial(4);
assert(result1 == 24)

int result2 = fibonacci(7);
assert(result2 == 13)

function factorial(n) {
    if (n <= 1) {
        return 1;
    }
    return n * factorial(n - 1);
}

function fibonacci(n) {
    if (n <= 1) {
        return n;
    }
    return fibonacci(n - 1) + fibonacci(n - 2);
}

function gcd(a, b) {
    if (b == 0) {
        return a;
    }
    return gcd(b, a % b);
}

// Test mutually recursive functions called before definition
assert(isEven(10) == true)
assert(isOdd(10) == false)
assert(isEven(7) == false)
assert(isOdd(7) == true)

function isEven(n) {
    if (n == 0) {
        return true;
    }
    return isOdd(n - 1);
}

function isOdd(n) {
    if (n == 0) {
        return false;
    }
    return isEven(n - 1);
}";
    expect_ok(SCRIPT);
}

/// 对应 Java testsuite 脚本 `testsuite/independent/function/return_left_value.ql`。
#[test]
fn function_return_left_value() {
    const SCRIPT: &str = r"map = {a:1, b:123}

function returnLeftValue() {
    return map.b;
}

c = returnLeftValue();
assert(c == 123);
// c's modification will not effect m.b
c = 190;
assert(map.b == 123);";
    expect_ok(SCRIPT);
}

/// 对应 Java testsuite 脚本 `testsuite/independent/if/if_as_expr.ql`。
#[test]
fn if_if_as_expr() {
    const SCRIPT: &str = r"a = if (11 == 11) {
  10
} else {
  20 + 2
} + 1;
b = if (a == 11) 20 else 9;
c = if (a != 11) 11 else 12;
println(b);
assert(b == 20);
assert(c == 12);

assert(if (20==20) {
  11 == 11
});

assert(if (20==20) 11 == 11);";
    expect_ok(SCRIPT);
}

/// 对应 Java testsuite 脚本 `testsuite/independent/if/if_condition_not_bool.ql`。
#[test]
fn if_if_condition_not_bool() {
    const SCRIPT: &str = r#"/*
{
  "errCode": "CONDITION_BOOL_REQUIRED"
}
*/
if (1) {
  return 2;
}"#;
    expect_err_code(SCRIPT, "CONDITION_BOOL_REQUIRED");
}

/// 对应 Java testsuite 脚本 `testsuite/independent/if/if_else_if.ql`。
#[test]
fn if_if_else_if() {
    const SCRIPT: &str = r"a = 11;
if (a >= 0 && a < 5) {
  assert(false);
} else if (a >= 5 && a < 10) {
  assert(false);
} else if (a >= 10 && a < 15) {
  assert(true);
}";
    expect_ok(SCRIPT);
}

/// 对应 Java testsuite 脚本 `testsuite/independent/if/if_else_miss_body.ql`。
#[test]
fn if_if_else_miss_body() {
    const SCRIPT: &str = r#"/*
{
  "errCode": "SYNTAX_ERROR"
}
*/
if (1>2) {
  return 10;
} else"#;
    expect_err_code(SCRIPT, "SYNTAX_ERROR");
}

/// 对应 Java testsuite 脚本 `testsuite/independent/if/if_followed_by_cast.ql`。
#[test]
fn if_if_followed_by_cast() {
    const SCRIPT: &str = r"if(true) {
}
((long)2);";
    expect_ok(SCRIPT);
}

/// 对应 Java testsuite 脚本 `testsuite/independent/if/if_miss_body.ql`。
#[test]
fn if_if_miss_body() {
    const SCRIPT: &str = r#"/*
{
  "errCode": "SYNTAX_ERROR"
}
*/
if (1>2)"#;
    expect_err_code(SCRIPT, "SYNTAX_ERROR");
}

/// 对应 Java testsuite 脚本 `testsuite/independent/if/if_with_one_statement_body.ql`。
#[test]
fn if_if_with_one_statement_body() {
    const SCRIPT: &str = r"b = () -> if (a != 100)
  return 11;
else
  return 12;
;
assert(b() == 11);";
    expect_ok(SCRIPT);
}

/// 对应 Java testsuite 脚本 `testsuite/independent/if/if_without_condition.ql`。
#[test]
fn if_if_without_condition() {
    const SCRIPT: &str = r#"/*
{
  "errCode": "SYNTAX_ERROR"
}
*/
if () {
}"#;
    expect_err_code(SCRIPT, "SYNTAX_ERROR");
}

/// 对应 Java testsuite 脚本 `testsuite/independent/if/if_without_condition_2.ql`。
#[test]
fn if_if_without_condition_2() {
    const SCRIPT: &str = r#"/*
{
  "errCode": "SYNTAX_ERROR"
}
*/
if("#;
    expect_err_code(SCRIPT, "SYNTAX_ERROR");
}

/// 对应 Java testsuite 脚本 `testsuite/independent/if/miss_if_lparen.ql`。
#[test]
fn if_miss_if_lparen() {
    const SCRIPT: &str = r#"/*
{
  "errCode": "SYNTAX_ERROR"
}
*/
if a("#;
    expect_err_code(SCRIPT, "SYNTAX_ERROR");
}

/// 对应 Java testsuite 脚本 `testsuite/independent/if/miss_if_rparen.ql`。
#[test]
fn if_miss_if_rparen() {
    const SCRIPT: &str = r#"/*
{
  "errCode": "SYNTAX_ERROR"
}
*/
if (a>10;"#;
    expect_err_code(SCRIPT, "SYNTAX_ERROR");
}

/// 对应 Java testsuite 脚本 `testsuite/independent/if/return_at_if.ql`。
#[test]
fn if_return_at_if() {
    const SCRIPT: &str = r"function returnFromIf(a) {
    int i = if (a > 10) {
      return 100;
    } else {
      if (a < 5) {
        return 1000;
      }
      return 101;
    };
}

assert(returnFromIf(11) == 100);
assert(returnFromIf(5) == 101);
assert(returnFromIf(-5) == 1000)";
    expect_ok(SCRIPT);
}

/// 对应 Java testsuite 脚本 `testsuite/independent/if/simple_if.ql`。
#[test]
fn if_simple_if() {
    const SCRIPT: &str = r"int a = 10;
if (a > 9) {
  a = 11;
} else {
  a = 5;
}

assert(a == 11);

b = 5;
if (a > 20) {
  b = 90;
}
assert(b == 5);

if (b==5) a = 90 else a = 900;

assert(a == 90);

if (b==5) {
  int m = 100;
}

if (mmm != null) {
} else {
  int mmm = 201;
}";
    expect_ok(SCRIPT);
}

/// 对应 Java testsuite 脚本 `testsuite/independent/lambda/invalid_argument.ql`。
#[test]
fn lambda_invalid_argument() {
    const SCRIPT: &str = r#"/*
{
  "errCode": "INVALID_ARGUMENT"
}
*/
add = (a, int b) -> {
  return a + b;
};
add('aa', 'bbb');"#;
    expect_err_code(SCRIPT, "INVALID_ARGUMENT");
}

/// 对应 Java testsuite 脚本 `testsuite/independent/lambda/invalid_argument_call.ql`。
#[test]
fn lambda_invalid_argument_call() {
    const SCRIPT: &str = r#"/*
{
  "errCode": "INVALID_ARGUMENT"
}
*/
l = (int c) -> c + 1;

l("abc");"#;
    expect_err_code(SCRIPT, "INVALID_ARGUMENT");
}

/// 对应 Java testsuite 脚本 `testsuite/independent/lambda/lambda_doc.ql`。
#[test]
fn lambda_lambda_doc() {
    const SCRIPT: &str = r"add = (a, b) -> {
  return a + b;
};
i = add(1,2);
assert(i == 3);";
    expect_ok(SCRIPT);
}

/// 对应 Java testsuite 脚本 `testsuite/independent/lambda/lambda_return.ql`。
#[test]
fn lambda_lambda_return() {
    const SCRIPT: &str = r"add = (a, int b) -> {
  return a + b;
};
i = add(1,2);
assert(i == 3);
j = add(4,5);
assert(j == 9);";
    expect_ok(SCRIPT);
}

/// 对应 Java testsuite 脚本 `testsuite/independent/lambda/simple_lambda.ql`。
#[test]
fn lambda_simple_lambda() {
    const SCRIPT: &str = r"exprLambda = () -> 12;
assert(exprLambda() == 12);

blockLambda = () -> {
  return 6 + 6;
};
assert(blockLambda() == 12);

emptyLambda = () -> {};
assert(emptyLambda() == null);";
    expect_ok(SCRIPT);
}

/// 对应 Java testsuite 脚本 `testsuite/independent/lambda/unmatch_param_num.ql`。
#[test]
fn lambda_unmatch_param_num() {
    const SCRIPT: &str = r#"l = (a,b) -> a + b;

assert(l("abc-") == "abc-null");"#;
    expect_ok(SCRIPT);
}

/// 对应 Java testsuite 脚本 `testsuite/independent/macro/empty_macro.ql`。
#[test]
fn macro_empty_macro() {
    const SCRIPT: &str = r"macro empty {
}

function func() {
  1+1;
  empty;
}

assert(func() == null);";
    expect_ok(SCRIPT);
}

/// 对应 Java testsuite 脚本 `testsuite/independent/macro/invalid_macro_name.ql`。
#[test]
fn macro_invalid_macro_name() {
    const SCRIPT: &str = r#"/*
{
  "errCode": "SYNTAX_ERROR"
}
*/
macro if {
}"#;
    expect_err_code(SCRIPT, "SYNTAX_ERROR");
}

/// 对应 Java testsuite 脚本 `testsuite/independent/macro/macro.ql`。
#[test]
fn macro_macro() {
    const SCRIPT: &str = r"// tag::addMacroInScript[]
macro add {
  c = a + b;
}

a = 1;
b = 2;
add;
assert(c == 3);
// end::addMacroInScript[]
b = 10;
add;
assert(c == 11);
// variable has the same name with macro
add = 100;
a = 3;
add;
assert(c == 13);
assert(add == 100);

// expression auto return
function macroReturn(a, b) {
  add
}

assert(macroReturn(6,7)==13)";
    expect_ok(SCRIPT);
}

/// 对应 Java testsuite 脚本 `testsuite/independent/macro/macro_break_continue.ql`。
#[test]
fn macro_macro_break_continue() {
    const SCRIPT: &str = r"macro bc {
  if (i < 5) {
    continue;
  }
}

s = 0;
for (int i = 0; i < 10; i++) {
  bc;
  s++;
}
assert(s == 5)";
    expect_ok(SCRIPT);
}

/// 对应 Java testsuite 脚本 `testsuite/independent/macro/macro_control_flow.ql`。
#[test]
fn macro_macro_control_flow() {
    const SCRIPT: &str = r"macro control {
  if (i > 3) {
    return;
  }
};

t = -1;
forBody = (i) -> {
  control;
  t = i;
};

forBody(10);
assert(t == -1);
forBody(2);
assert(t == 2);";
    expect_ok(SCRIPT);
}

/// 对应 Java testsuite 脚本 `testsuite/independent/macro/macro_define_in_sub_scope.ql`。
#[test]
fn macro_macro_define_in_sub_scope() {
    const SCRIPT: &str = r"function testMacroInSubScope() {
  macro add {
    int c = a + b;
  }
  int a = 1;
  int b = 10;
  add;
  return c;
}

c = testMacroInSubScope()
assert(c==11)
a = 11
b = 100
// add is not a macro in this scope
add
assert(c==11)";
    expect_ok(SCRIPT);
}

/// 对应 Java testsuite 脚本 `testsuite/independent/macro/macro_return.ql`。
#[test]
fn macro_macro_return() {
    const SCRIPT: &str = r"macro test {
  1+1
}

l = () -> {
  test
};

assert(l() == 2)";
    expect_ok(SCRIPT);
}

/// 对应 Java testsuite 脚本 `testsuite/independent/macro/missing_lbrace.ql`。
#[test]
fn macro_missing_lbrace() {
    const SCRIPT: &str = r#"/*
{
  "errCode": "SYNTAX_ERROR"
}
*/
macro m a=1"#;
    expect_err_code(SCRIPT, "SYNTAX_ERROR");
}

/// 对应 Java testsuite 脚本 `testsuite/independent/map/colon_absent_between_map_entry.ql`。
#[test]
fn map_colon_absent_between_map_entry() {
    const SCRIPT: &str = r#"/*
{
  "errCode": "SYNTAX_ERROR"
}
*/
map = {
  aa: 111
  bb: 222
};"#;
    expect_err_code(SCRIPT, "SYNTAX_ERROR");
}

/// 对应 Java testsuite 脚本 `testsuite/independent/map/colon_absent_in_entry.ql`。
#[test]
fn map_colon_absent_in_entry() {
    const SCRIPT: &str = r#"/*
{
  "errCode": "SYNTAX_ERROR"
}
*/
{
  aa: 123,
  bb 444
}"#;
    expect_err_code(SCRIPT, "SYNTAX_ERROR");
}

/// 对应 Java testsuite 脚本 `testsuite/independent/map/invalid_map_key.ql`。
#[test]
fn map_invalid_map_key() {
    const SCRIPT: &str = r#"/*
{
  "errCode": "SYNTAX_ERROR"
}
*/
{
  12 : 'aa'
}"#;
    expect_err_code(SCRIPT, "SYNTAX_ERROR");
}

/// 对应 Java testsuite 脚本 `testsuite/independent/map/key_word_can_not_get_from_field.ql`。
#[test]
fn map_key_word_can_not_get_from_field() {
    const SCRIPT: &str = r#"/*
{
  "errCode": "SYNTAX_ERROR"
}
*/
map = {
  if: 1
};
assert(map.if == 1);"#;
    expect_err_code(SCRIPT, "SYNTAX_ERROR");
}

/// 对应 Java testsuite 脚本 `testsuite/independent/map/keyword_key_map.ql`。
#[test]
fn map_keyword_key_map() {
    const SCRIPT: &str = r"map = {
  if: 1,
  else: 2,
  int: 3
};
assert(map['if'] == 1);
assert(map['else'] == 2);
assert(map['int'] == 3);";
    expect_ok(SCRIPT);
}

/// 对应 Java testsuite 脚本 `testsuite/independent/map/map_at_block.ql`。
#[test]
fn map_map_at_block() {
    const SCRIPT: &str = r"m = {
  {
    mmm: 111,
    ccc: 222
  }
};
assert(m.mmm == 111);";
    expect_ok(SCRIPT);
}

/// 对应 Java testsuite 脚本 `testsuite/independent/map/map_define.ql`。
#[test]
fn map_map_define() {
    const SCRIPT: &str = r#"address = {
  'owner': 'cole',
  age: 30,
  contacts: [
    {
      name: 'cassandra',
      phoneNumber: '0000000'
    },
    {
      name: 'cole',
      phoneNumber: '1111111'
    }
  ]
};
assert(address['owner'] == 'cole');
assert(address['age'] == 30);
assert(address.contacts[0].phoneNumber == '0000000');

List addressBook = [address, {owner: 'john'}];

assert(addressBook[0].owner == 'cole');
assert(addressBook[1].owner == 'john');

empty = {:};
assert(empty.a == null);

extra_comma_map = {
    "test_id" : "acd",
    "cc_id"   : "ttt",
}
assert(extra_comma_map.test_id == "acd")"#;
    expect_ok(SCRIPT);
}

/// 对应 Java testsuite 脚本 `testsuite/independent/map/string_literal_as_field_access.ql`。
#[test]
fn map_string_literal_as_field_access() {
    const SCRIPT: &str = r#"assert({"门店 test": 1234}.'门店 test' == 1234)

a = {"门店 test": 1234, "a b c d": 'oopp'}
assert(a.'门店 test' == 1234)
assert(a.'a b c d' == 'oopp')

"#;
    expect_ok(SCRIPT);
}

/// 对应 Java testsuite 脚本 `testsuite/independent/newlines/newlines.ql`。
#[test]
fn newlines_newlines() {
    const SCRIPT: &str = r"function testAdd(
    int a, int b
    , int c, int d,
    int e
) {
}

a = (int a
     , int b, int c,
     int d, int e) ->
    {
        a
    }

try {
} catch (int a) {
} catch (Object b) {
} finally {
}

assert(
    a(1,2
    ,3,
    4,5,6) ==
    1
)

m =
[
    1,2
    ,3
    ,4,5,
]

assert(m[
  1
  :
  3
]==[2,3])

assert(m[
    2 +
    1
] == 4)

new ArrayList(
    10
)

a = 3
b = new int[
    a+1
][
    9
]

c = new int[] {
    1,2
    ,3,4,
    5
}
assert(c[2]==3)

int ddd =
c[3],
eee = 10
, mmm = 90;

Map<
    String
    ,Map<String,
        List<String>
    >
> map =
{
    'aaa':
    {'bbb':['ccc']}
};

Map<
> map2 = {
  :
};

f = true ?
    10:
    11
assert(f==10)";
    expect_ok(SCRIPT);
}

/// 对应 Java testsuite 脚本 `testsuite/independent/number/number.ql`。
#[test]
fn number_number() {
    const SCRIPT: &str = r"assert(-1==-1)
assert(17 == 0x11);
assert(0x11 == 021);
assert(021 == 0b10001);
assert(-17 == -0x11);
assert(-0x11 == -021);
assert(-021 == -0b10001);
assert(.0 == 0.);
assert(1l == 1L);
assert(1f == 1F);
assert(13.45d == 1.345e1);
assert(13.45d == 1.345e+1);
assert(13.45d == 134.5e-1);
assert(0 == 0);";
    expect_ok(SCRIPT);
}

/// 对应 Java testsuite 脚本 `testsuite/independent/number/precise.ql`。
#[test]
fn number_precise() {
    const SCRIPT: &str = r"assert(123456789.123456789+987654321.987654321==1111111111.11111111)
assert(123456789012345678901234567890*987654321098765432109876543210==121932631137021795226185032733622923332237463801111263526900)
assert(123456789.123456789/0.000000001==123456789123456789)
assert((123456789.123456789 + 987654321.987654321) * (1 - 0.000000001) / 2==555555554.999999999444444445)";
    expect_ok(SCRIPT);
}

/// 对应 Java testsuite 脚本 `testsuite/independent/operator/big_decimal.ql`。
#[test]
fn operator_big_decimal() {
    const SCRIPT: &str = r#"/*
{
  "qlOptions": QLOptions.builder().precise(true)
}
*/

// plus
x = 0.1 + 1.1;
assert(x instanceof BigDecimal)
assert(x == 1.2);

x = 3 + 2.2
assert(x == 5.2)
assert(x instanceof BigDecimal)

x = 2.2 + 4
assert(x instanceof BigDecimal)
assert(x == 6.2)

y = x + 1
assert(y instanceof BigDecimal)
assert(y == 7.2)

z = y + x + 1 + 2
assert(z instanceof BigDecimal)
assert(z == 16.4)

// minus
x = 1.1 - 0.01
assert(x == 1.09)

x = 6 - 2.2
assert(x == 3.8)

x = 5.8 - 2
assert(x == 3.8)

y = x - 1
assert(y == 2.8)

// multiply
x = 3 * 2.0
assert(x == 6.0)

x = 3.0 * 2
assert(x == 6.0)

x = 3.0 * 2.0
assert(x == 6.0)

y = x * 2
assert(y == 12.0)

y = 11 * 3.333
assert(y == 36.663)

y = 3.333 * 11
assert(y == 36.663)

// divide
x = 80.0 / 4
assert(x == 20.0 , "x = " + x)

x = 80 / 4.0
assert(x == 20.0 , "x = " + x)

y = x / 2
assert(y == 10.0 , "y = " + y)
assert(y == 10 , "y = " + y)

y = 34 / 3.000;
assert(y == 11.3333333333);

y = 34.00000000000 / 3;
assert(y == 11.3333333333);

// remainder
x = 100.0 % 3
assert(x == 1)

y = 5.5
y %= 2.0
assert(y == 1.5)

y = -5.5
y %= 2.0
assert(y == -1.5)"#;
    expect_ok_with(SCRIPT, &QLOptions::builder().precise(true).build());
}

/// 对应 Java testsuite 脚本 `testsuite/independent/operator/big_integer.ql`。
#[test]
fn operator_big_integer() {
    const SCRIPT: &str = r"// assign
BigInteger bi;
bi = (byte) 20;
assert(bi instanceof BigInteger);
assert(bi == 20);

bi = (short) 20
assert(bi instanceof BigInteger)
assert(bi == 20)

bi = (int) 20
assert(bi instanceof BigInteger)
assert(bi == 20)

bi = (long) 20
assert(bi instanceof BigInteger)
assert(bi == 20)

bi = (float) 0.5f
assert(bi instanceof BigInteger)
assert(bi == 0)

bi = (double) 0.5d
assert(bi instanceof BigInteger)
assert(bi == 0)

bi = 10.5
assert(bi instanceof BigInteger)
assert(bi == 10)

double d;
d = 1000;
d *= d
d *= d
d *= d
assert((long)d != d)
assert((BigInteger) d == d)

// plus
x = BigInteger.valueOf(2) + BigInteger.valueOf(3)
assert(x instanceof BigInteger)
assert(x == 5)

// multiply
x = BigInteger.valueOf(2) * BigInteger.valueOf(3)
assert(x instanceof BigInteger)
assert(x == 6)

// remainder
x = BigInteger.valueOf(100) % 3
assert(x == 1)

y = BigInteger.valueOf(11)
y %= 3
assert(y == 2)
y = BigInteger.valueOf(-11)
y %= 3
assert(y == -2)";
    expect_ok(SCRIPT);
}

/// 对应 Java testsuite 脚本 `testsuite/independent/operator/bitwise.ql`。
#[test]
fn operator_bitwise() {
    const SCRIPT: &str = r#"assert(true & true);
assertFalse(true & false);
assertFalse(true & null);

assertFalse(false & true);
assertFalse(false & false);
assertFalse(false & null);

assert(true | true);
assert(true | false);
assert(true | null);

assert(false | true);
assertFalse(false | false);
assertFalse(false | null);

assertFalse(true ^ true);
assert(true ^ false);
assert(true ^ null);

assert(false ^ true);
assertFalse(false ^ false);
assertFalse(false ^ null);

// bitwise shift
a = 4;
b = -4;
assert(a << 1 == 8);
assert(a << 2 == 16);
assert(a >> 1 == 2);
assert(a >> 2 == 1);
assert(a >>> 1 == 2);
assert(a >>> 2 == 1);
assert(b << 1 == -8);
assert(b << 2 == -16);
assert(b >> 1 == -2);
assert(b >> 2 == -1);
assert(b >>> 1 == 0x7FFFFFFE);
assert(b >>> 2 == 0x3FFFFFFF);

assertErrorCode(()-> {8.0F >> 2}, "EXECUTE_OPERATOR_EXCEPTION")
assertErrorCode(()-> {8 >> 2.0}, "EXECUTE_OPERATOR_EXCEPTION")

// bitwise shift equal
a = 4;
a <<= 1;
assert(a == 8);
a <<= 2;
assert(a == 32);
a >>= 1;
assert(a == 16);
a >>= 2;
assert(a == 4);

b = -4;
b <<= 1;
assert(b == -8);
b <<= 2;
assert(b == -32);
b >>= 1;
assert(b == -16);
b >>= 2;
assert(b == -4);

b = -4;
b >>>= 1;
assert(b == 0x7FFFFFFE);
b = -8;
b >>>= 2;
assert(b == 0x3FFFFFFE);

// bitwise and
a = 13;
assert((a & 3) == 1); // 0x0000000D & 0x00000003
assert((a & 7) == 5); // 0x0000000D & 0x00000007
b = -13;
assert((b & 3) == 3); // 0xFFFFFFF3 & 0x00000003
assert((b & 7) == 3); // 0xFFFFFFF3 & 0x00000007

// bitwise and equals
a = 13;
a &= 3;
assert(a == 1); // 0x0000000D & 0x00000003

a &= 4;
assert(a == 0); // 0x00000001 & 0x00000004

b = -13;
b &= 3;
assert(b == 3); // 0xFFFFFFF3 & 0x00000003

b &= 7;
assert(b == 3); // 0x00000003 & 0x00000007

// bitwise or
a = 13;
assert((a | 8) == 13);   // 0x0000000D | 0x00000008
assert((a | 16) == 29);  // 0x0000000D | 0x00000010
b = -13;
assert((b | 8) == -5);   // 0xFFFFFFF3 | 0x00000008
assert((b | 16) == -13); // 0xFFFFFFF3 | 0x00000010

// bitwise or equal
a = 13;
a |= 2;
assert(a == 15); // 0x0000000D | 0x00000002
a |= 16;
assert(a == 31); // 0x0000000F | 0x0000001F
b = -13;
b |= 8;
assert(b == -5); // 0xFFFFFFF3 | 0x00000008
b |= 1;
assert(b == -5); // 0xFFFFFFFB | 0x00000001

// bitwise xor
a = 13;
assert((a ^ 10) == 7); // 0x0000000D ^ 0x0000000A = 0x000000007
assert((a ^ 15) == 2); // 0x0000000D ^ 0x0000000F = 0x000000002
b = -13;
assert((b ^ 10) == -7); // 0xFFFFFFF3 ^ 0x0000000A = 0xFFFFFFF9
assert((b ^ 15) == -4); // 0xFFFFFFF3 ^ 0x0000000F = 0xFFFFFFFC

// bitwise xor equal
a = 13;
a ^= 8;
assert(a == 5); // 0x0000000D ^ 0x00000008 = 0x000000005
a ^= 16
assert(a == 21); // 0x00000005 ^ 0x00000010 = 0x000000015
b = -13;
b ^= 8;
assert(b == -5); // 0xFFFFFFF3 ^ 0x00000008 = 0xFFFFFFFB
b ^= 16;
assert(b == -21); // 0xFFFFFFFB ^ 0x00000010 = 0xFFFFFFEB

// bitwise negation
assert(~1 == -2); // ~0x00000001 = 0xFFFFFFFE
assert(~(-1) == 0); // ~0xFFFFFFFF = 0x00000000
assert(~(~5) == 5); // ~~0x00000005 = ~0xFFFFFFFA = 0xFFFFFFF5
a = 13;
assert(~a == -14); // ~0x0000000D = 0xFFFFFFF2
assert(~(~a) == 13); // ~~0x0000000D = ~0xFFFFFFF2 = 0x0000000D
assert(-(~a) == 14); // -~0x0000000D = -0xFFFFFFF2 = 0x0000000E

// bitwise exception
assertErrorCode(()-> {~"8"}, "INVALID_UNARY_OPERAND")"#;
    expect_ok(SCRIPT);
}

/// 对应 Java testsuite 脚本 `testsuite/independent/operator/boolean.ql`。
#[test]
fn operator_boolean() {
    const SCRIPT: &str = r"// comparison
assert(true)
assert(true != false)

x = true
assert(x)
assert(x == true)
assert(x != false)

x = false
assert(x == false)
assert(x != true)
assert(!x)

y = false
assert(x == y)

y = true
assert(x != y)

// if branch
x = false
r = false
if (x) {
    // ignore
}
else {
    r = true
}
assert(r)

x = true
r = false
if (x) {
    r = true
}
else {
    // ignore
}
assert(r)

if (!x) {
    r = false
}
else {
    r = true
}
assert(r)

// expression
x = 5
value = x > 2
assert(value)

value = x < 2
assert(value == false)

// ops
boolean x = true;
boolean y = false;
assert((x & x) == true)
assert((x & y) == false)
assert((y & x) == false)
assert((y & y) == false)

assert((x | x) == true)
assert((x | y) == true)
assert((y | x) == true)
assert((y | y) == false)

assert((x ^ x) == false)
assert((x ^ y) == true)
assert((y ^ x) == true)
assert((y ^ y) == false)

assert((!x) == false)
assert((!y) == true)

// assign ops
boolean z = true;
z &= true
assert(z == true)
z &= false
assert(z == false)

z = true
z |= true
assert(z == true)
z |= false
assert(z == true)
z = false
z |= false
assert(z == false)
z |= true
assert(z == true)

z = true
z ^= true
assert(z == false)
z ^= true
assert(z == true)
z ^= false
assert(z == true)
z ^= true
assert(z == false)
z ^= false
assert(z == false)";
    expect_ok(SCRIPT);
}

/// 对应 Java testsuite 脚本 `testsuite/independent/operator/character.ql`。
#[test]
fn operator_character() {
    const SCRIPT: &str = r#"assert(98 > (char)'a');
assert(98 == (char)'b');
assert(98 < (char)'c');

assert((char)'a' < 98);
assert((char)'b' == 98);
assert((char)'c' > 98);

assert(98 != (char)'a');
assert(98 <> (char)'a');
assert((char)'a' != 98);
assert((char)'a' <> 98);

assert((char)'a' < (char)'b');
assert((char)'b' == (char)'b');
assert((char)'c' > (char)'b');

assertErrorCode(() -> {'测试一下' > 1}, "INVALID_BINARY_OPERAND")"#;
    expect_ok(SCRIPT);
}

/// 对应 Java testsuite 脚本 `testsuite/independent/operator/comparable.ql`。
#[test]
fn operator_comparable() {
    const SCRIPT: &str = r"import com.alibaba.qlexpress4.inport.Person;

p1 = new Person(10)
p2 = new Person(20)
assert(p1 < p2)
assert(p1 <= p2)
assert(p1 != p2)
assert(p1 <> p2)

assert(p2 > p1)
assert(p2 >= p1)
assert(p2 != p1)
assert(p2 <> p1)";
    expect_ok(SCRIPT);
}

/// 对应 Java testsuite 脚本 `testsuite/independent/operator/double.ql`。
#[test]
fn operator_double() {
    const SCRIPT: &str = r#"// plus
x = 2.1d + 2.1d
assert(x == 4.2d)

x = 3d + 2.2d
assert(x == 5.2d)

x = 2.2d + 4d
assert(x == 6.2d)

y = x + 1d
assert(y == 7.2d)

z = y + x + 1d + 2d
assert(z == 16.4d)

// minus
x = 6d - 2.2d
assert(x == 3.8d)

x = 5.8d - 2d
assert(x == 3.8d)

y = x - 1d
assert(y == 2.8d)

// multiply
x = 3d * 2.0d
assert(x == 6.0d)

x = 3.0d * 2d
assert(x == 6.0d)

x = 3.0d * 2.0d
assert(x == 6.0d)
y = x * 2d
assert(y == 12.0d)

// divide
x = 80.0d / 4d
assert(x == 20.0d, "x = " + x)

x = 80d / 4.0d
assert(x == 20.0d, "x = " + x)

y = x / 2d
assert(y == 10.0d, "y = " + y)

// remainder
x = 100d % 3
assert(x == 1d)

y = 11d
y %= 3d
assert(y == 2d)
y = -11d
y %= 3d
assert(y == -2d)
"#;
    expect_ok(SCRIPT);
}

/// 对应 Java testsuite 脚本 `testsuite/independent/operator/equals.ql`。
#[test]
fn operator_equals() {
    const SCRIPT: &str = r"assert(0 == 0);
assert(512 == 512);
assert(512 == 512L);
assert(512 == 512F);
assert(512 == 512D);

assert(512L == 512L);
assert(512L == 512F);
assert(512L == 512D);

assert(512F == 512F);
assert(512F == 512D);

assert(512D == 512D);

assertFalse(512 == 513);

assert((char)'a' == 97);
assert(97 == (char)'a');
assertFalse((char)'b' == 97);
assertFalse(97 == (char)'b');

assert((char)'b' != 97);
assert(97 != (char)'b');
assertFalse((char)'a' != 97);
assertFalse(97 != (char)'a');

assert((char)'b' <> 97);
assert(97 <> (char)'b');
assertFalse((char)'a' <> 97);
assertFalse(97 <> (char)'a');

assert(null == null);
assertFalse(null != null);
";
    expect_ok(SCRIPT);
}

/// 对应 Java testsuite 脚本 `testsuite/independent/operator/in_not_in.ql`。
#[test]
fn operator_in_not_in() {
    const SCRIPT: &str = r#"assert(null in null);
assertFalse(null in "abc");
assertFalse(null in 123);

assertFalse(null not_in null);
assert(null not_in "abc");
assert(null not_in 123);

assertFalse(null in [1, 2, 3]);
assertFalse(null in new int[]{1, 2, 3});

assert(null not_in [1, 2, 3]);
assert(null not_in new int[]{1, 2, 3});

assertFalse(null in ["abc", "def", "ghi"]);
assertFalse(null in new String[]{"abc", "def", "ghi"});

assert(null not_in ["abc", "def", "ghi"]);
assert(null not_in new String[]{"abc", "def", "ghi"});

assert(1 in [1, 2, 3]);
assert(1 in new int[]{1, 2, 3});

assertFalse(1 not_in [1, 2, 3]);
assertFalse(1 not_in new int[]{1, 2, 3});

assertFalse(1 in ["1", "2", "3"]);
assertFalse(1 in new String[]{"1", "2", "3"});

assert(1 not_in ["1", "2", "3"]);
assert(1 not_in new String[]{"1", "2", "3"});

assert("abc" in ["abc", "def", "ghi"]);
assert("abc" in new String[]{"abc", "def", "ghi"});

assertFalse("abc" not_in ["abc", "def", "gcpghi"]);
assertFalse("abc" not_in new String[]{"abc", "def", "ghi"});

assertFalse("bcd" in ["abc", "def", "ghi"]);
assertFalse("bcd" in new String[]{"abc", "def", "ghi"});

assert("bcd" not_in ["abc", "def", "ghi"]);
assert("bcd" not_in new String[]{"abc", "def", "ghi"});

assert("bc" in "abc");
assert("bc" in "bcd");
assert("bc" in "abcd");
assertFalse("bc" in "ab");
assertFalse("bc" in "cd");
assertFalse("bc" in "abd");
assertFalse("bc" in "acd");

assertFalse("bc" not_in "abc");
assertFalse("bc" not_in "bcd");
assertFalse("bc" not_in "abcd");
assert("bc" not_in "ab");
assert("bc" not_in "cd");
assert("bc" not_in "abd");
assert("bc" not_in "acd");"#;
    expect_ok(SCRIPT);
}

/// 对应 Java testsuite 脚本 `testsuite/independent/operator/integer.ql`。
#[test]
fn operator_integer() {
    const SCRIPT: &str = r"// plus
x = 2 + 2
assert(x == 4)

y = x + 1
assert(y == 5)

z = y + x + 1 + 2
assert(z == 12)

// unary plus
x = 3
y = +x
assert(y == 3)

// character plus
char c1 = 1;
char c2 = 2;

x = c2 + 2
assert(x == 4)

x = 2 + c2
assert(x == 4)

x = c2 + c2
assert(x == 4)

y = x + c1
assert(y == 5)

y = c1 + x
assert(y == 5)

z = y + x + c1 + 2
assert(z == 12)

z = y + x + 1 + c2
assert(z == 12)

z = y + x + c1 + c2
assert(z == 12)

// minus
x = 6 - 2
assert(x == 4)

x = 6
x -= 2
assert(x == 4)

y = x - 1
assert(y == 3)

// unary minus
x = 3
y = -x
assert(y == -3)

// bitwise negate
x = 3
y = ~x
assert(y == -4)

// character minus
Character c1 = 1;
Character c2 = 2;
Character c6 = 6;

x = c6 - 2
assert(x == 4)

x = 6 - c2
assert(x == 4)

x = c6 - c2
assert(x == 4)

y = x - c1
assert(y == 3)

// multiply
x = 3 * 2
assert(x == 6)

y = x * 2
assert(y == 12)

// divide
x = 80 / 4
assert(x == 20.0)

x = 80
x /= 4
assert(x == 20.0)

y = x / 2
assert(y == 10.0)

// remainder
x = 100 % 3
assert(x == 1)

y = 11
y %= 3
assert(y == 2)

y = -11
y %= 3
assert(y == -2)

// and
x = 1 & 3
assert(x == 1)

// or
x = 1 | 3
assert(x == 3)

// shift operator
x = 8 >> 1
assert(x == 4)
assert(x instanceof Integer)

x = 8 << 2
assert(x == 32)
assert(x instanceof Integer)

x = 8L << 2
assert(x == 32)
assert(x instanceof Long)

x = -16 >> 4
assert(x == -1)

x = -16 >>> 4
assert(x == 0xFFFFFFF)

//Ensure that the type of the right operand (shift distance) is ignored when calculating the
//result.  This is how java works, and for these operators, it makes sense to keep that behavior.
x = Integer.MAX_VALUE << 1L
assert(x == -2)
assert(x instanceof Integer)

x = new Long(Integer.MAX_VALUE).longValue() << 1
assert(x == 0xfffffffe)
assert(x instanceof Long)";
    expect_ok(SCRIPT);
}

/// 对应 Java testsuite 脚本 `testsuite/independent/operator/like.ql`。
#[test]
fn operator_like() {
    const SCRIPT: &str = r#"assert(null like null);
assertFalse(null not_like null);

assertFalse("a" like null);
assert("a" not_like null);

assert("1006" like "%6");
assert("1006" like "1%");
assert("ABCD" like "A%B%D");

assertFalse("1006" not_like "%6");
assertFalse("1006" not_like "1%");
assertFalse("ABCD" not_like "A%B%D");

// error code
assertErrorCode(() -> {"ABCD" like 200}, "INVALID_BINARY_OPERAND");
assertErrorCode(() -> {200 like "200"}, "INVALID_BINARY_OPERAND");

assertErrorCode(() -> {"ABCD" not_like 200}, "INVALID_BINARY_OPERAND");
assertErrorCode(() -> {200 not_like "200"}, "INVALID_BINARY_OPERAND");"#;
    expect_ok(SCRIPT);
}

/// 对应 Java testsuite 脚本 `testsuite/independent/operator/logic.ql`。
#[test]
fn operator_logic() {
    const SCRIPT: &str = r"assert(true && true);
assertFalse(true && false);
assertFalse(true && null);
assert(true and true);
assertFalse(true and false);
assertFalse(true and null);

assertFalse(false && true);
assertFalse(false && false);
assertFalse(false && null);

assertFalse(null && true);
assertFalse(null && false);
assertFalse(null && null);
";
    expect_ok(SCRIPT);
}

/// 对应 Java testsuite 脚本 `testsuite/independent/operator/optional_chaining.ql`。
#[test]
fn operator_optional_chaining() {
    const SCRIPT: &str = r"assert(a?.b?.c?.d == null);

try {
    assert(a?.b.c == null);
    throw new RuntimeException();
} catch (e) {
    assert(e instanceof NullPointerException);
}

mm = {cc: 123}

assert(mm?.cc == 123);
assert(mm.dd?.ee == null);
assert(mm.dd?.test() == null);";
    expect_ok(SCRIPT);
}

/// 对应 Java testsuite 脚本 `testsuite/independent/operator/string.ql`。
#[test]
fn operator_string() {
    const SCRIPT: &str = r#"x = "hello " + "there"
assert(x == "hello there")

x = "hello " + 2
assert(x == "hello 2")

x = "hello " + 1.2
assert(x == "hello 1.2")

y = x + 1
assert(y == "hello 1.21")

x = "hello" + " " + "there" + " nice" + " day"
assert(x == "hello there nice day")

assert("bc" > "ab")
assert("bc" > "ab")
assert("bcd" > "ab")
assert("bcd" > "abc")"#;
    expect_ok(SCRIPT);
}

/// 对应 Java testsuite 脚本 `testsuite/independent/scope/block_scope.ql`。
#[test]
fn scope_block_scope() {
    const SCRIPT: &str = r#"{
  int b = 12;
  a = 100
}
assert(a + "-" + b == '100-null');

"#;
    expect_ok(SCRIPT);
}

/// 对应 Java testsuite 脚本 `testsuite/independent/scope/global_variable.ql`。
#[test]
fn scope_global_variable() {
    const SCRIPT: &str = r"function setA(value) {
    a = value;
}

a = 10;
assert(a == 10);
setA(10000);
assert(a == 10000);";
    expect_ok(SCRIPT);
}

/// 对应 Java testsuite 脚本 `testsuite/independent/scope/lexical_scope.ql`。
#[test]
fn scope_lexical_scope() {
    const SCRIPT: &str = r#"String a = "lexical first";

aFactory = () -> a;

{
    String a = "runtime first-block";
    assert(aFactory() == "lexical first");
}

function testScope(a) {
    assert(aFactory() == "lexical first");
}

testScope("runtime first-function");"#;
    expect_ok(SCRIPT);
}

/// 对应 Java testsuite 脚本 `testsuite/independent/scope/scope_cover.ql`。
#[test]
fn scope_scope_cover() {
    const SCRIPT: &str = r"a = 10;

assert(a == 10);

{
    int a = 100;
    assert(a == 100);
}

assert(a == 10);";
    expect_ok(SCRIPT);
}

/// 对应 Java testsuite 脚本 `testsuite/independent/spread/arr_spread.ql`。
#[test]
fn spread_arr_spread() {
    const SCRIPT: &str = r#"Map[] arr = new Map[]{{"a":1},{"a":2}};
assert(arr*.a==[1,2])

Map[] arr1 = new Map[]{{"a":1},{"a":2}, null};
assertErrorCode(() -> arr1*.a, "NULL_FIELD_ACCESS")

Map[] b = new Map[]{{"get100": () -> 100}, null};
assertErrorCode(() -> b*.get100(), "NULL_METHOD_ACCESS")"#;
    expect_ok(SCRIPT);
}

/// 对应 Java testsuite 脚本 `testsuite/independent/spread/list_spread.ql`。
#[test]
fn spread_list_spread() {
    const SCRIPT: &str = r#"// tag::spreadExample[]
list = [
  {
    "name": "Li",
    "age": 10
  },
  {
    "name": "Wang",
    "age": 15
  }
]

// get field from list
assert(list*.age==[10,15])

mm = {
  "aaa": 1,
  "bbb": 2
}

// get map key value list
assert(mm*.key==["aaa", "bbb"])
assert(mm*.value==[1, 2])
// end::spreadExample[]

methodMaps = [
  {
    "getNum": () -> 100,
  },
  {
    "getNum": () -> 101,
  },
  {
    "getNum": () -> 102,
  }
]

assert(methodMaps*.getNum()==[100,101,102])

a = [{"c":2}, null]
assertErrorCode(() -> a*.c, "NULL_FIELD_ACCESS")
assertErrorCode(() -> notExist*.c, "NONTRAVERSABLE_OBJECT")

b = [{"get100": () -> 100}, null]
assertErrorCode(() -> b*.get100(), "NULL_METHOD_ACCESS")
assertErrorCode(() -> notExist*.get100(), "NONTRAVERSABLE_OBJECT")"#;
    expect_ok(SCRIPT);
}

/// 对应 Java testsuite 脚本 `testsuite/independent/spread/nested_list_spread.ql`。
#[test]
fn spread_nested_list_spread() {
    const SCRIPT: &str = r#"// tag::nestedSpreadExample[]
// Use case 1: Spread nested list to get field values
l = [[{"a":10},{"a":12}],[{"a":13}],[{"a":14}]]
result = l*.a
assert(result == [10, 12, 13, 14])

// Use case 2: Spread nested list to call methods
l2 = [[{"a":10},{"a":12}],[{"a":13}],[{"a":14}]]
result2 = l2*.get("a")
assert(result2 == [10, 12, 13, 14])

// Use case 3: if field exists at current level, don't flatten
l3 = [[{"a":10},{"a":12}],[{"a":13}],[{"a":14}]]
result3 = l3*.length
assert(result3 == [2, 1, 1])
// end::nestedSpreadExample[]

// Additional test: nested arrays
arr = [[1, 2], [3, 4, 5]]
// Arrays should also support nested spread
// For now, let's test if arrays have length property
arr_result = arr*.length
assert(arr_result == [2, 3])

// Test with deeper nesting (3 levels)
l4 = [[[{"a":1}], [{"a":2}]], [[{"a":3}]]]
result4 = l4*.a
assert(result4 == [1, 2, 3])
"#;
    expect_ok(SCRIPT);
}

/// 对应 Java testsuite 脚本 `testsuite/independent/spread/spread_avoid_null.ql`。
#[test]
fn spread_spread_avoid_null() {
    const SCRIPT: &str = r#"/*
{
  "qlOptions": QLOptions.builder().avoidNullPointer(true)
}
*/
a = [{"c":2}, null]
assert(a*.c==[2, null])
assert(notExist*.c==null)
assert(notExist*.c()==null)

b = [{"get100": () -> 100}, null]
assert(b*.get100()==[100, null])

Map[] arr1 = new Map[]{{"a":1},{"a":2}, null};
assert(arr1*.a==[1,2,null])

Map[] brr = new Map[]{{"get100": () -> 100}, null};
assert(brr*.get100()==[100,null])"#;
    expect_ok_with(
        SCRIPT,
        &QLOptions::builder().avoid_null_pointer(true).build(),
    );
}

/// 对应 Java testsuite 脚本 `testsuite/independent/string/char.ql`。
#[test]
fn string_char() {
    const SCRIPT: &str = r#"char a = 'a';
char b = "a";
assert(a instanceof Character);
assert(b instanceof Character);
assert(a == b);"#;
    expect_ok(SCRIPT);
}

/// 对应 Java testsuite 脚本 `testsuite/independent/string/interpolation.ql`。
#[test]
fn string_interpolation() {
    const SCRIPT: &str = r#"a = 123;
b = "test"

assert("Hello ${a} ${b } ccc" == "Hello 123 test ccc");
// $ escape
assert("Hello \${a bb cc" == 'Hello ${a bb cc')
// selector variable
assert(${a} == 123)

assert("${a-1}" == "122")

assert("m xx ${
  if (b like 't%') {
      'YYY'
  }
}" == "m xx YYY")

assert("m xx ${
  if (b like 't%') {
      "YYY"
  }
}" == "m xx YYY")

// nest interpolation
assert("m xx ${
  if (b like 't%') {
      "YY${b}Y"
  }
}" == "m xx YYtestY")

assert("m xx ${
  if (b like 'mm%') {
      'YYY'
  }
}" == "m xx null")


"#;
    expect_ok(SCRIPT);
}

/// 对应 Java testsuite 脚本 `testsuite/independent/string/invalid_char.ql`。
#[test]
fn string_invalid_char() {
    const SCRIPT: &str = r#"/*
{
  "errCode": "INCOMPATIBLE_ASSIGNMENT_TYPE"
}
*/
char a = 'aa';
println(a);"#;
    expect_err_code(SCRIPT, "INCOMPATIBLE_ASSIGNMENT_TYPE");
}

/// 对应 Java testsuite 脚本 `testsuite/independent/string/literal.ql`。
#[test]
fn string_literal() {
    const SCRIPT: &str = r#"assert('Hello World' == "Hello World")"#;
    expect_ok(SCRIPT);
}

/// 对应 Java testsuite 脚本 `testsuite/independent/string/string_escape.ql`。
#[test]
fn string_string_escape() {
    const SCRIPT: &str = r#"assert('\' \\r \'' == "' \\r '")

assert('hello
world' == "hello\nworld")

a = "hello
qlexpress"
assert(a == "hello\nqlexpress")

assert("hello

qlexpress" == "hello
\nqlexpress")"#;
    expect_ok(SCRIPT);
}

/// 对应 Java testsuite 脚本 `testsuite/independent/string/string_not_close.ql`。
#[test]
fn string_string_not_close() {
    const SCRIPT: &str = r#"/*
{
  "errCode": "SYNTAX_ERROR"
}
*/
a = "abc"#;
    expect_err_code(SCRIPT, "SYNTAX_ERROR");
}

/// 对应 Java testsuite 脚本 `testsuite/independent/switch/switch_basic.ql`。
#[test]
fn switch_switch_basic() {
    const SCRIPT: &str = r#"int day = 3;
String dayName;

switch (day) {
  case 1:
    dayName = "Monday";
    break;
  case 2:
    dayName = "Tuesday";
    break;
  case 3:
    dayName = "Wednesday";
    break;
  case 4:
    dayName = "Thursday";
    break;
  case 5:
    dayName = "Friday";
    break;
  default:
    dayName = "Weekend";
}

assert(dayName == "Wednesday");

// test default
int num = 10;
String result;
switch (num) {
  case 1:
    result = "one";
    break;
  case 2:
    result = "two";
    break;
  default:
    result = "other";
}

assert(result == "other");"#;
    expect_ok(SCRIPT);
}

/// 对应 Java testsuite 脚本 `testsuite/independent/switch/switch_expression_basic.ql`。
#[test]
fn switch_switch_expression_basic() {
    const SCRIPT: &str = r#"// Basic switch expression with arrow syntax
// tag::switchExpression[]
score = 70
result = switch (score) {
    case 90, 100 -> "优秀"
    case 60, 70, 80 -> "及格"
    default -> "不及格"
}
assert(result == "及格")
// end::switchExpression[]

// Test with numbers
num = 2
result2 = switch (num) {
    case 1 -> 10
    case 2 -> 20
    case 3 -> 30
    default -> 0
}
assert(result2 == 20)

// Test with default
num3 = 999
result3 = switch (num3) {
    case 1, 2, 3 -> "small"
    default -> "large"
}
assert(result3 == "large")

// Test single case value
status = 1
result4 = switch (status) {
    case 0 -> "off"
    case 1 -> "on"
    default -> "unknown"
}
assert(result4 == "on")
"#;
    expect_ok(SCRIPT);
}

/// 对应 Java testsuite 脚本 `testsuite/independent/switch/switch_expression_nested.ql`。
#[test]
fn switch_switch_expression_nested() {
    const SCRIPT: &str = r#"// Nested switch expressions
x = 1
y = 2

result = switch (x) {
    case 1 -> switch (y) {
        case 1 -> "1-1"
        case 2 -> "1-2"
        default -> "1-other"
    }
    case 2 -> "2"
    default -> "other"
}

assert(result == "1-2")
"#;
    expect_ok(SCRIPT);
}

/// 对应 Java testsuite 脚本 `testsuite/independent/switch/switch_fallthrough.ql`。
#[test]
fn switch_switch_fallthrough() {
    const SCRIPT: &str = r#"/*
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

// Test multiple cases sharing a code block
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
    expect_null(SCRIPT);
}

/// 对应 Java testsuite 脚本 `testsuite/independent/switch/switch_nested.ql`。
#[test]
fn switch_switch_nested() {
    const SCRIPT: &str = r#"// nested switch
int x = 1;
int y = 2;
String result;

switch (x) {
  case 1:
    switch (y) {
      case 1:
        result = "1-1";
        break;
      case 2:
        result = "1-2";
        break;
      default:
        result = "1-other";
    }
    break;
  case 2:
    result = "two";
    break;
  default:
    result = "other";
}

assert(result == "1-2");

// switch nested in if
int a = 5;
String msg;

if (a > 0) {
  switch (a) {
    case 1:
      msg = "one";
      break;
    case 5:
      msg = "five";
      break;
    default:
      msg = "other";
  }
} else {
  msg = "negative";
}

assert(msg == "five");
"#;
    expect_ok(SCRIPT);
}

/// 对应 Java testsuite 脚本 `testsuite/independent/ternary/missing_colon.ql`。
#[test]
fn ternary_missing_colon() {
    const SCRIPT: &str = r#"/*
{
  "errCode": "SYNTAX_ERROR"
}
*/
a = x > 10? 10;"#;
    expect_err_code(SCRIPT, "SYNTAX_ERROR");
}

/// 对应 Java testsuite 脚本 `testsuite/independent/ternary/ternary.ql`。
#[test]
fn ternary_ternary() {
    const SCRIPT: &str = r"l = (x) -> x > 10? 11: 100;
assert(l(11) == 11);
assert(l(5) == 100);

l1 = (x) -> x > 100? 101: x > 50? 51: 11;
assert(l1(120) == 101);
assert(l1(99) == 51);
assert(l1(15) == 11);

l2 = x -> x <= 10? -9: x < 20? 19: 11;
assert(l2(1) == -9);
assert(l2(17) == 19);
assert(l2(29) == 11);

l3 = true? a = 100: b = 200;
assert(a == 100);
assert(b == null);";
    expect_ok(SCRIPT);
}

/// 对应 Java testsuite 脚本 `testsuite/independent/timeout/timeout.ql`。
#[test]
fn timeout_timeout() {
    const SCRIPT: &str = r#"/*
{
  "qlOptions": QLOptions.builder().timeoutMillis(10),
  "errCode": "SCRIPT_TIME_OUT"
}
*/
while (true) {
  1+1
}"#;
    expect_err_code_with(
        SCRIPT,
        &QLOptions::builder().timeout_millis(10).build(),
        "SCRIPT_TIME_OUT",
    );
}

/// 对应 Java testsuite 脚本 `testsuite/independent/trycatch/catch_order.ql`。
#[test]
fn trycatch_catch_order() {
    const SCRIPT: &str = r"a = try {
    throw 10;
} catch (int a) {
    100
} catch (int b) {
    1000
}

assert(a == 100)";
    expect_ok(SCRIPT);
}

/// 对应 Java testsuite 脚本 `testsuite/independent/trycatch/missing_lbrace_at_try.ql`。
#[test]
fn trycatch_missing_lbrace_at_try() {
    const SCRIPT: &str = r#"/*
{
  "errCode": "SYNTAX_ERROR"
}
*/
try 1+1"#;
    expect_err_code(SCRIPT, "SYNTAX_ERROR");
}

/// 对应 Java testsuite 脚本 `testsuite/independent/trycatch/missing_lbrace_at_try_finally.ql`。
#[test]
fn trycatch_missing_lbrace_at_try_finally() {
    const SCRIPT: &str = r#"/*
{
  "errCode": "SYNTAX_ERROR"
}
*/
try {
  2+1
} catch(Object o) {

} finally }"#;
    expect_err_code(SCRIPT, "SYNTAX_ERROR");
}

/// 对应 Java testsuite 脚本 `testsuite/independent/trycatch/multi_exception_catch.ql`。
#[test]
fn trycatch_multi_exception_catch() {
    const SCRIPT: &str = r"function f(x) {
  try {
    throw x;
  } catch (int | long i) {
    assert(i == x);
  }
}

f(1);

f(100L);

try {
  f(1.1d);
  assert(false);
} catch (double d) {
  assert(d == 1.1d);
}
";
    expect_ok(SCRIPT);
}

/// 对应 Java testsuite 脚本 `testsuite/independent/trycatch/return_from_try.ql`。
#[test]
fn trycatch_return_from_try() {
    const SCRIPT: &str = r"function tryTest() {
    try {
        return 10;
    } catch (ignore) {
    }
    return 1000;
}

assert(tryTest() == 10)

function catchTest() {
    try {
        throw 10;
    } catch (ignore) {
        return 1000;
    }
    return 10000;
}

assert(catchTest()==1000)

function returnInsideFinally() {
    try {
        return 30;
    } catch (ignore) {
    } finally {
        return 9000;
    }
}

assert(returnInsideFinally() == 30)";
    expect_ok(SCRIPT);
}

/// 对应 Java testsuite 脚本 `testsuite/independent/trycatch/throw_number.ql`。
#[test]
fn trycatch_throw_number() {
    const SCRIPT: &str = r"try {
  throw 11;
  assert(false);
} catch(int i) {
  assert(i == 11);
}";
    expect_ok(SCRIPT);
}

/// 对应 Java testsuite 脚本 `testsuite/independent/trycatch/try_catch_expr.ql`。
#[test]
fn trycatch_try_catch_expr() {
    const SCRIPT: &str = r"a = 1 + try {
  100 + 1/0
} catch(Object e) {
  11
};

assert(a == 12);

b = 1 + try {
  100 + 1/0
} catch(Object e) {
  11
} finally {
  1000
};
assert(b == 12);";
    expect_ok(SCRIPT);
}

/// 对应 Java testsuite 脚本 `testsuite/independent/trycatch/try_catch_final_scope.ql`。
#[test]
fn trycatch_try_catch_final_scope() {
    const SCRIPT: &str = r"int a = 10;

try {
  int a = 1000;
  throw new NullPointerException();
  assert(false);
} catch (Object o) {
  assert(a == 10);
} finally {
  assert(a == 10);
}";
    expect_ok(SCRIPT);
}

/// 对应 Java testsuite 脚本 `testsuite/independent/while/break_continue.ql`。
#[test]
fn while_break_continue() {
    const SCRIPT: &str = r"i = 0;
while (i < 5) {
  if (++i == 2) {
    break;
  }
}
assert(i == 2);

sum = 0;
i = 0;
while (i < 5) {
  if (i == 2) {
    i += 1;
    continue;
  }
  sum += i++;
}
assert(sum == 8);";
    expect_ok(SCRIPT);
}

/// 对应 Java testsuite 脚本 `testsuite/independent/while/condition_not_bool.ql`。
#[test]
fn while_condition_not_bool() {
    const SCRIPT: &str = r#"/*
{
  "errCode": "WHILE_CONDITION_BOOL_REQUIRED"
}
*/
while (1) {
  true;
}"#;
    expect_err_code(SCRIPT, "WHILE_CONDITION_BOOL_REQUIRED");
}

/// 对应 Java testsuite 脚本 `testsuite/independent/while/missing_lparen.ql`。
#[test]
fn while_missing_lparen() {
    const SCRIPT: &str = r#"/*
{
  "errCode": "SYNTAX_ERROR"
}
*/
while i < 5 {
}"#;
    expect_err_code(SCRIPT, "SYNTAX_ERROR");
}

/// 对应 Java testsuite 脚本 `testsuite/independent/while/missing_rparen.ql`。
#[test]
fn while_missing_rparen() {
    const SCRIPT: &str = r#"/*
{
  "errCode": "SYNTAX_ERROR"
}
*/
i = 0;
while (i < 5 {
  i++;
}"#;
    expect_err_code(SCRIPT, "SYNTAX_ERROR");
}

/// 对应 Java testsuite 脚本 `testsuite/independent/while/while.ql`。
#[test]
fn while_while() {
    const SCRIPT: &str = r"i = 0;
sum = 0;
// m not in scope
while (i < 4 && m == null) {
  int m = 10;
  sum += (i++);
}
assert(sum==6);";
    expect_ok(SCRIPT);
}
