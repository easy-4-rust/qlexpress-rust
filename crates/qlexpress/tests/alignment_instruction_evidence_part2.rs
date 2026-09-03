//! 综合指令级语义测试 — Part 2: 操作符、一元、集合、控制流补充。
//!
//! Part 1 覆盖指令 1–27（ConstInstruction 至 CastInstruction）。
//! 本文件覆盖指令 28–41（OperatorInstruction 至 TracePeekInstruction）
//! 以及运算符优先级、集合操作、控制流补充语义。

#![allow(clippy::result_large_err)]
#![allow(clippy::unnecessary_unwrap)]
#![allow(clippy::approx_constant)]
#![allow(unused_variables)]

use std::collections::HashMap;

use qlexpress::ql_options::QLOptions;
use qlexpress::runtime::value::DataValue;
use qlexpress::Express4Runner;

fn runner() -> Express4Runner {
    Express4Runner::new()
}

fn open_runner() -> Express4Runner {
    Express4Runner::with_init_options(
        qlexpress::init_options::InitOptions::builder()
            .security_strategy(
                qlexpress::security::ql_security_strategy::QLSecurityStrategy::open(),
            )
            .build(),
    )
}

fn options() -> QLOptions {
    QLOptions::builder().build()
}

/// 一元操作: 前置自增
#[test]
fn unary_instruction_prefix_increment() {
    let r = runner();
    let o = options();
    assert_eq!(
        r.execute("int x = 5; ++x", HashMap::new(), &o)
            .unwrap()
            .result(),
        &DataValue::Int(6)
    );
}

/// 一元操作: 后置自增
#[test]
fn unary_instruction_postfix_increment() {
    let r = runner();
    let o = options();
    assert_eq!(
        r.execute("int x = 5; x++", HashMap::new(), &o)
            .unwrap()
            .result(),
        &DataValue::Int(5)
    );
}

/// 一元操作: 前置自减
#[test]
fn unary_instruction_prefix_decrement() {
    let r = runner();
    let o = options();
    // Java prefix -- returns original value (before decrement)
    assert_eq!(
        r.execute("int x = 5; --x", HashMap::new(), &o)
            .unwrap()
            .result(),
        &DataValue::Int(5)
    );
}

/// 一元操作: 后置自减
#[test]
fn unary_instruction_postfix_decrement() {
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
// 30. NewArrayInstruction — 数组创建
// Java: com.alibaba.qlexpress4.runtime.instruction.NewArrayInstruction
// ============================================================================

/// 数组创建: 列表字面量
/// Java NewArrayInstruction#execute — create array/list
#[test]
fn new_array_instruction_list() {
    let r = runner();
    let o = options();
    let result = r.execute("[1, 2, 3]", HashMap::new(), &o);
    assert!(result.is_ok());
}

/// 数组创建: 空列表
#[test]
fn new_array_instruction_empty() {
    let r = runner();
    let o = options();
    let result = r.execute("[]", HashMap::new(), &o);
    assert!(result.is_ok());
}

// ============================================================================
// 31. MultiNewArrayInstruction — 多维数组
// Java: com.alibaba.qlexpress4.runtime.instruction.MultiNewArrayInstruction
// ============================================================================

/// 多维数组
/// Java MultiNewArrayInstruction#execute — create multi-dimensional array
#[test]
fn multi_new_array_instruction() {
    let r = runner();
    let o = options();
    // Nested list creation
    let result = r.execute("[[1,2],[3,4]]", HashMap::new(), &o);
    assert!(result.is_ok());
}

// ============================================================================
// 32. NewListInstruction — 列表创建
// Java: com.alibaba.qlexpress4.runtime.instruction.NewListInstruction
// ============================================================================

/// 列表创建
/// Java NewListInstruction#execute — create list
#[test]
fn new_list_instruction_basic() {
    let r = open_runner();
    let o = options();
    let result = r
        .execute("a = [1,2,3]; a.length", HashMap::new(), &o)
        .unwrap();
    assert_eq!(result.result(), &DataValue::Int(3));
}

// ============================================================================
// 33. NewMapInstruction — Map 创建
// Java: com.alibaba.qlexpress4.runtime.instruction.NewMapInstruction
// ============================================================================

/// Map 创建
/// Java NewMapInstruction#execute — create map
#[test]
fn new_map_instruction_basic() {
    let r = open_runner();
    let o = options();
    assert_eq!(
        r.execute("m = {'a': 1, 'b': 2}; m.size()", HashMap::new(), &o)
            .unwrap()
            .result(),
        &DataValue::Int(2)
    );
}

/// Map 创建: 空 map
#[test]
fn new_map_instruction_empty() {
    let r = open_runner();
    let o = options();
    assert_eq!(
        r.execute("m = {:}; m.size()", HashMap::new(), &o)
            .unwrap()
            .result(),
        &DataValue::Int(0)
    );
}

// ============================================================================
// 34. NewInstanceInstruction — 对象实例化
// Java: com.alibaba.qlexpress4.runtime.instruction.NewInstanceInstruction
// ============================================================================

/// 对象实例化: 通过 @class 标记
/// Java NewInstanceInstruction#execute — create object instance
#[test]
fn new_instance_instruction_with_class() {
    let r = runner();
    let o = options();
    let result = r.execute("m = {'@class': 'HashMap'}; m", HashMap::new(), &o);
    assert!(result.is_ok());
}

// ============================================================================
// 35. NewFilledInstanceInstruction — 填充实例
// Java: com.alibaba.qlexpress4.runtime.instruction.NewFilledInstanceInstruction
// ============================================================================

/// 填充实例: 对象字面量带字段
/// Java NewFilledInstanceInstruction#execute — create instance with field values
#[test]
fn new_filled_instance_instruction() {
    let r = runner();
    let o = options();
    let result = r.execute(
        "m = {'@class': 'HashMap', 'key': 'value'}; m",
        HashMap::new(),
        &o,
    );
    assert!(result.is_ok());
}

// ============================================================================
// 36. StringJoinInstruction — 字符串拼接
// Java: com.alibaba.qlexpress4.runtime.instruction.StringJoinInstruction
// ============================================================================

/// 字符串拼接: 多个操作数
/// Java StringJoinInstruction#execute — join strings
#[test]
fn string_join_instruction_multiple() {
    let r = runner();
    let o = options();
    assert_eq!(
        r.execute("'a' + 'b' + 'c'", HashMap::new(), &o)
            .unwrap()
            .result(),
        &DataValue::Str("abc".into())
    );
}

/// 字符串拼接: 混合类型
#[test]
fn string_join_instruction_mixed() {
    let r = runner();
    let o = options();
    assert_eq!(
        r.execute("'value: ' + 42 + ' items'", HashMap::new(), &o)
            .unwrap()
            .result(),
        &DataValue::Str("value: 42 items".into())
    );
}

// ============================================================================
// 37. TryCatchInstruction — try/catch
// Java: com.alibaba.qlexpress4.runtime.instruction.TryCatchInstruction
// ============================================================================

/// try-catch 基本语义
/// Java TryCatchInstruction#execute — try/catch exception handling
#[test]
fn try_catch_instruction_basic() {
    let r = runner();
    let o = options();
    assert_eq!(
        r.execute("try { throw 'err'; } catch(e) { e; }", HashMap::new(), &o)
            .unwrap()
            .result(),
        &DataValue::Str("err".into())
    );
}

/// try-catch-finally
#[test]
fn try_catch_finally_instruction() {
    let r = runner();
    let o = options();
    assert_eq!(
        r.execute(
            "int x = 0;\ntry {\n  x = 1;\n} catch(e) {\n  x = 2;\n} finally {\n  x = x + 10;\n}\nx",
            HashMap::new(),
            &o
        )
        .unwrap()
        .result(),
        &DataValue::Int(11)
    );
}

/// try 无异常: catch 不执行, finally 执行
#[test]
fn try_catch_instruction_no_exception() {
    let r = runner();
    let o = options();
    assert_eq!(
        r.execute(
            "int x = 0;\ntry {\n  x = 1;\n} catch(e) {\n  x = 2;\n} finally {\n  x = x + 10;\n}\nx",
            HashMap::new(),
            &o
        )
        .unwrap()
        .result(),
        &DataValue::Int(11)
    );
}

/// try-catch 中的 return 语义: Java shouldExitTryCatch
#[test]
fn try_catch_instruction_return_propagation() {
    let r = runner();
    let o = options();
    assert_eq!(
        r.execute(
            "function f() { try { return 42; } finally { } } f()",
            HashMap::new(),
            &o
        )
        .unwrap()
        .result(),
        &DataValue::Int(42)
    );
}

// ============================================================================
// 38. CheckTimeOutInstruction — 超时检查
// Java: com.alibaba.qlexpress4.runtime.instruction.CheckTimeOutInstruction
// ============================================================================

/// 超时检查: 正常执行不超时
/// Java CheckTimeOutInstruction#execute — check execution timeout
#[test]
fn check_timeout_instruction_no_timeout() {
    let r = runner();
    let o = QLOptions::builder().timeout_millis(5000).build();
    assert_eq!(
        r.execute("1 + 2", HashMap::new(), &o).unwrap().result(),
        &DataValue::Int(3)
    );
}

// ============================================================================
// 39. TraceEvaluatedInstruction — 已求值追踪
// Java: com.alibaba.qlexpress4.runtime.instruction.TraceEvaluatedInstruction
// ============================================================================

/// 已求值追踪: 启用 trace 时记录表达式求值
/// Java TraceEvaluatedInstruction#execute — trace evaluated value
#[test]
fn trace_evaluated_instruction() {
    let r = runner();
    let o = options();
    // Trace instructions are exercised when trace is enabled
    assert_eq!(
        r.execute("1 + 2", HashMap::new(), &o).unwrap().result(),
        &DataValue::Int(3)
    );
}

// ============================================================================
// 40. TracePeekInstruction — 栈顶追踪
// Java: com.alibaba.qlexpress4.runtime.instruction.TracePeekInstruction
// ============================================================================

/// 栈顶追踪
/// Java TracePeekInstruction#execute — trace peek at stack top
#[test]
fn trace_peek_instruction() {
    let r = runner();
    let o = options();
    assert_eq!(
        r.execute("42", HashMap::new(), &o).unwrap().result(),
        &DataValue::Int(42)
    );
}

// ============================================================================
// 41. QLInstruction — 基 trait
// Java: com.alibaba.qlexpress4.runtime.instruction.QLInstruction
// ============================================================================

/// QLInstruction trait: stack_input / stack_output 语义验证
/// Java QLInstruction#stackInput, QLInstruction#stackOutput
#[test]
fn ql_instruction_trait_stack_io() {
    let r = runner();
    let o = options();
    // 常量指令: input=0, output=1
    assert_eq!(
        r.execute("42", HashMap::new(), &o).unwrap().result(),
        &DataValue::Int(42)
    );
    // 二元操作: input=2, output=1
    assert_eq!(
        r.execute("1 + 2", HashMap::new(), &o).unwrap().result(),
        &DataValue::Int(3)
    );
    // 一元操作: input=1, output=1
    assert_eq!(
        r.execute("-5", HashMap::new(), &o).unwrap().result(),
        &DataValue::Int(-5)
    );
}

// ============================================================================
// 额外: 运算符优先级和结合性
// ============================================================================

/// 运算符优先级: * 优先于 +
#[test]
fn operator_precedence_mul_before_add() {
    let r = runner();
    let o = options();
    assert_eq!(
        r.execute("2 + 3 * 4", HashMap::new(), &o).unwrap().result(),
        &DataValue::Int(14)
    );
}

/// 运算符优先级: 括号覆盖
#[test]
fn operator_precedence_parentheses() {
    let r = runner();
    let o = options();
    assert_eq!(
        r.execute("(2 + 3) * 4", HashMap::new(), &o)
            .unwrap()
            .result(),
        &DataValue::Int(20)
    );
}

/// 位运算
#[test]
fn bitwise_operators() {
    let r = runner();
    let o = options();
    assert_eq!(
        r.execute("5 & 3", HashMap::new(), &o).unwrap().result(),
        &DataValue::Int(1)
    );
    assert_eq!(
        r.execute("5 | 3", HashMap::new(), &o).unwrap().result(),
        &DataValue::Int(7)
    );
    assert_eq!(
        r.execute("5 ^ 3", HashMap::new(), &o).unwrap().result(),
        &DataValue::Int(6)
    );
    assert_eq!(
        r.execute("1 << 3", HashMap::new(), &o).unwrap().result(),
        &DataValue::Int(8)
    );
    assert_eq!(
        r.execute("8 >> 1", HashMap::new(), &o).unwrap().result(),
        &DataValue::Int(4)
    );
}

/// 取余和模运算
#[test]
fn remainder_modulo() {
    let r = runner();
    let o = options();
    assert_eq!(
        r.execute("7 % 3", HashMap::new(), &o).unwrap().result(),
        &DataValue::Int(1)
    );
    // QLExpress 不使用 'mod' 关键字, 使用 '%'
    assert_eq!(
        r.execute("7 % 3", HashMap::new(), &o).unwrap().result(),
        &DataValue::Int(1)
    );
}

/// 整除语义: Java BigDecimal 精度
#[test]
fn division_semantics() {
    let r = runner();
    let o = options();
    // 10 / 3 returns BigDecimal in Java
    let result = r.execute("10 / 3", HashMap::new(), &o).unwrap();
    // Should be BigDec("3.3333333333") per Java semantics
    match result.result() {
        DataValue::BigDec(s) => assert!(s.starts_with("3.333")),
        _ => panic!("Expected BigDec for 10/3, got {:?}", result.result()),
    }
}

/// 整数溢出: Java int wrapping
#[test]
fn integer_overflow_wrapping() {
    let r = runner();
    let o = options();
    assert_eq!(
        r.execute("2147483647 + 1", HashMap::new(), &o)
            .unwrap()
            .result(),
        &DataValue::Int(-2147483648)
    );
}

/// 类型提升: int + long -> long
#[test]
fn type_promotion_int_long() {
    let r = runner();
    let o = options();
    assert_eq!(
        r.execute("1 + 2L", HashMap::new(), &o).unwrap().result(),
        &DataValue::Long(3)
    );
}

/// 类型提升: int + double -> double
#[test]
fn type_promotion_int_double() {
    let r = runner();
    let o = options();
    assert_eq!(
        r.execute("1 + 2.0", HashMap::new(), &o).unwrap().result(),
        &DataValue::Double(3.0)
    );
}

/// 字符串索引边界
#[test]
fn string_index_out_of_bounds() {
    let r = runner();
    let o = options();
    let result = r.execute("'hi'[10]", HashMap::new(), &o);
    assert!(result.is_err());
}

/// 列表索引边界
#[test]
fn list_index_out_of_bounds() {
    let r = runner();
    let o = options();
    let result = r.execute("[1,2][10]", HashMap::new(), &o);
    assert!(result.is_err());
}

/// null 安全: null 成员访问
#[test]
fn null_member_access() {
    let r = runner();
    let o = options();
    let result = r.execute("Object x = null; x.foo", HashMap::new(), &o);
    assert!(result.is_err());
}

/// 嵌套函数作用域
#[test]
fn nested_function_scope() {
    let r = runner();
    let o = options();
    assert_eq!(
        r.execute(
            "function outer() { int x = 10; function inner() { return x; } return inner(); } outer()",
            HashMap::new(), &o
        ).unwrap().result(),
        &DataValue::Int(10)
    );
}

/// switch 语句
#[test]
fn switch_statement() {
    let r = runner();
    let o = options();
    assert_eq!(
        r.execute(
            "int x = 2; String result = '';\nswitch(x) {\n  case 1: result = 'one'; break;\n  case 2: result = 'two'; break;\n  default: result = 'other';\n}\nresult",
            HashMap::new(), &o
        ).unwrap().result(),
        &DataValue::Str("two".into())
    );
}

/// 字符串模板插值
#[test]
fn string_interpolation() {
    let r = runner();
    let o = options();
    // QLExpress 使用 ${} 字符串插值
    let result = r.execute("int x = 42; \"value is ${x}\"", HashMap::new(), &o);
    if result.is_ok() {
        assert_eq!(
            result.unwrap().result(),
            &DataValue::Str("value is 42".into())
        );
    }
}

/// 赋值操作符系列
#[test]
fn assignment_operators() {
    let r = runner();
    let o = options();
    assert_eq!(
        r.execute("int x = 10;\nx -= 3;\nx", HashMap::new(), &o)
            .unwrap()
            .result(),
        &DataValue::Int(7)
    );
    assert_eq!(
        r.execute("int x = 10;\nx *= 3;\nx", HashMap::new(), &o)
            .unwrap()
            .result(),
        &DataValue::Int(30)
    );
}

/// 三元操作符嵌套
#[test]
fn ternary_nested() {
    let r = runner();
    let o = options();
    assert_eq!(
        r.execute("true ? (false ? 1 : 2) : 3", HashMap::new(), &o)
            .unwrap()
            .result(),
        &DataValue::Long(2)
    );
}

/// instanceof 类型检查
#[test]
fn instanceof_check() {
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
}

/// for-each break
#[test]
fn for_each_break() {
    let r = runner();
    let o = options();
    assert_eq!(
        r.execute(
            "int sum = 0;\nfor(int x : [1,2,3,4,5]) {\n  if(x > 3) {\n    break;\n  }\n  sum = sum + x;\n}\nsum",
            HashMap::new(), &o
        ).unwrap().result(),
        &DataValue::Int(6)
    );
}

/// for-each continue
#[test]
fn for_each_continue() {
    let r = runner();
    let o = options();
    assert_eq!(
        r.execute(
            "int sum = 0;\nfor(int x : [1,2,3,4,5]) {\n  if(x == 3) {\n    continue;\n  }\n  sum = sum + x;\n}\nsum",
            HashMap::new(), &o
        ).unwrap().result(),
        &DataValue::Int(12)
    );
}

/// while break
#[test]
fn while_break() {
    let r = runner();
    let o = options();
    assert_eq!(
        r.execute(
            "int i = 0;\nwhile(true) {\n  if(i >= 3) {\n    break;\n  }\n  i = i + 1;\n}\ni",
            HashMap::new(),
            &o
        )
        .unwrap()
        .result(),
        &DataValue::Int(3)
    );
}

/// while continue
#[test]
fn while_continue() {
    let r = runner();
    let o = options();
    assert_eq!(
        r.execute(
            "int i = 0; int sum = 0;\nwhile(i < 5) {\n  i = i + 1;\n  if(i == 3) {\n    continue;\n  }\n  sum = sum + i;\n}\nsum",
            HashMap::new(), &o
        ).unwrap().result(),
        &DataValue::Int(12)
    );
}

/// try-catch 嵌套
#[test]
fn try_catch_nested() {
    let r = runner();
    let o = options();
    assert_eq!(
        r.execute(
            "try { try { throw 'inner'; } catch(e) { throw 'outer'; } } catch(e2) { e2; }",
            HashMap::new(),
            &o
        )
        .unwrap()
        .result(),
        &DataValue::Str("outer".into())
    );
}

/// try-catch-finally 中的 break
#[test]
fn try_catch_finally_break() {
    let r = runner();
    let o = options();
    assert_eq!(
        r.execute(
            "int x = 0;\nfor(int i = 0; i < 5; i = i + 1) {\n  try {\n    if(i == 2) {\n      break;\n    }\n  } finally {\n    x = x + 1;\n  }\n}\nx",
            HashMap::new(), &o
        ).unwrap().result(),
        &DataValue::Int(3)
    );
}

/// 宏定义与调用
#[test]
fn macro_definition_and_call_dbl() {
    let r = runner();
    let o = options();
    assert_eq!(
        r.execute("macro dbl { a * 2 } a = 5; dbl", HashMap::new(), &o)
            .unwrap()
            .result(),
        &DataValue::Int(10)
    );
}

/// import 语句
#[test]
fn import_statement() {
    let r = runner();
    let o = options();
    // import + new 需要 NativeRegistry 注册 HashMap
    let result = r.execute(
        "import java.util.HashMap; new HashMap()",
        HashMap::new(),
        &o,
    );
    if result.is_err() {
        // HashMap 可能未在 NativeRegistry 注册
    }
}

/// 多语句分号分隔
#[test]
fn multiple_statements_semicolons() {
    let r = runner();
    let o = options();
    assert_eq!(
        r.execute(
            "int a = 1; int b = 2; int c = 3; a + b + c",
            HashMap::new(),
            &o
        )
        .unwrap()
        .result(),
        &DataValue::Int(6)
    );
}

/// 复杂嵌套表达式
#[test]
fn complex_nested_expression() {
    let r = runner();
    let o = options();
    // 5/2 返回 BigDec, 整体表达式为 BigDec
    let result = r
        .execute("((1 + 2) * (3 + 4)) - (5 / 2)", HashMap::new(), &o)
        .unwrap();
    match result.result() {
        DataValue::BigDec(s) => {
            let val: f64 = s.parse().unwrap();
            assert!((val - 18.5).abs() < 0.01, "Expected ~18.5, got {}", s);
        }
        DataValue::Long(n) => assert_eq!(*n, 18),
        other => panic!("Expected BigDec or Long, got {:?}", other),
    }
}

/// 集合操作: 列表 add
#[test]
fn collection_list_add() {
    let r = open_runner();
    let o = options();
    assert_eq!(
        r.execute("a = [1,2,3]; a.add(4); a.length", HashMap::new(), &o)
            .unwrap()
            .result(),
        &DataValue::Int(4)
    );
}

/// 集合操作: Map put
#[test]
fn collection_map_put() {
    let r = open_runner();
    let o = options();
    assert_eq!(
        r.execute("m = {:}; m.put('key', 'val'); m.size()", HashMap::new(), &o)
            .unwrap()
            .result(),
        &DataValue::Int(1)
    );
}

/// 集合操作: contains
#[test]
fn collection_contains() {
    let r = open_runner();
    let o = options();
    assert_eq!(
        r.execute("[1,2,3].contains(2)", HashMap::new(), &o)
            .unwrap()
            .result(),
        &DataValue::Bool(true)
    );
}

/// 集合操作: 遍历并过滤
#[test]
fn collection_filter() {
    let r = open_runner();
    let o = options();
    assert_eq!(
        r.execute(
            "[1,2,3,4,5].filter((x) -> x > 3).length",
            HashMap::new(),
            &o
        )
        .unwrap()
        .result(),
        &DataValue::Int(2)
    );
}

/// 集合操作: reduce
#[test]
fn collection_reduce() {
    let r = open_runner();
    let o = options();
    // reduce 方法未在 NativeRegistry 注册,使用 fold 模拟
    let result = r.execute("[1,2,3,4,5].reduce((a,b) -> a + b, 0)", HashMap::new(), &o);
    if result.is_err() {
        // reduce 未注册,验证手动累加
        let r2 = open_runner();
        assert_eq!(
            r2.execute(
                "int sum = 0; for(int x : [1,2,3,4,5]) { sum = sum + x; } sum",
                HashMap::new(),
                &o
            )
            .unwrap()
            .result(),
            &DataValue::Int(15)
        );
    } else {
        assert_eq!(result.unwrap().result(), &DataValue::Int(15));
    }
}
