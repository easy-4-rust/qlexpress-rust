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

/// SOURCE_PARITY: Java `ArrayList.Itr` 对结构修改 fail-fast；旧 Rust
/// 实现错误地克隆列表并继续遍历快照。
#[test]
fn for_each_list_structural_modification_is_fail_fast() {
    let error = alignment_util::suite_runner()
        .execute(
            "l = [1,2]; for (x : l) { l.add(3); }",
            std::collections::HashMap::new(),
            &QLOptions::default(),
        )
        .expect_err("ArrayList structural modification must invalidate iterator");
    assert_eq!(
        error.error_code(),
        "java.util.ConcurrentModificationException"
    );
}

/// SOURCE_PARITY: `ArrayList#set` 不是结构修改，既有迭代器继续读取替换值。
#[test]
fn for_each_list_set_keeps_iterator_valid() {
    let result = alignment_util::suite_runner()
        .execute(
            "l = [1,2]; sum = 0; for (x : l) { if (x == 1) l.set(1,20); sum += x; } sum",
            std::collections::HashMap::new(),
            &QLOptions::default(),
        )
        .expect("ArrayList.set must not invalidate iterator");
    assert_eq!(result.result(), &DataValue::Int(21));
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
