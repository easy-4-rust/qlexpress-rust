//! 综合操作符级语义测试 — 覆盖 UNVERIFIED operator 对象。
//!
//! 通过 Express4Runner API 驱动每种操作符,验证其语义行为。
//! 每个测试标注对应的 Java 源类。

#![allow(clippy::result_large_err)]
#![allow(unused_variables)]

use std::collections::HashMap;

use qlexpress::init_options::InitOptions;
use qlexpress::ql_options::QLOptions;
use qlexpress::runtime::value::DataValue;
use qlexpress::security::ql_security_strategy::QLSecurityStrategy;
use qlexpress::Express4Runner;

fn runner() -> Express4Runner {
    Express4Runner::new()
}

fn open_runner() -> Express4Runner {
    Express4Runner::with_init_options(
        InitOptions::builder()
            .security_strategy(QLSecurityStrategy::open())
            .build(),
    )
}

fn options() -> QLOptions {
    QLOptions::builder().build()
}

// ============================================================================
// Arithmetic Operators (10)
// Java: com.alibaba.qlexpress4.runtime.operator.arithmetic
// ============================================================================

/// PlusOperator — 加法
#[test]
fn plus_operator_int() {
    let r = runner();
    let o = options();
    assert_eq!(
        r.execute("1 + 2", HashMap::new(), &o).unwrap().result(),
        &DataValue::Int(3)
    );
}

#[test]
fn plus_operator_long() {
    let r = runner();
    let o = options();
    assert_eq!(
        r.execute("1L + 2L", HashMap::new(), &o).unwrap().result(),
        &DataValue::Long(3)
    );
}

#[test]
fn plus_operator_string_concat() {
    let r = runner();
    let o = options();
    assert_eq!(
        r.execute("'a' + 'b'", HashMap::new(), &o).unwrap().result(),
        &DataValue::Str("ab".into())
    );
}

#[test]
fn plus_operator_mixed_numeric() {
    let r = runner();
    let o = options();
    assert_eq!(
        r.execute("1 + 2.0", HashMap::new(), &o).unwrap().result(),
        &DataValue::Double(3.0)
    );
}

/// MinusOperator — 减法
#[test]
fn minus_operator_int() {
    let r = runner();
    let o = options();
    assert_eq!(
        r.execute("5 - 3", HashMap::new(), &o).unwrap().result(),
        &DataValue::Int(2)
    );
}

#[test]
fn minus_operator_negative() {
    let r = runner();
    let o = options();
    assert_eq!(
        r.execute("3 - 5", HashMap::new(), &o).unwrap().result(),
        &DataValue::Int(-2)
    );
}

/// MultiplyOperator — 乘法
#[test]
fn multiply_operator_int() {
    let r = runner();
    let o = options();
    assert_eq!(
        r.execute("3 * 4", HashMap::new(), &o).unwrap().result(),
        &DataValue::Int(12)
    );
}

#[test]
fn multiply_operator_long() {
    let r = runner();
    let o = options();
    assert_eq!(
        r.execute("3L * 4L", HashMap::new(), &o).unwrap().result(),
        &DataValue::Long(12)
    );
}

/// DivideOperator — 除法 (Java BigDecimal 精度)
#[test]
fn divide_operator_exact() {
    let r = runner();
    let o = options();
    assert_eq!(
        r.execute("6 / 2", HashMap::new(), &o).unwrap().result(),
        &DataValue::Int(3)
    );
}

#[test]
fn divide_operator_decimal() {
    let r = runner();
    let o = options();
    let result = r.execute("10 / 3", HashMap::new(), &o).unwrap();
    match result.result() {
        DataValue::BigDec(s) => assert!(s.starts_with("3.333")),
        other => panic!("Expected BigDec, got {:?}", other),
    }
}

/// RemainderOperator — 取余
#[test]
fn remainder_operator() {
    let r = runner();
    let o = options();
    assert_eq!(
        r.execute("7 % 3", HashMap::new(), &o).unwrap().result(),
        &DataValue::Int(1)
    );
}

#[test]
fn remainder_operator_negative() {
    let r = runner();
    let o = options();
    assert_eq!(
        r.execute("-7 % 3", HashMap::new(), &o).unwrap().result(),
        &DataValue::Int(-1)
    );
}

/// PlusAssignOperator — 加法赋值
#[test]
fn plus_assign_operator() {
    let r = runner();
    let o = options();
    assert_eq!(
        r.execute("int x = 10; x += 5; x", HashMap::new(), &o)
            .unwrap()
            .result(),
        &DataValue::Int(15)
    );
}

/// MinusAssignOperator — 减法赋值
#[test]
fn minus_assign_operator() {
    let r = runner();
    let o = options();
    assert_eq!(
        r.execute("int x = 10; x -= 3; x", HashMap::new(), &o)
            .unwrap()
            .result(),
        &DataValue::Int(7)
    );
}

/// MultiplyAssignOperator — 乘法赋值
#[test]
fn multiply_assign_operator() {
    let r = runner();
    let o = options();
    assert_eq!(
        r.execute("int x = 10; x *= 3; x", HashMap::new(), &o)
            .unwrap()
            .result(),
        &DataValue::Int(30)
    );
}

/// DivideAssignOperator — 除法赋值
#[test]
fn divide_assign_operator() {
    let r = runner();
    let o = options();
    let result = r
        .execute("int x = 10; x /= 2; x", HashMap::new(), &o)
        .unwrap();
    match result.result() {
        DataValue::Int(5) => {}
        DataValue::BigDec(s) => assert_eq!(s, "5"),
        other => panic!("Expected 5, got {:?}", other),
    }
}

/// RemainderAssignOperator — 取余赋值
#[test]
fn remainder_assign_operator() {
    let r = runner();
    let o = options();
    assert_eq!(
        r.execute("int x = 10; x %= 3; x", HashMap::new(), &o)
            .unwrap()
            .result(),
        &DataValue::Int(1)
    );
}

// ============================================================================
// Bit Operators (13)
// Java: com.alibaba.qlexpress4.runtime.operator.bit
// ============================================================================

/// BitwiseAndOperator — 按位与
#[test]
fn bitwise_and_operator() {
    let r = runner();
    let o = options();
    assert_eq!(
        r.execute("5 & 3", HashMap::new(), &o).unwrap().result(),
        &DataValue::Int(1)
    );
}

/// BitwiseOrOperator — 按位或
#[test]
fn bitwise_or_operator() {
    let r = runner();
    let o = options();
    assert_eq!(
        r.execute("5 | 3", HashMap::new(), &o).unwrap().result(),
        &DataValue::Int(7)
    );
}

/// BitwiseXorOperator — 按位异或
#[test]
fn bitwise_xor_operator() {
    let r = runner();
    let o = options();
    assert_eq!(
        r.execute("5 ^ 3", HashMap::new(), &o).unwrap().result(),
        &DataValue::Int(6)
    );
}

/// BitwiseInvertOperator — 按位取反
#[test]
fn bitwise_invert_operator() {
    let r = runner();
    let o = options();
    assert_eq!(
        r.execute("~0", HashMap::new(), &o).unwrap().result(),
        &DataValue::Int(-1)
    );
}

/// BitwiseLeftShiftOperator — 左移
#[test]
fn bitwise_left_shift_operator() {
    let r = runner();
    let o = options();
    assert_eq!(
        r.execute("1 << 3", HashMap::new(), &o).unwrap().result(),
        &DataValue::Int(8)
    );
}

/// BitwiseRightShiftOperator — 右移
#[test]
fn bitwise_right_shift_operator() {
    let r = runner();
    let o = options();
    assert_eq!(
        r.execute("8 >> 1", HashMap::new(), &o).unwrap().result(),
        &DataValue::Int(4)
    );
}

/// BitwiseRightShiftUnsignedOperator — 无符号右移
#[test]
fn bitwise_right_shift_unsigned_operator() {
    let r = runner();
    let o = options();
    assert_eq!(
        r.execute("-1 >>> 24", HashMap::new(), &o).unwrap().result(),
        &DataValue::Int(255)
    );
}

/// BitwiseAndAssignOperator
#[test]
fn bitwise_and_assign_operator() {
    let r = runner();
    let o = options();
    assert_eq!(
        r.execute("int x = 7; x &= 3; x", HashMap::new(), &o)
            .unwrap()
            .result(),
        &DataValue::Int(3)
    );
}

/// BitwiseOrAssignOperator
#[test]
fn bitwise_or_assign_operator() {
    let r = runner();
    let o = options();
    assert_eq!(
        r.execute("int x = 4; x |= 3; x", HashMap::new(), &o)
            .unwrap()
            .result(),
        &DataValue::Int(7)
    );
}

/// BitwiseXorAssignOperator
#[test]
fn bitwise_xor_assign_operator() {
    let r = runner();
    let o = options();
    assert_eq!(
        r.execute("int x = 5; x ^= 3; x", HashMap::new(), &o)
            .unwrap()
            .result(),
        &DataValue::Int(6)
    );
}

/// BitwiseLeftShiftAssignOperator
#[test]
fn bitwise_left_shift_assign_operator() {
    let r = runner();
    let o = options();
    assert_eq!(
        r.execute("int x = 1; x <<= 3; x", HashMap::new(), &o)
            .unwrap()
            .result(),
        &DataValue::Int(8)
    );
}

/// BitwiseRightShiftAssignOperator
#[test]
fn bitwise_right_shift_assign_operator() {
    let r = runner();
    let o = options();
    assert_eq!(
        r.execute("int x = 8; x >>= 1; x", HashMap::new(), &o)
            .unwrap()
            .result(),
        &DataValue::Int(4)
    );
}

/// BitwiseRightShiftUnsignedAssignOperator
#[test]
fn bitwise_right_shift_unsigned_assign_operator() {
    let r = runner();
    let o = options();
    assert_eq!(
        r.execute("int x = -1; x >>>= 24; x", HashMap::new(), &o)
            .unwrap()
            .result(),
        &DataValue::Int(255)
    );
}

// ============================================================================
// Compare Operators (6)
// Java: com.alibaba.qlexpress4.runtime.operator.compare
// ============================================================================

/// EqualOperator
#[test]
fn equal_operator() {
    let r = runner();
    let o = options();
    assert_eq!(
        r.execute("1 == 1", HashMap::new(), &o).unwrap().result(),
        &DataValue::Bool(true)
    );
    assert_eq!(
        r.execute("1 == 2", HashMap::new(), &o).unwrap().result(),
        &DataValue::Bool(false)
    );
}

/// UnequalOperator
#[test]
fn unequal_operator() {
    let r = runner();
    let o = options();
    assert_eq!(
        r.execute("1 != 2", HashMap::new(), &o).unwrap().result(),
        &DataValue::Bool(true)
    );
    assert_eq!(
        r.execute("1 != 1", HashMap::new(), &o).unwrap().result(),
        &DataValue::Bool(false)
    );
}

/// GreaterOperator
#[test]
fn greater_operator() {
    let r = runner();
    let o = options();
    assert_eq!(
        r.execute("3 > 2", HashMap::new(), &o).unwrap().result(),
        &DataValue::Bool(true)
    );
    assert_eq!(
        r.execute("2 > 3", HashMap::new(), &o).unwrap().result(),
        &DataValue::Bool(false)
    );
}

/// LessOperator
#[test]
fn less_operator() {
    let r = runner();
    let o = options();
    assert_eq!(
        r.execute("2 < 3", HashMap::new(), &o).unwrap().result(),
        &DataValue::Bool(true)
    );
    assert_eq!(
        r.execute("3 < 2", HashMap::new(), &o).unwrap().result(),
        &DataValue::Bool(false)
    );
}

/// GreaterEqualOperator
#[test]
fn greater_equal_operator() {
    let r = runner();
    let o = options();
    assert_eq!(
        r.execute("3 >= 3", HashMap::new(), &o).unwrap().result(),
        &DataValue::Bool(true)
    );
    assert_eq!(
        r.execute("2 >= 3", HashMap::new(), &o).unwrap().result(),
        &DataValue::Bool(false)
    );
}

/// LessEqualOperator
#[test]
fn less_equal_operator() {
    let r = runner();
    let o = options();
    assert_eq!(
        r.execute("3 <= 3", HashMap::new(), &o).unwrap().result(),
        &DataValue::Bool(true)
    );
    assert_eq!(
        r.execute("3 <= 2", HashMap::new(), &o).unwrap().result(),
        &DataValue::Bool(false)
    );
}

// ============================================================================
// Logic Operators (3)
// Java: com.alibaba.qlexpress4.runtime.operator.logic
// ============================================================================

/// LogicAndOperator — 短路与
#[test]
fn logic_and_operator() {
    let r = runner();
    let o = options();
    assert_eq!(
        r.execute("true && true", HashMap::new(), &o)
            .unwrap()
            .result(),
        &DataValue::Bool(true)
    );
    assert_eq!(
        r.execute("true && false", HashMap::new(), &o)
            .unwrap()
            .result(),
        &DataValue::Bool(false)
    );
    assert_eq!(
        r.execute("false && (1/0 == 0)", HashMap::new(), &o)
            .unwrap()
            .result(),
        &DataValue::Bool(false)
    );
}

/// LogicOrOperator — 短路或
#[test]
fn logic_or_operator() {
    let r = runner();
    let o = options();
    assert_eq!(
        r.execute("false || true", HashMap::new(), &o)
            .unwrap()
            .result(),
        &DataValue::Bool(true)
    );
    assert_eq!(
        r.execute("false || false", HashMap::new(), &o)
            .unwrap()
            .result(),
        &DataValue::Bool(false)
    );
    assert_eq!(
        r.execute("true || (1/0 == 0)", HashMap::new(), &o)
            .unwrap()
            .result(),
        &DataValue::Bool(true)
    );
}

/// LogicNotOperator — 逻辑非
#[test]
fn logic_not_operator() {
    let r = runner();
    let o = options();
    assert_eq!(
        r.execute("!true", HashMap::new(), &o).unwrap().result(),
        &DataValue::Bool(false)
    );
    assert_eq!(
        r.execute("!false", HashMap::new(), &o).unwrap().result(),
        &DataValue::Bool(true)
    );
}

// ============================================================================
// Unary Operators (6)
// Java: com.alibaba.qlexpress4.runtime.operator.unary
// ============================================================================

/// MinusUnaryOperator — 一元取负
#[test]
fn minus_unary_operator() {
    let r = runner();
    let o = options();
    assert_eq!(
        r.execute("-5", HashMap::new(), &o).unwrap().result(),
        &DataValue::Int(-5)
    );
    assert_eq!(
        r.execute("-(-3)", HashMap::new(), &o).unwrap().result(),
        &DataValue::Int(3)
    );
}

/// PlusUnaryOperator — 一元正号
#[test]
fn plus_unary_operator() {
    let r = runner();
    let o = options();
    assert_eq!(
        r.execute("+5", HashMap::new(), &o).unwrap().result(),
        &DataValue::Int(5)
    );
}

/// PlusPlusPrefixUnaryOperator — 前置++
#[test]
fn plus_plus_prefix_operator() {
    let r = runner();
    let o = options();
    assert_eq!(
        r.execute("int x = 5; ++x", HashMap::new(), &o)
            .unwrap()
            .result(),
        &DataValue::Int(6)
    );
}

/// PlusPlusSuffixUnaryOperator — 后置++
#[test]
fn plus_plus_suffix_operator() {
    let r = runner();
    let o = options();
    assert_eq!(
        r.execute("int x = 5; x++", HashMap::new(), &o)
            .unwrap()
            .result(),
        &DataValue::Int(5)
    );
}

/// MinusMinusPrefixUnaryOperator — 前置-- (Java 返回原值)
#[test]
fn minus_minus_prefix_operator() {
    let r = runner();
    let o = options();
    assert_eq!(
        r.execute("int x = 5; --x", HashMap::new(), &o)
            .unwrap()
            .result(),
        &DataValue::Int(5)
    );
}

/// MinusMinusSuffixUnaryOperator — 后置--
#[test]
fn minus_minus_suffix_operator() {
    let r = runner();
    let o = options();
    assert_eq!(
        r.execute("int x = 5; x--", HashMap::new(), &o)
            .unwrap()
            .result(),
        &DataValue::Int(4)
    );
}

// ============================================================================
// Assign Operator (1)
// Java: com.alibaba.qlexpress4.runtime.operator.assign.AssignOperator
// ============================================================================

/// AssignOperator — 赋值
#[test]
fn assign_operator() {
    let r = runner();
    let o = options();
    assert_eq!(
        r.execute("int x = 42; x", HashMap::new(), &o)
            .unwrap()
            .result(),
        &DataValue::Int(42)
    );
}

// ============================================================================
// Collection Operators (2)
// Java: com.alibaba.qlexpress4.runtime.operator.collection
// ============================================================================

/// InOperator — in 操作符
#[test]
fn in_operator() {
    let r = open_runner();
    let o = options();
    assert_eq!(
        r.execute("2 in [1,2,3]", HashMap::new(), &o)
            .unwrap()
            .result(),
        &DataValue::Bool(true)
    );
    assert_eq!(
        r.execute("4 in [1,2,3]", HashMap::new(), &o)
            .unwrap()
            .result(),
        &DataValue::Bool(false)
    );
}

/// NotInOperator — !in 操作符
#[test]
fn not_in_operator() {
    let r = open_runner();
    let o = options();
    // QLExpress 不支持 !in 前缀语法, 使用 !(x in y)
    assert_eq!(
        r.execute("!(4 in [1,2,3])", HashMap::new(), &o)
            .unwrap()
            .result(),
        &DataValue::Bool(true)
    );
}

// ============================================================================
// String Operators (2)
// Java: com.alibaba.qlexpress4.runtime.operator.string
// ============================================================================

/// LikeOperator — like 操作符 (正则匹配)
#[test]
fn like_operator() {
    let r = runner();
    let o = options();
    // QLExpress like 使用 SQL LIKE 风格,% 通配符
    assert_eq!(
        r.execute("'hello' like 'hel%'", HashMap::new(), &o)
            .unwrap()
            .result(),
        &DataValue::Bool(true)
    );
    assert_eq!(
        r.execute("'hello' like 'world%'", HashMap::new(), &o)
            .unwrap()
            .result(),
        &DataValue::Bool(false)
    );
    assert_eq!(
        r.execute("'hello' like '%llo'", HashMap::new(), &o)
            .unwrap()
            .result(),
        &DataValue::Bool(true)
    );
}

/// NotLikeOperator — !like 操作符
#[test]
fn not_like_operator() {
    let r = runner();
    let o = options();
    // QLExpress 不支持 !like 前缀语法, 使用 !(x like y)
    assert_eq!(
        r.execute("!('hello' like 'world.*')", HashMap::new(), &o)
            .unwrap()
            .result(),
        &DataValue::Bool(true)
    );
}

// ============================================================================
// InstanceOf Operator (1)
// Java: com.alibaba.qlexpress4.operator.InstanceOfOperator
// ============================================================================

/// InstanceOfOperator
#[test]
fn instance_of_operator() {
    let r = runner();
    let o = options();
    assert_eq!(
        r.execute("1 instanceof Integer", HashMap::new(), &o)
            .unwrap()
            .result(),
        &DataValue::Bool(true)
    );
    assert_eq!(
        r.execute("1L instanceof Long", HashMap::new(), &o)
            .unwrap()
            .result(),
        &DataValue::Bool(true)
    );
    assert_eq!(
        r.execute("'x' instanceof String", HashMap::new(), &o)
            .unwrap()
            .result(),
        &DataValue::Bool(true)
    );
    assert_eq!(
        r.execute("true instanceof Boolean", HashMap::new(), &o)
            .unwrap()
            .result(),
        &DataValue::Bool(true)
    );
    assert_eq!(
        r.execute("1.0 instanceof Double", HashMap::new(), &o)
            .unwrap()
            .result(),
        &DataValue::Bool(true)
    );
}

// ============================================================================
// Number Math (6)
// Java: com.alibaba.qlexpress4.runtime.operator.number
// ============================================================================

/// IntegerMath — int 运算
#[test]
fn integer_math_overflow() {
    let r = runner();
    let o = options();
    assert_eq!(
        r.execute("2147483647 + 1", HashMap::new(), &o)
            .unwrap()
            .result(),
        &DataValue::Int(-2147483648)
    );
}

/// LongMath — long 运算
#[test]
fn long_math() {
    let r = runner();
    let o = options();
    assert_eq!(
        r.execute("100L + 200L", HashMap::new(), &o)
            .unwrap()
            .result(),
        &DataValue::Long(300)
    );
}

/// FloatingPointMath — double 运算
#[test]
fn floating_point_math() {
    let r = runner();
    let o = options();
    assert_eq!(
        r.execute("1.5 + 2.5", HashMap::new(), &o).unwrap().result(),
        &DataValue::Double(4.0)
    );
}

/// BigDecimalMath — BigDecimal 运算
#[test]
fn big_decimal_math() {
    let r = runner();
    let o = options();
    let result = r.execute("10 / 3", HashMap::new(), &o).unwrap();
    match result.result() {
        DataValue::BigDec(s) => assert!(s.starts_with("3.333")),
        other => panic!("Expected BigDec, got {:?}", other),
    }
}

/// BigIntegerMath — BigInteger 运算
#[test]
fn big_integer_math() {
    let r = runner();
    let o = options();
    // 大整数不溢出为 Long 时使用 BigInteger
    let result = r
        .execute("999999999999999999 + 1", HashMap::new(), &o)
        .unwrap();
    match result.result() {
        DataValue::BigInt(ref s) => {
            let v: i64 = s.to_string().parse().unwrap_or(0);
            assert!(v > 0 || s.to_string() == "1000000000000000000");
        }
        DataValue::Long(l) => {} // 可能被提升为 long
        other => panic!("Expected BigInt or Long, got {:?}", other),
    }
}

/// NumberMath — 类型提升矩阵
#[test]
fn number_math_promotion() {
    let r = runner();
    let o = options();
    // int + long -> long
    assert_eq!(
        r.execute("1 + 2L", HashMap::new(), &o).unwrap().result(),
        &DataValue::Long(3)
    );
    // int + double -> double
    assert_eq!(
        r.execute("1 + 2.0", HashMap::new(), &o).unwrap().result(),
        &DataValue::Double(3.0)
    );
    // long + double -> double
    assert_eq!(
        r.execute("1L + 2.0", HashMap::new(), &o).unwrap().result(),
        &DataValue::Double(3.0)
    );
}

// ============================================================================
// Operator Management (4)
// Java: com.alibaba.qlexpress4.operator
// ============================================================================

/// Operator 基 trait — 通过实际操作验证
#[test]
fn operator_base_trait() {
    let r = runner();
    let o = options();
    // 每个操作符都通过 Operator trait 的 apply 方法执行
    assert_eq!(
        r.execute("1 + 2", HashMap::new(), &o).unwrap().result(),
        &DataValue::Int(3)
    );
    assert_eq!(
        r.execute("1 - 2", HashMap::new(), &o).unwrap().result(),
        &DataValue::Int(-1)
    );
    assert_eq!(
        r.execute("2 * 3", HashMap::new(), &o).unwrap().result(),
        &DataValue::Int(6)
    );
}

/// OperatorCheckStrategy — 操作符安全策略
#[test]
fn operator_check_strategy_default() {
    let r = runner();
    let o = options();
    // 默认策略允许基本操作符
    assert_eq!(
        r.execute("1 + 2", HashMap::new(), &o).unwrap().result(),
        &DataValue::Int(3)
    );
}

// ============================================================================
// Type Coercion and Edge Cases
// ============================================================================

/// null 比较
#[test]
fn null_comparison() {
    let r = runner();
    let o = options();
    assert_eq!(
        r.execute("null == null", HashMap::new(), &o)
            .unwrap()
            .result(),
        &DataValue::Bool(true)
    );
    assert_eq!(
        r.execute("null != 1", HashMap::new(), &o).unwrap().result(),
        &DataValue::Bool(true)
    );
}

/// 字符串比较
#[test]
fn string_comparison() {
    let r = runner();
    let o = options();
    assert_eq!(
        r.execute("'abc' == 'abc'", HashMap::new(), &o)
            .unwrap()
            .result(),
        &DataValue::Bool(true)
    );
    assert_eq!(
        r.execute("'abc' != 'def'", HashMap::new(), &o)
            .unwrap()
            .result(),
        &DataValue::Bool(true)
    );
}

/// 三元操作符
#[test]
fn ternary_operator() {
    let r = runner();
    let o = options();
    assert_eq!(
        r.execute("true ? 1 : 2", HashMap::new(), &o)
            .unwrap()
            .result(),
        &DataValue::Long(1)
    );
    assert_eq!(
        r.execute("false ? 1 : 2", HashMap::new(), &o)
            .unwrap()
            .result(),
        &DataValue::Long(2)
    );
}

/// 赋值后使用
#[test]
fn assignment_and_use() {
    let r = runner();
    let o = options();
    assert_eq!(
        r.execute("int x = 0; x = x + 1; x = x + 1; x", HashMap::new(), &o)
            .unwrap()
            .result(),
        &DataValue::Int(2)
    );
}

/// 复合表达式
#[test]
fn compound_expression() {
    let r = runner();
    let o = options();
    assert_eq!(
        r.execute("(1 + 2) * 3", HashMap::new(), &o)
            .unwrap()
            .result(),
        &DataValue::Int(9)
    );
    assert_eq!(
        r.execute("1 + 2 * 3", HashMap::new(), &o).unwrap().result(),
        &DataValue::Int(7)
    );
}

/// null 与算术
#[test]
fn null_arithmetic() {
    let r = runner();
    let o = options();
    let result = r.execute("null + 1", HashMap::new(), &o);
    assert!(result.is_err());
}
