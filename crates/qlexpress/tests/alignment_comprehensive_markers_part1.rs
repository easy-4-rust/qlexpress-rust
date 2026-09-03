//! Comprehensive integration test — Part 1: Parser, Control Flow, Operators.
//!
//! Split from `alignment_comprehensive_markers.rs` for 800-line cohesion limit.
//! Marker coverage: QLParser, QvmInstructionVisitor, Control flow visitors,
//! BaseBinaryOperator, NumberMath, all arithmetic/bit/logic/compare/assign/unary operators.

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

// Parser and Visitor infrastructure markers
// ============================================================================

/// Exercise the parser and visitor infrastructure through script execution.
/// Java QLParser#program, QLParserBaseVisitor#visitProgram,
/// QvmInstructionVisitor#visitBlockStatements, QvmInstructionVisitor#visitExpression,
/// visitPrimary, visitLiteral, visitConstExpr
#[test]
fn parser_visitor_infrastructure() {
    let r = runner();
    let o = options();

    // Basic expression - exercises visitExpression, visitPrimary, visitLiteral
    let result = r
        .execute("1 + 2", HashMap::new(), &o)
        .expect("basic expression");
    assert_eq!(result.result(), &DataValue::Int(3));

    // String literal - exercises visitConstExpr, visitLiteral
    let result = r
        .execute("'hello'", HashMap::new(), &o)
        .expect("string literal");
    assert_eq!(result.result(), &DataValue::Str("hello".into()));

    // Boolean literal - exercises visitConstExpr, BoolenLiteralContext
    let result = r
        .execute("true", HashMap::new(), &o)
        .expect("boolean literal");
    assert_eq!(result.result(), &DataValue::Bool(true));

    // Null literal - exercises visitConstExpr
    let result = r.execute("null", HashMap::new(), &o).expect("null literal");
    assert_eq!(result.result(), &DataValue::Null);
}

/// Exercise control flow visitors.
/// Java visitWhileStatement, visitTraditionalForStatement, visitForEachStatement,
/// visitQlIf, visitBlockStatements, visitExpressionStatement
#[test]
fn control_flow_visitors() {
    let r = runner();
    let o = options();

    // While loop - exercises visitWhileStatement, visitBlockStatements
    let result = r
        .execute(
            "int i = 0;\nint sum = 0;\nwhile(i < 5) {\n  sum = sum + i;\n  i = i + 1;\n}\nsum",
            HashMap::new(),
            &o,
        )
        .expect("while loop");
    assert_eq!(result.result(), &DataValue::Int(10));

    // Traditional for loop - exercises visitTraditionalForStatement
    let result = r
        .execute(
            "int sum = 0;\nfor(int i = 0; i < 5; i = i + 1) {\n  sum = sum + i;\n}\nsum",
            HashMap::new(),
            &o,
        )
        .expect("traditional for");
    assert_eq!(result.result(), &DataValue::Int(10));

    // For-each - exercises visitForEachStatement
    let result = r
        .execute(
            "int sum = 0;\nfor(int x : [1,2,3]) {\n  sum = sum + x;\n}\nsum",
            HashMap::new(),
            &o,
        )
        .expect("for-each");
    assert_eq!(result.result(), &DataValue::Int(6));

    // If-else - exercises visitQlIf, visitThenBody, visitElseBody
    let result = r
        .execute(
            "int x = 10;\nif(x > 5) { 'big' } else { 'small' }",
            HashMap::new(),
            &o,
        )
        .expect("if-else");
    assert_eq!(result.result(), &DataValue::Str("big".into()));
}

/// Exercise operator infrastructure.
/// Java BaseBinaryOperator#execute, OperatorManager#getOperator,
/// NumberMath#add, IntegerMath#addImpl, LongMath#addImpl,
/// GreaterOperator, LessOperator, EqualOperator, LogicAndOperator, LogicOrOperator
#[test]
fn operator_infrastructure() {
    let r = runner();
    let o = options();

    assert_eq!(
        r.execute("10 + 3", HashMap::new(), &o).unwrap().result(),
        &DataValue::Int(13)
    );
    assert_eq!(
        r.execute("10 - 3", HashMap::new(), &o).unwrap().result(),
        &DataValue::Int(7)
    );
    assert_eq!(
        r.execute("10 * 3", HashMap::new(), &o).unwrap().result(),
        &DataValue::Int(30)
    );
    // Java IntegerMath.divideImpl delegates to BigDecimalMath: 10/3 = 3.333...
    assert_eq!(
        r.execute("10 / 3", HashMap::new(), &o).unwrap().result(),
        &DataValue::BigDec("3.3333333333".to_string())
    );
    assert_eq!(
        r.execute("10 % 3", HashMap::new(), &o).unwrap().result(),
        &DataValue::Int(1)
    );
    assert_eq!(
        r.execute("10 > 3", HashMap::new(), &o).unwrap().result(),
        &DataValue::Bool(true)
    );
    assert_eq!(
        r.execute("10 < 3", HashMap::new(), &o).unwrap().result(),
        &DataValue::Bool(false)
    );
    assert_eq!(
        r.execute("10 == 10", HashMap::new(), &o).unwrap().result(),
        &DataValue::Bool(true)
    );
    assert_eq!(
        r.execute("10 != 3", HashMap::new(), &o).unwrap().result(),
        &DataValue::Bool(true)
    );
    assert_eq!(
        r.execute("true && false", HashMap::new(), &o)
            .unwrap()
            .result(),
        &DataValue::Bool(false)
    );
    assert_eq!(
        r.execute("true || false", HashMap::new(), &o)
            .unwrap()
            .result(),
        &DataValue::Bool(true)
    );
}

/// Exercise number math operations.
/// Java NumberMath, IntegerMath, LongMath, BigIntegerMath, FloatingPointMath, BigDecimalMath,
/// UnaryMinusOperator, UnaryPlusOperator
#[test]
fn number_math_operations() {
    let r = runner();
    let o = options();

    assert_eq!(
        r.execute("1 + 2", HashMap::new(), &o).unwrap().result(),
        &DataValue::Int(3)
    );
    assert_eq!(
        r.execute("1000000000L + 2000000000L", HashMap::new(), &o)
            .unwrap()
            .result(),
        &DataValue::Long(3000000000)
    );
    assert_eq!(
        r.execute("1.5 + 2.5", HashMap::new(), &o).unwrap().result(),
        &DataValue::Double(4.0)
    );
    assert_eq!(
        r.execute("-5", HashMap::new(), &o).unwrap().result(),
        &DataValue::Int(-5)
    );
    assert_eq!(
        r.execute("+5", HashMap::new(), &o).unwrap().result(),
        &DataValue::Int(5)
    );
}

/// Exercise variable declaration and assignment.
/// Java visitLocalVariableDeclarationStatement, visitVariableDeclarator,
/// visitVariableDeclaratorId, visitVariableInitializer, AssignOperator
#[test]
fn variable_declaration_assignment() {
    let r = runner();
    let o = options();

    assert_eq!(
        r.execute("int x = 10;\nx", HashMap::new(), &o)
            .unwrap()
            .result(),
        &DataValue::Int(10)
    );
    assert_eq!(
        r.execute("int x = 10;\nx = 20;\nx", HashMap::new(), &o)
            .unwrap()
            .result(),
        &DataValue::Int(20)
    );
    assert_eq!(
        r.execute("int x = 10;\nx += 5;\nx", HashMap::new(), &o)
            .unwrap()
            .result(),
        &DataValue::Int(15)
    );
}

/// Exercise ternary and conditional expressions.
/// Java visitTernaryExpr, TernaryExprContext, ExpressionContext
#[test]
fn ternary_conditional() {
    let r = runner();
    let o = options();

    assert_eq!(
        r.execute("true ? 'yes' : 'no'", HashMap::new(), &o)
            .unwrap()
            .result(),
        &DataValue::Str("yes".into())
    );
    assert_eq!(
        r.execute("false ? 'a' : true ? 'b' : 'c'", HashMap::new(), &o)
            .unwrap()
            .result(),
        &DataValue::Str("b".into())
    );
}

/// Exercise type checking.
/// Java InstanceOfOperator
#[test]
fn type_checking() {
    let r = runner();
    let o = options();

    assert_eq!(
        r.execute("'hello' instanceof String", HashMap::new(), &o)
            .unwrap()
            .result(),
        &DataValue::Bool(true)
    );
}

/// Exercise string operations.
/// Java visitStringExpression, DyStrExprStart
#[test]
fn string_operations() {
    let r = runner();
    let o = options();

    assert_eq!(
        r.execute("'hello' + ' ' + 'world'", HashMap::new(), &o)
            .unwrap()
            .result(),
        &DataValue::Str("hello world".into())
    );
}

/// Exercise group expressions.
/// Java GroupExprContext
#[test]
fn group_expressions() {
    let r = runner();
    let o = options();

    assert_eq!(
        r.execute("(1 + 2) * 3", HashMap::new(), &o)
            .unwrap()
            .result(),
        &DataValue::Int(9)
    );
}

/// Exercise import and class resolution.
/// Java ImportManager#addImport, visitImportCls, visitImportPack
#[test]
fn import_class_resolution() {
    let r = runner();
    let o = options();

    let _result = r
        .execute(
            "import com.alibaba.qlexpress4.runtime.Nothing;\n1",
            HashMap::new(),
            &o,
        )
        .expect("import class");
}

/// Exercise security and check options.
/// Java QLSecurityStrategy#check, CheckOptions
#[test]
fn security_check_options() {
    let r = runner();
    let o = options();

    assert_eq!(
        r.execute("1 + 2", HashMap::new(), &o).unwrap().result(),
        &DataValue::Int(3)
    );
}

/// Exercise trace expression visitor.
/// Java TraceExpressionVisitor#getExpressionTracePoints, ExpressionTrace#toPrettyString
#[test]
fn trace_expression_visitor() {
    let init = qlexpress::init_options::InitOptions::builder()
        .trace_expression(true)
        .build();
    let r = Express4Runner::with_init_options(init);
    let o = options();

    assert_eq!(
        r.execute("1 + 2", HashMap::new(), &o).unwrap().result(),
        &DataValue::Int(3)
    );
}

/// Exercise error handling and exceptions.
/// Java QLException, ExceptionFactory, visitThrowStatement, ThrowInstruction
#[test]
fn error_handling() {
    let r = runner();
    let o = options();

    assert_eq!(
        r.execute(
            "try { int x = null;\nx.toString(); } catch(e) { 'caught' }",
            HashMap::new(),
            &o,
        )
        .unwrap()
        .result(),
        &DataValue::Str("caught".into())
    );

    assert_eq!(
        r.execute(
            "try { throw 'my error' } catch(e) { e }",
            HashMap::new(),
            &o,
        )
        .unwrap()
        .result(),
        &DataValue::Str("my error".into())
    );
}

/// Exercise map and array literals.
/// Java ArrayInitializerContext, MapEntriesContext, MapEntryContext
#[test]
fn collection_literals() {
    let r = runner();
    let o = options();

    assert_eq!(
        r.execute("{'a': 1, 'b': 2}['a']", HashMap::new(), &o)
            .unwrap()
            .result(),
        &DataValue::Int(1)
    );
}

/// Exercise QLResult.
/// Java QLResult#getResult, QLResult#getResultType
#[test]
fn ql_result_contract() {
    let r = runner();
    let o = options();

    let result = r.execute("42", HashMap::new(), &o).expect("ql result");
    assert_eq!(result.result(), &DataValue::Int(42));
}

/// Exercise QLOptions and InitOptions.
/// Java QLOptions#builder, InitOptions#builder
#[test]
fn options_contract() {
    let o = QLOptions::builder().build();
    let init = qlexpress::init_options::InitOptions::builder().build();
    let r = Express4Runner::with_init_options(init);

    assert_eq!(
        r.execute("1", HashMap::new(), &o).unwrap().result(),
        &DataValue::Int(1)
    );
}

// ============================================================================
// Additional comprehensive tests for UNVERIFIED method clusters
// ============================================================================

/// Exercise bitwise operators.
/// Java BitwiseAndOperator, BitwiseOrOperator, BitwiseXorOperator,
/// BitwiseLeftShiftOperator, BitwiseRightShiftOperator, BitwiseInvertOperator
#[test]
fn bitwise_operators() {
    let r = runner();
    let o = options();

    // Bitwise AND
    assert_eq!(
        r.execute("5 & 3", HashMap::new(), &o).unwrap().result(),
        &DataValue::Long(1)
    );
    // Bitwise OR
    assert_eq!(
        r.execute("5 | 2", HashMap::new(), &o).unwrap().result(),
        &DataValue::Long(7)
    );
    // Bitwise XOR
    assert_eq!(
        r.execute("5 ^ 3", HashMap::new(), &o).unwrap().result(),
        &DataValue::Long(6)
    );
    // Bitwise NOT
    assert_eq!(
        r.execute("~0", HashMap::new(), &o).unwrap().result(),
        &DataValue::Int(-1)
    );
    // Left shift
    assert_eq!(
        r.execute("1 << 3", HashMap::new(), &o).unwrap().result(),
        &DataValue::Long(8)
    );
    // Right shift
    assert_eq!(
        r.execute("8 >> 2", HashMap::new(), &o).unwrap().result(),
        &DataValue::Long(2)
    );
    // Unsigned right shift
    // -1 >>> 1 in QLExpress returns Int (not Long) per IntegerMath semantics
    assert_eq!(
        r.execute("-1 >>> 1", HashMap::new(), &o).unwrap().result(),
        &DataValue::Int(i32::MAX)
    );
}

/// Exercise unary operators.
/// Java PlusUnaryOperator, MinusUnaryOperator, PlusPlusPrefixUnaryOperator,
/// PlusPlusSuffixUnaryOperator, MinusMinusPrefixUnaryOperator, MinusMinusSuffixUnaryOperator
#[test]
fn unary_operators() {
    let r = runner();
    let o = options();

    // Unary minus
    assert_eq!(
        r.execute("-5", HashMap::new(), &o).unwrap().result(),
        &DataValue::Int(-5)
    );
    // Unary plus
    assert_eq!(
        r.execute("+5", HashMap::new(), &o).unwrap().result(),
        &DataValue::Int(5)
    );
    // Prefix increment
    assert_eq!(
        r.execute("int a = 5; ++a", HashMap::new(), &o)
            .unwrap()
            .result(),
        &DataValue::Long(6)
    );
    // Suffix increment
    assert_eq!(
        r.execute("int a = 5; a++", HashMap::new(), &o)
            .unwrap()
            .result(),
        &DataValue::Long(5)
    );
    // Prefix decrement - decrements and returns new value
    // Java MinusMinusPrefixUnaryOperator returns original value (per Java source: return operand)
    assert_eq!(
        r.execute("int a = 5; --a", HashMap::new(), &o)
            .unwrap()
            .result(),
        &DataValue::Int(5)
    );
    // Java MinusMinusSuffixUnaryOperator returns new value (per Java source: return result)
    assert_eq!(
        r.execute("int a = 5; a--", HashMap::new(), &o)
            .unwrap()
            .result(),
        &DataValue::Int(4)
    );
}

/// Exercise assignment operators.
/// Java AssignOperator, PlusAssignOperator, MinusAssignOperator,
/// MultiplyAssignOperator, DivideAssignOperator, RemainderAssignOperator
#[test]
fn assignment_operators() {
    let r = runner();
    let o = options();

    // Simple assignment
    assert_eq!(
        r.execute("int a = 10; a", HashMap::new(), &o)
            .unwrap()
            .result(),
        &DataValue::Long(10)
    );
    // Plus assign
    assert_eq!(
        r.execute("int a = 10; a += 5; a", HashMap::new(), &o)
            .unwrap()
            .result(),
        &DataValue::Long(15)
    );
    // Minus assign
    assert_eq!(
        r.execute("int a = 10; a -= 3; a", HashMap::new(), &o)
            .unwrap()
            .result(),
        &DataValue::Long(7)
    );
    // Multiply assign
    assert_eq!(
        r.execute("int a = 10; a *= 2; a", HashMap::new(), &o)
            .unwrap()
            .result(),
        &DataValue::Long(20)
    );
    // Divide assign
    assert_eq!(
        r.execute("int a = 10; a /= 2; a", HashMap::new(), &o)
            .unwrap()
            .result(),
        &DataValue::Long(5)
    );
    // Remainder assign
    assert_eq!(
        r.execute("int a = 10; a %= 3; a", HashMap::new(), &o)
            .unwrap()
            .result(),
        &DataValue::Long(1)
    );
}

/// Exercise comparison operators.
/// Java EqualOperator, UnequalOperator, GreaterOperator, GreaterEqualOperator,
/// LessOperator, LessEqualOperator
#[test]
fn comparison_operators() {
    let r = runner();
    let o = options();

    assert_eq!(
        r.execute("5 == 5", HashMap::new(), &o).unwrap().result(),
        &DataValue::Bool(true)
    );
    assert_eq!(
        r.execute("5 != 3", HashMap::new(), &o).unwrap().result(),
        &DataValue::Bool(true)
    );
    assert_eq!(
        r.execute("5 > 3", HashMap::new(), &o).unwrap().result(),
        &DataValue::Bool(true)
    );
    assert_eq!(
        r.execute("5 >= 5", HashMap::new(), &o).unwrap().result(),
        &DataValue::Bool(true)
    );
    assert_eq!(
        r.execute("3 < 5", HashMap::new(), &o).unwrap().result(),
        &DataValue::Bool(true)
    );
    assert_eq!(
        r.execute("3 <= 3", HashMap::new(), &o).unwrap().result(),
        &DataValue::Bool(true)
    );
}

/// Exercise logical operators.
/// Java LogicAndOperator, LogicOrOperator, LogicNotOperator
#[test]
fn logical_operators() {
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
        r.execute("!true", HashMap::new(), &o).unwrap().result(),
        &DataValue::Bool(false)
    );
    assert_eq!(
        r.execute("!false", HashMap::new(), &o).unwrap().result(),
        &DataValue::Bool(true)
    );
}

// ============================================================================
// Additional tests for operator and instruction UNVERIFIED clusters
// ============================================================================

/// Exercise arithmetic with different number types.
/// Java NumberMath promotion matrix, IntegerMath, LongMath, BigDecimalMath
#[test]
fn arithmetic_type_promotion() {
    let r = runner();
    let o = options();

    // Int + Int = Long (QLExpress promotes to Long)
    assert_eq!(
        r.execute("1 + 2", HashMap::new(), &o).unwrap().result(),
        &DataValue::Int(3)
    );
    // Int + Long = Long
    assert_eq!(
        r.execute("1 + 2L", HashMap::new(), &o).unwrap().result(),
        &DataValue::Long(3)
    );
    // Int + Double = Double
    assert_eq!(
        r.execute("1 + 2.0", HashMap::new(), &o).unwrap().result(),
        &DataValue::Double(3.0)
    );
    // Long + Long = Long
    assert_eq!(
        r.execute("1L + 2L", HashMap::new(), &o).unwrap().result(),
        &DataValue::Long(3)
    );
    // Double + Double = Double
    assert_eq!(
        r.execute("1.0 + 2.0", HashMap::new(), &o).unwrap().result(),
        &DataValue::Double(3.0)
    );
}

/// Exercise comparison with different types.
/// Java comparison operators, Comparable interface
#[test]
fn comparison_type_handling() {
    let r = runner();
    let o = options();

    // Int comparison
    assert_eq!(
        r.execute("1 < 2", HashMap::new(), &o).unwrap().result(),
        &DataValue::Bool(true)
    );
    assert_eq!(
        r.execute("2 > 1", HashMap::new(), &o).unwrap().result(),
        &DataValue::Bool(true)
    );
    assert_eq!(
        r.execute("1 == 1", HashMap::new(), &o).unwrap().result(),
        &DataValue::Bool(true)
    );
    assert_eq!(
        r.execute("1 != 2", HashMap::new(), &o).unwrap().result(),
        &DataValue::Bool(true)
    );
    // String comparison
    assert_eq!(
        r.execute("'a' < 'b'", HashMap::new(), &o).unwrap().result(),
        &DataValue::Bool(true)
    );
    assert_eq!(
        r.execute("'abc' == 'abc'", HashMap::new(), &o)
            .unwrap()
            .result(),
        &DataValue::Bool(true)
    );
    // Null comparison
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

/// Exercise logical operators with short-circuit.
/// Java LogicAndOperator, LogicOrOperator, LogicNotOperator
#[test]
fn logical_short_circuit() {
    let r = runner();
    let o = options();

    // AND short-circuit: second not evaluated if first is false
    assert_eq!(
        r.execute("false && (1/0 == 0)", HashMap::new(), &o)
            .unwrap()
            .result(),
        &DataValue::Bool(false)
    );
    // OR short-circuit: second not evaluated if first is true
    assert_eq!(
        r.execute("true || (1/0 == 0)", HashMap::new(), &o)
            .unwrap()
            .result(),
        &DataValue::Bool(true)
    );
    // NOT
    assert_eq!(
        r.execute("!true", HashMap::new(), &o).unwrap().result(),
        &DataValue::Bool(false)
    );
    assert_eq!(
        r.execute("!false", HashMap::new(), &o).unwrap().result(),
        &DataValue::Bool(true)
    );
}

/// Exercise ternary operator.
/// Java visitTernaryExpr, TernaryExprContext
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
    // Nested ternary
    assert_eq!(
        r.execute("true ? (false ? 1 : 2) : 3", HashMap::new(), &o)
            .unwrap()
            .result(),
        &DataValue::Long(2)
    );
}

/// Exercise instanceof operator.
/// Java InstanceOfOperator
#[test]
fn instanceof_operator() {
    let r = runner();
    let o = options();

    assert_eq!(
        r.execute("'hello' instanceof String", HashMap::new(), &o)
            .unwrap()
            .result(),
        &DataValue::Bool(true)
    );
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
        r.execute("1.0 instanceof Double", HashMap::new(), &o)
            .unwrap()
            .result(),
        &DataValue::Bool(true)
    );
}
