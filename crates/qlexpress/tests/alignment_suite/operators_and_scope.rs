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
