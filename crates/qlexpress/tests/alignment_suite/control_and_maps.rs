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
