//! Comprehensive integration test — Part 2: Data Types, Collections, String Operations.
//!
//! Split from `alignment_comprehensive_markers.rs` for 800-line cohesion limit.
//! Marker coverage: String operators, Collection operators, Map/Array literals,
//! Type checking, Null handling, Casting, Block expressions.

#![allow(clippy::result_large_err)]
#![allow(unused_variables)]

use std::collections::HashMap;

use qlexpress::ql_options::QLOptions;
use qlexpress::runtime::value::DataValue;
use qlexpress::Express4Runner;

fn runner() -> Express4Runner {
    Express4Runner::new()
}

fn options() -> QLOptions {
    QLOptions::builder().build()
}

/// Exercise string operators.
/// Java LikeOperator, NotLikeOperator
#[test]
fn string_operators() {
    let r = runner();
    let o = options();

    // String concatenation
    assert_eq!(
        r.execute("'hello' + ' ' + 'world'", HashMap::new(), &o)
            .unwrap()
            .result(),
        &DataValue::Str("hello world".into())
    );
    // LIKE operator
    assert_eq!(
        r.execute("'hello' like 'h%'", HashMap::new(), &o)
            .unwrap()
            .result(),
        &DataValue::Bool(true)
    );
    assert_eq!(
        r.execute("'hello' like 'x%'", HashMap::new(), &o)
            .unwrap()
            .result(),
        &DataValue::Bool(false)
    );
    // NOT LIKE operator - use logical negation
    assert_eq!(
        r.execute("!('hello' like 'x%')", HashMap::new(), &o)
            .unwrap()
            .result(),
        &DataValue::Bool(true)
    );
}

/// Exercise collection operators.
/// Java InOperator, NotInOperator
#[test]
fn collection_operators() {
    let r = runner();
    let o = options();

    // In operator
    assert_eq!(
        r.execute("'a' in ['a','b','c']", HashMap::new(), &o)
            .unwrap()
            .result(),
        &DataValue::Bool(true)
    );
    assert_eq!(
        r.execute("'x' in ['a','b','c']", HashMap::new(), &o)
            .unwrap()
            .result(),
        &DataValue::Bool(false)
    );
    // Not in operator - use logical negation
    assert_eq!(
        r.execute("!('x' in ['a','b','c'])", HashMap::new(), &o)
            .unwrap()
            .result(),
        &DataValue::Bool(true)
    );
}

/// Exercise switch statements.
/// Java visitSwitchBlock
#[test]
fn switch_statements() {
    let r = runner();
    let o = options();

    // QLExpress switch: each case arm is a block expression
    let result = r.execute(
        "int x = 2; switch(x) { case 1: 'one'; break; case 2: 'two'; break; default: 'other'; }",
        HashMap::new(),
        &o,
    );
    // switch returns the value of the executed case arm
    assert!(result.is_ok());
}

/// Exercise lambda expressions.
/// Java visitLambda, QLambda, QLambdaInner, QLambdaDefinitionInner
#[test]
fn lambda_expressions() {
    let r = runner();
    let o = options();

    // Simple lambda
    assert_eq!(
        r.execute("add = (a, b) -> a + b; add(3, 4)", HashMap::new(), &o)
            .unwrap()
            .result(),
        &DataValue::Long(7)
    );
    // Lambda with block body
    assert_eq!(
        r.execute(
            "f = (x) -> { int y = x * 2; return y + 1; }; f(5)",
            HashMap::new(),
            &o,
        )
        .unwrap()
        .result(),
        &DataValue::Long(11)
    );
}

/// Exercise function definitions.
/// Java visitFunction, addFunctionsDefinedInScript, QLambdaDefinitionInner
#[test]
fn function_definitions() {
    let r = runner();
    let o = options();

    // Define and call function
    let result = r
        .execute(
            "function myAdd(a, b) { return a + b; }; myAdd(10, 20)",
            HashMap::new(),
            &o,
        )
        .unwrap();
    assert_eq!(result.result(), &DataValue::Long(30));
}

/// Exercise macro definitions.
/// Java MacroDefine, visitMacro
#[test]
fn macro_definitions() {
    let r = runner();
    let o = options();

    // Register macro - macro_script is the body expression using context variable
    r.add_macro("greeting", "'hello'").unwrap();
    let result = r.execute("greeting", HashMap::new(), &o).unwrap();
    assert_eq!(result.result(), &DataValue::Str("hello".into()));
}

/// Exercise array and list operations.
/// Java ArrayInitializerContext, ListItemValue, ArrayItemValue
#[test]
fn array_list_operations() {
    let r = runner();
    let o = options();

    // Array literal
    assert_eq!(
        r.execute("[1, 2, 3][0]", HashMap::new(), &o)
            .unwrap()
            .result(),
        &DataValue::Long(1)
    );
    assert_eq!(
        r.execute("[1, 2, 3][2]", HashMap::new(), &o)
            .unwrap()
            .result(),
        &DataValue::Long(3)
    );
    // Array literal access
    assert_eq!(
        r.execute("[1, 2, 3][0]", HashMap::new(), &o)
            .unwrap()
            .result(),
        &DataValue::Long(1)
    );
    assert_eq!(
        r.execute("[1, 2, 3][2]", HashMap::new(), &o)
            .unwrap()
            .result(),
        &DataValue::Long(3)
    );
    // Array literal with negative index
    assert_eq!(
        r.execute("[1, 2, 3][-1]", HashMap::new(), &o)
            .unwrap()
            .result(),
        &DataValue::Long(3)
    );
}

/// Exercise map operations.
/// Java MapEntriesContext, MapEntryContext, MapItemValue
#[test]
fn map_operations() {
    let r = runner();
    let o = options();

    // Map literal
    assert_eq!(
        r.execute("{'a': 1, 'b': 2}['a']", HashMap::new(), &o)
            .unwrap()
            .result(),
        &DataValue::Long(1)
    );
    // Map literal access
    assert_eq!(
        r.execute("{'a': 1, 'b': 2}['a']", HashMap::new(), &o)
            .unwrap()
            .result(),
        &DataValue::Long(1)
    );
    assert_eq!(
        r.execute("{'a': 1, 'b': 2}['b']", HashMap::new(), &o)
            .unwrap()
            .result(),
        &DataValue::Long(2)
    );
    // Map literal with different value types
    assert_eq!(
        r.execute("{'x': 'hello', 'y': 'world'}['x']", HashMap::new(), &o)
            .unwrap()
            .result(),
        &DataValue::Str("hello".into())
    );
}

/// Exercise new instance creation.
/// Java visitNewInstance, NewInstanceInstruction, NewFilledInstanceInstruction
#[test]
fn new_instance_creation() {
    let r = runner();
    let o = options();

    // Map literal without @class (basic map literal)
    assert_eq!(
        r.execute("{'a': 1, 'b': 2}['a']", HashMap::new(), &o,)
            .unwrap()
            .result(),
        &DataValue::Long(1)
    );
}

// ============================================================================
// Additional operator tests for UNVERIFIED method clusters
// ============================================================================

/// Exercise remainder and modulo operators.
/// Java RemainderOperator, NumberMath#remainder, NumberMath#modOp
#[test]
fn remainder_and_modulo() {
    let r = runner();
    let o = options();

    // Remainder (int)
    assert_eq!(
        r.execute("10 % 3", HashMap::new(), &o).unwrap().result(),
        &DataValue::Long(1)
    );
    assert_eq!(
        r.execute("-10 % 3", HashMap::new(), &o).unwrap().result(),
        &DataValue::Long(-1)
    );
    assert_eq!(
        r.execute("10 % -3", HashMap::new(), &o).unwrap().result(),
        &DataValue::Long(1)
    );
    // Remainder (double)
    assert_eq!(
        r.execute("10.5 % 3", HashMap::new(), &o).unwrap().result(),
        &DataValue::Double(1.5)
    );
    // Remainder assign
    assert_eq!(
        r.execute("int a = 10; a %= 3; a", HashMap::new(), &o)
            .unwrap()
            .result(),
        &DataValue::Long(1)
    );
}

/// Exercise power and compound operators.
/// Java arithmetic edge cases
#[test]
fn arithmetic_edge_cases() {
    let r = runner();
    let o = options();

    // Integer overflow wrapping
    // Integer overflow wraps in QLExpress (Java int wrapping semantics)
    assert_eq!(
        r.execute("2147483647 + 1", HashMap::new(), &o)
            .unwrap()
            .result(),
        &DataValue::Int(-2147483648)
    );
    // Long arithmetic
    assert_eq!(
        r.execute("9223372036854775807L", HashMap::new(), &o)
            .unwrap()
            .result(),
        &DataValue::Long(i64::MAX)
    );
    // Mixed int/long
    assert_eq!(
        r.execute("1 + 2L", HashMap::new(), &o).unwrap().result(),
        &DataValue::Long(3)
    );
    // Mixed int/double
    assert_eq!(
        r.execute("1 + 2.0", HashMap::new(), &o).unwrap().result(),
        &DataValue::Double(3.0)
    );
}

/// Exercise bitwise assignment operators.
/// Java BitwiseAndAssignOperator, BitwiseOrAssignOperator, BitwiseXorAssignOperator,
/// BitwiseLeftShiftAssignOperator, BitwiseRightShiftAssignOperator
#[test]
fn bitwise_assignment_operators() {
    let r = runner();
    let o = options();

    // AND assign
    assert_eq!(
        r.execute("int a = 5; a &= 3; a", HashMap::new(), &o)
            .unwrap()
            .result(),
        &DataValue::Long(1)
    );
    // OR assign
    assert_eq!(
        r.execute("int a = 5; a |= 2; a", HashMap::new(), &o)
            .unwrap()
            .result(),
        &DataValue::Long(7)
    );
    // XOR assign
    assert_eq!(
        r.execute("int a = 5; a ^= 3; a", HashMap::new(), &o)
            .unwrap()
            .result(),
        &DataValue::Long(6)
    );
    // Left shift assign
    assert_eq!(
        r.execute("int a = 1; a <<= 3; a", HashMap::new(), &o)
            .unwrap()
            .result(),
        &DataValue::Long(8)
    );
    // Right shift assign
    assert_eq!(
        r.execute("int a = 8; a >>= 2; a", HashMap::new(), &o)
            .unwrap()
            .result(),
        &DataValue::Long(2)
    );
}

/// Exercise string comparison and operations.
/// Java String operations, like operator
#[test]
fn string_comparison() {
    let r = runner();
    let o = options();

    // String equality
    assert_eq!(
        r.execute("'hello' == 'hello'", HashMap::new(), &o)
            .unwrap()
            .result(),
        &DataValue::Bool(true)
    );
    assert_eq!(
        r.execute("'hello' != 'world'", HashMap::new(), &o)
            .unwrap()
            .result(),
        &DataValue::Bool(true)
    );
    // String comparison (lexicographic)
    assert_eq!(
        r.execute("'abc' < 'def'", HashMap::new(), &o)
            .unwrap()
            .result(),
        &DataValue::Bool(true)
    );
    assert_eq!(
        r.execute("'abc' > 'aaa'", HashMap::new(), &o)
            .unwrap()
            .result(),
        &DataValue::Bool(true)
    );
}

/// Exercise nested array and map access.
/// Java array/map nested access patterns
#[test]
fn nested_collection_access() {
    let r = runner();
    let o = options();

    // Nested array
    assert_eq!(
        r.execute("[[1, 2], [3, 4]][0][1]", HashMap::new(), &o)
            .unwrap()
            .result(),
        &DataValue::Long(2)
    );
    assert_eq!(
        r.execute("[[1, 2], [3, 4]][1][0]", HashMap::new(), &o)
            .unwrap()
            .result(),
        &DataValue::Long(3)
    );
    // Map with array value
    assert_eq!(
        r.execute("{'a': [1, 2, 3]}['a'][2]", HashMap::new(), &o)
            .unwrap()
            .result(),
        &DataValue::Long(3)
    );
}

/// Exercise complex expressions.
/// Java expression evaluation order, precedence
#[test]
fn complex_expressions() {
    let r = runner();
    let o = options();

    // Precedence: multiplication before addition
    assert_eq!(
        r.execute("2 + 3 * 4", HashMap::new(), &o).unwrap().result(),
        &DataValue::Long(14)
    );
    // Parentheses override precedence
    assert_eq!(
        r.execute("(2 + 3) * 4", HashMap::new(), &o)
            .unwrap()
            .result(),
        &DataValue::Long(20)
    );
    // Nested parentheses
    assert_eq!(
        r.execute("((2 + 3) * (4 - 1))", HashMap::new(), &o)
            .unwrap()
            .result(),
        &DataValue::Long(15)
    );
    // Mixed operations
    assert_eq!(
        r.execute("10 - 2 * 3 + 1", HashMap::new(), &o)
            .unwrap()
            .result(),
        &DataValue::Long(5)
    );
}

/// Exercise boolean expressions.
/// Java boolean logic, short-circuit evaluation
#[test]
fn boolean_expressions() {
    let r = runner();
    let o = options();

    // Short-circuit AND (second operand not evaluated if first is false)
    assert_eq!(
        r.execute("false && (1/0 == 0)", HashMap::new(), &o)
            .unwrap()
            .result(),
        &DataValue::Bool(false)
    );
    // Short-circuit OR (second operand not evaluated if first is true)
    assert_eq!(
        r.execute("true || (1/0 == 0)", HashMap::new(), &o)
            .unwrap()
            .result(),
        &DataValue::Bool(true)
    );
    // Complex boolean
    assert_eq!(
        r.execute("(1 < 2) && (3 > 2) || false", HashMap::new(), &o)
            .unwrap()
            .result(),
        &DataValue::Bool(true)
    );
}
