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
