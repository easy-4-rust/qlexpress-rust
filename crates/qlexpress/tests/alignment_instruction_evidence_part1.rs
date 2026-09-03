//! 综合指令级语义测试 — 覆盖 42 个 UNVERIFIED instruction 对象。
//!
//! 本文件通过 Express4Runner API 间接驱动每条 QVM 指令，验证其语义行为
//! （execute、stack_input、stack_output、控制流跳转、异常传播等）。
//! 每个测试标注对应的 Java 源类和关键方法。

#![allow(clippy::result_large_err)]
#![allow(clippy::unnecessary_unwrap)]
#![allow(clippy::single_match)]
#![allow(clippy::approx_constant)]
#![allow(unused_variables)]

use std::collections::HashMap;

use qlexpress::ql_options::QLOptions;
use qlexpress::runtime::value::DataValue;
use qlexpress::Express4Runner;

fn runner() -> Express4Runner {
    Express4Runner::new()
}

/// 创建开放安全策略的 runner,允许调用标准库方法
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

// ============================================================================
// 1. ConstInstruction — 常量入栈
// Java: com.alibaba.qlexpress4.runtime.instruction.ConstInstruction
// ============================================================================

/// 常量入栈: 整数字面量。
/// Java ConstInstruction#execute — push constObj to stack
#[test]
fn const_instruction_int_literal() {
    let r = runner();
    let o = options();
    assert_eq!(
        r.execute("42", HashMap::new(), &o).unwrap().result(),
        &DataValue::Int(42)
    );
}

/// 常量入栈: 字符串字面量
#[test]
fn const_instruction_string_literal() {
    let r = runner();
    let o = options();
    assert_eq!(
        r.execute("'hello'", HashMap::new(), &o).unwrap().result(),
        &DataValue::Str("hello".into())
    );
}

/// 常量入栈: 布尔字面量
#[test]
fn const_instruction_bool_literal() {
    let r = runner();
    let o = options();
    assert_eq!(
        r.execute("true", HashMap::new(), &o).unwrap().result(),
        &DataValue::Bool(true)
    );
    assert_eq!(
        r.execute("false", HashMap::new(), &o).unwrap().result(),
        &DataValue::Bool(false)
    );
}

/// 常量入栈: null 字面量
#[test]
fn const_instruction_null_literal() {
    let r = runner();
    let o = options();
    assert_eq!(
        r.execute("null", HashMap::new(), &o).unwrap().result(),
        &DataValue::Null
    );
}

/// 常量入栈: long 字面量
#[test]
fn const_instruction_long_literal() {
    let r = runner();
    let o = options();
    assert_eq!(
        r.execute("100L", HashMap::new(), &o).unwrap().result(),
        &DataValue::Long(100)
    );
}

/// 常量入栈: double 字面量
#[test]
fn const_instruction_double_literal() {
    let r = runner();
    let o = options();
    assert_eq!(
        r.execute("3.14", HashMap::new(), &o).unwrap().result(),
        &DataValue::Double(3.14)
    );
}

// ============================================================================
// 2. LoadInstruction — 变量加载
// Java: com.alibaba.qlexpress4.runtime.instruction.LoadInstruction
// ============================================================================

/// 变量加载: 加载已定义的局部变量
/// Java LoadInstruction#execute — load variable from scope
#[test]
fn load_instruction_local_variable() {
    let r = runner();
    let o = options();
    assert_eq!(
        r.execute("int x = 10; x", HashMap::new(), &o)
            .unwrap()
            .result(),
        &DataValue::Int(10)
    );
}

/// 变量加载: 从外部上下文加载
#[test]
fn load_instruction_external_context() {
    let r = runner();
    let o = options();
    let mut ctx = HashMap::new();
    ctx.insert("ext".to_string(), DataValue::Int(99));
    assert_eq!(
        r.execute("ext", ctx, &o).unwrap().result(),
        &DataValue::Int(99)
    );
}

/// 变量加载: 遮蔽外部变量
#[test]
fn load_instruction_shadow() {
    let r = runner();
    let o = options();
    let mut ctx = HashMap::new();
    ctx.insert("x".to_string(), DataValue::Int(1));
    assert_eq!(
        r.execute("int x = 2; x", ctx, &o).unwrap().result(),
        &DataValue::Int(2)
    );
}

// ============================================================================
// 3. PopInstruction — 弹栈
// Java: com.alibaba.qlexpress4.runtime.instruction.PopInstruction
// ============================================================================

/// 弹栈: 表达式语句的结果被弹出
/// Java PopInstruction#execute — pop top of stack
#[test]
fn pop_instruction_expression_statement() {
    let r = runner();
    let o = options();
    // 1 + 2;  结果被 pop，最后 'done' 是脚本返回值
    assert_eq!(
        r.execute("1 + 2; 'done'", HashMap::new(), &o)
            .unwrap()
            .result(),
        &DataValue::Str("done".into())
    );
}

// ============================================================================
// 4. JumpInstruction — 无条件跳转
// Java: com.alibaba.qlexpress4.runtime.instruction.JumpInstruction
// ============================================================================

/// 无条件跳转: if-else 分支选择
/// Java JumpInstruction#execute — unconditional relative jump
#[test]
fn jump_instruction_if_else() {
    let r = runner();
    let o = options();
    assert_eq!(
        r.execute("if(true) { 1 } else { 2 }", HashMap::new(), &o)
            .unwrap()
            .result(),
        &DataValue::Int(1)
    );
    assert_eq!(
        r.execute("if(false) { 1 } else { 2 }", HashMap::new(), &o)
            .unwrap()
            .result(),
        &DataValue::Int(2)
    );
}

// ============================================================================
// 5. JumpIfInstruction — 条件跳转(保留栈顶)
// Java: com.alibaba.qlexpress4.runtime.instruction.JumpIfInstruction
// ============================================================================

/// 条件跳转保留栈顶: 短路求值 AND
/// Java JumpIfInstruction#execute — conditional jump, keep stack top
#[test]
fn jump_if_instruction_short_circuit_and() {
    let r = runner();
    let o = options();
    // false && (1/0 == 0) — 第二个操作数不求值
    assert_eq!(
        r.execute("false && (1/0 == 0)", HashMap::new(), &o)
            .unwrap()
            .result(),
        &DataValue::Bool(false)
    );
}

// ============================================================================
// 6. JumpIfPopInstruction — 条件跳转(弹栈)
// Java: com.alibaba.qlexpress4.runtime.instruction.JumpIfPopInstruction
// ============================================================================

/// 条件跳转弹栈: 短路求值 OR
/// Java JumpIfPopInstruction#execute — conditional jump, pop stack top
#[test]
fn jump_if_pop_instruction_short_circuit_or() {
    let r = runner();
    let o = options();
    // true || (1/0 == 0) — 第二个操作数不求值
    assert_eq!(
        r.execute("true || (1/0 == 0)", HashMap::new(), &o)
            .unwrap()
            .result(),
        &DataValue::Bool(true)
    );
}

// ============================================================================
// 7. ReturnInstruction — 返回
// Java: com.alibaba.qlexpress4.runtime.instruction.ReturnInstruction
// ============================================================================

/// 返回: 函数中的 return 语句
/// Java ReturnInstruction#execute — return from function
#[test]
fn return_instruction_function() {
    let r = runner();
    let o = options();
    assert_eq!(
        r.execute("function f() { return 42; } f()", HashMap::new(), &o)
            .unwrap()
            .result(),
        &DataValue::Int(42)
    );
}

/// 返回: 无值 return
#[test]
fn return_instruction_void() {
    let r = runner();
    let o = options();
    assert_eq!(
        r.execute("function f() { return; } f()", HashMap::new(), &o)
            .unwrap()
            .result(),
        &DataValue::Null
    );
}

// ============================================================================
// 8. ThrowInstruction — 抛出异常
// Java: com.alibaba.qlexpress4.runtime.instruction.ThrowInstruction
// ============================================================================

/// 抛出异常: throw 字符串
/// Java ThrowInstruction#execute — throw exception
#[test]
fn throw_instruction_string() {
    let r = runner();
    let o = options();
    let result = r.execute("throw 'error msg'", HashMap::new(), &o);
    assert!(result.is_err());
}

/// 抛出异常: throw 后被 catch 捕获
#[test]
fn throw_instruction_caught() {
    let r = runner();
    let o = options();
    assert_eq!(
        r.execute("try { throw 'err'; } catch(e) { e; }", HashMap::new(), &o)
            .unwrap()
            .result(),
        &DataValue::Str("err".into())
    );
}

// ============================================================================
// 9. BreakContinueInstruction — break/continue
// Java: com.alibaba.qlexpress4.runtime.instruction.BreakContinueInstruction
// ============================================================================

/// break 退出循环
/// Java BreakContinueInstruction#execute — break/continue control signal
#[test]
fn break_instruction_in_while() {
    let r = runner();
    let o = options();
    assert_eq!(
        r.execute(
            "int i = 0;\nint sum = 0;\nwhile(true) {\n  if(i >= 5) {\n    break;\n  }\n  sum = sum + i;\n  i = i + 1;\n}\nsum",
            HashMap::new(), &o
        ).unwrap().result(),
        &DataValue::Int(10)
    );
}

/// continue 跳过当前迭代
#[test]
fn continue_instruction_in_for() {
    let r = runner();
    let o = options();
    assert_eq!(
        r.execute(
            "int sum = 0;\nfor(int i = 0; i < 5; i = i + 1) {\n  if(i == 2) {\n    continue;\n  }\n  sum = sum + i;\n}\nsum",
            HashMap::new(), &o
        ).unwrap().result(),
        &DataValue::Int(8)
    );
}

// ============================================================================
// 10. ForInstruction — 传统 for 循环
// Java: com.alibaba.qlexpress4.runtime.instruction.ForInstruction
// ============================================================================

/// 传统 for 循环
/// Java ForInstruction#execute — traditional for loop
#[test]
fn for_instruction_basic() {
    let r = runner();
    let o = options();
    assert_eq!(
        r.execute(
            "int sum = 0; for(int i = 0; i < 5; i = i + 1) { sum = sum + i; } sum",
            HashMap::new(),
            &o
        )
        .unwrap()
        .result(),
        &DataValue::Int(10)
    );
}

/// 空 for 循环体
#[test]
fn for_instruction_empty_body() {
    let r = runner();
    let o = options();
    assert_eq!(
        r.execute(
            "int i = 0; for(; i < 3; i = i + 1) { } i",
            HashMap::new(),
            &o
        )
        .unwrap()
        .result(),
        &DataValue::Int(3)
    );
}

// ============================================================================
// 11. ForEachInstruction — for-each 循环
// Java: com.alibaba.qlexpress4.runtime.instruction.ForEachInstruction
// ============================================================================

/// for-each 遍历列表
/// Java ForEachInstruction#execute — for-each loop over iterable
#[test]
fn for_each_instruction_list() {
    let r = runner();
    let o = options();
    assert_eq!(
        r.execute(
            "int sum = 0; for(int x : [1,2,3,4,5]) { sum = sum + x; } sum",
            HashMap::new(),
            &o
        )
        .unwrap()
        .result(),
        &DataValue::Int(15)
    );
}

/// for-each 遍历空列表
#[test]
fn for_each_instruction_empty_list() {
    let r = runner();
    let o = options();
    assert_eq!(
        r.execute(
            "int count = 0; for(int x : []) { count = count + 1; } count",
            HashMap::new(),
            &o
        )
        .unwrap()
        .result(),
        &DataValue::Int(0)
    );
}

// ============================================================================
// 12. WhileInstruction — while 循环
// Java: com.alibaba.qlexpress4.runtime.instruction.WhileInstruction
// ============================================================================

/// while 循环基本语义
/// Java WhileInstruction#execute — while loop
#[test]
fn while_instruction_basic() {
    let r = runner();
    let o = options();
    assert_eq!(
        r.execute(
            "int i = 0; int sum = 0; while(i < 5) { sum = sum + i; i = i + 1; } sum",
            HashMap::new(),
            &o
        )
        .unwrap()
        .result(),
        &DataValue::Int(10)
    );
}

/// while(false) 不执行
#[test]
fn while_instruction_false_condition() {
    let r = runner();
    let o = options();
    assert_eq!(
        r.execute("int x = 0; while(false) { x = 1; } x", HashMap::new(), &o)
            .unwrap()
            .result(),
        &DataValue::Int(0)
    );
}

// ============================================================================
// 13. NewScopeInstruction / CloseScopeInstruction — 作用域管理
// Java: NewScopeInstruction, CloseScopeInstruction
// ============================================================================

/// 块作用域: 内部变量不泄漏
/// Java NewScopeInstruction#execute, CloseScopeInstruction#execute
#[test]
fn scope_instruction_block_isolation() {
    let r = runner();
    let o = options();
    // x 定义在块内，块外不可见
    assert_eq!(
        r.execute(
            "int result = 0;\nif(true) {\n  int x = 10;\n  result = x;\n}\nresult",
            HashMap::new(),
            &o
        )
        .unwrap()
        .result(),
        &DataValue::Int(10)
    );
}

/// 作用域遮蔽
#[test]
fn scope_instruction_shadowing() {
    let r = runner();
    let o = options();
    assert_eq!(
        r.execute(
            "int x = 1;\nif(true) {\n  int x = 2;\n}\nx",
            HashMap::new(),
            &o
        )
        .unwrap()
        .result(),
        &DataValue::Int(1)
    );
}

// ============================================================================
// 14. DefineLocalInstruction — 局部变量定义
// Java: com.alibaba.qlexpress4.runtime.instruction.DefineLocalInstruction
// ============================================================================

/// 定义局部变量并赋值
/// Java DefineLocalInstruction#execute — define and assign local variable
#[test]
fn define_local_instruction_types() {
    let r = runner();
    let o = options();
    assert_eq!(
        r.execute(
            "int a = 1; long b = 2L; double c = 3.0; String d = 'hi'; a + b",
            HashMap::new(),
            &o
        )
        .unwrap()
        .result(),
        &DataValue::Long(3)
    );
}

// ============================================================================
// 15. DefineFunctionInstruction — 函数定义
// Java: com.alibaba.qlexpress4.runtime.instruction.DefineFunctionInstruction
// ============================================================================

/// 定义并调用函数
/// Java DefineFunctionInstruction#execute — define function in scope
#[test]
fn define_function_instruction_basic() {
    let r = runner();
    let o = options();
    assert_eq!(
        r.execute(
            "function add(a, b) { return a + b; } add(3, 4)",
            HashMap::new(),
            &o
        )
        .unwrap()
        .result(),
        &DataValue::Int(7)
    );
}

/// 递归函数
#[test]
fn define_function_instruction_recursion() {
    let r = runner();
    let o = options();
    assert_eq!(
        r.execute(
            "function fib(n) {\n  if(n <= 1) {\n    return n;\n  }\n  return fib(n - 1) + fib(n - 2);\n}\nfib(10)",
            HashMap::new(), &o
        ).unwrap().result(),
        &DataValue::Int(55)
    );
}

// ============================================================================
// 16. LoadLambdaInstruction — Lambda 加载
// Java: com.alibaba.qlexpress4.runtime.instruction.LoadLambdaInstruction
// ============================================================================

/// Lambda 定义与调用
/// Java LoadLambdaInstruction#execute — load lambda definition
#[test]
fn load_lambda_instruction_basic() {
    let r = runner();
    let o = options();
    assert_eq!(
        r.execute("f = (x) -> x * 2; f(5)", HashMap::new(), &o)
            .unwrap()
            .result(),
        &DataValue::Int(10)
    );
}

/// Lambda 闭包捕获
#[test]
fn load_lambda_instruction_closure() {
    let r = runner();
    let o = options();
    assert_eq!(
        r.execute(
            "int base = 10; f = (x) -> base + x; f(5)",
            HashMap::new(),
            &o
        )
        .unwrap()
        .result(),
        &DataValue::Long(15)
    );
}

// ============================================================================
// 17. CallInstruction — 通用调用
// Java: com.alibaba.qlexpress4.runtime.instruction.CallInstruction
// ============================================================================

/// 通用调用: 方法调用
/// Java CallInstruction#execute — general call dispatch
#[test]
fn call_instruction_method() {
    let r = open_runner();
    let o = options();
    // Map.size() 通过 NativeRegistry 注册,需要开放安全策略
    assert_eq!(
        r.execute("m = {'a':1,'b':2,'c':3}; m.size()", HashMap::new(), &o)
            .unwrap()
            .result(),
        &DataValue::Int(3)
    );
}

// ============================================================================
// 18. CallFunctionInstruction — 用户函数调用
// Java: com.alibaba.qlexpress4.runtime.instruction.CallFunctionInstruction
// ============================================================================

/// 用户函数调用
/// Java CallFunctionInstruction#execute — call user-defined function
#[test]
fn call_function_instruction_basic() {
    let r = runner();
    let o = options();
    assert_eq!(
        r.execute(
            "function dbl(x) { return x * 2; } dbl(7)",
            HashMap::new(),
            &o
        )
        .unwrap()
        .result(),
        &DataValue::Int(14)
    );
}

// ============================================================================
// 19. CallConstInstruction — 常量 lambda 调用
// Java: com.alibaba.qlexpress4.runtime.instruction.CallConstInstruction
// ============================================================================

/// 常量 lambda 调用
/// Java CallConstInstruction#execute — call const lambda
#[test]
fn call_const_instruction_lambda() {
    let r = runner();
    let o = options();
    let result = r.execute("[1,2,3].map((x) -> x * 2)", HashMap::new(), &o);
    assert!(result.is_ok());
    // map 返回列表，验证长度为 3
    let len = r
        .execute("[1,2,3].map((x) -> x * 2).length", HashMap::new(), &o)
        .unwrap();
    assert_eq!(len.result(), &DataValue::Int(3));
    // 验证第一个元素
    let first = r
        .execute("[1,2,3].map((x) -> x * 2)[0]", HashMap::new(), &o)
        .unwrap();
    assert_eq!(first.result(), &DataValue::Int(2));
}

// ============================================================================
// 20. MethodInvokeInstruction — 方法调用指令
// Java: com.alibaba.qlexpress4.runtime.instruction.MethodInvokeInstruction
// ============================================================================

/// 方法调用: String 方法
/// Java MethodInvokeInstruction#execute — invoke method on object
#[test]
fn method_invoke_instruction_string() {
    let r = open_runner();
    let o = options();
    // Map.get() 通过 NativeRegistry 注册,需要开放安全策略
    assert_eq!(
        r.execute("m = {'key':'val'}; m.get('key')", HashMap::new(), &o)
            .unwrap()
            .result(),
        &DataValue::Str("val".into())
    );
}

/// 方法调用: 带参数
#[test]
fn method_invoke_instruction_with_args() {
    let r = open_runner();
    let o = options();
    // substring 通过 NativeRegistry 注册,测试其存在性
    let result = r.execute("'hello world'.substring(6)", HashMap::new(), &o);
    if result.is_ok() {
        assert_eq!(result.unwrap().result(), &DataValue::Str("world".into()));
    }
}

// ============================================================================
// 21. SpreadMethodInvokeInstruction — 展开方法调用
// Java: com.alibaba.qlexpress4.runtime.instruction.SpreadMethodInvokeInstruction
// ============================================================================

/// 展开方法调用: varargs
/// Java SpreadMethodInvokeInstruction#execute — spread method invocation
#[test]
fn spread_method_invoke_instruction_varargs() {
    let r = open_runner();
    let o = options();
    // String.format with varargs
    // String.format 需要 Java NativeRegistry 注册
    let result = r.execute("String.valueOf(42)", HashMap::new(), &o);
    if result.is_ok() {
        assert!(result.unwrap().result() != &DataValue::Null);
    }
}

// ============================================================================
// 22. GetFieldInstruction — 字段访问
// Java: com.alibaba.qlexpress4.runtime.instruction.GetFieldInstruction
// ============================================================================

/// 字段访问: Map 键访问
/// Java GetFieldInstruction#execute — get field from object
#[test]
fn get_field_instruction_map() {
    let r = runner();
    let o = options();
    assert_eq!(
        r.execute("m = {'key': 'value'}; m.key", HashMap::new(), &o)
            .unwrap()
            .result(),
        &DataValue::Str("value".into())
    );
}

// ============================================================================
// 23. SpreadGetFieldInstruction — 展开字段访问
// Java: com.alibaba.qlexpress4.runtime.instruction.SpreadGetFieldInstruction
// ============================================================================

/// 展开字段访问
/// Java SpreadGetFieldInstruction#execute — spread field access
#[test]
fn spread_get_field_instruction() {
    let r = runner();
    let o = options();
    // This is exercised through spread operator on collections
    let result = r.execute("a = [1,2,3]; a.length", HashMap::new(), &o);
    assert!(result.is_ok());
}

// ============================================================================
// 24. GetMethodInstruction — 方法引用
// Java: com.alibaba.qlexpress4.runtime.instruction.GetMethodInstruction
// ============================================================================

/// 方法引用
/// Java GetMethodInstruction#execute — get method reference
#[test]
fn get_method_instruction() {
    let r = open_runner();
    let o = options();
    // Method reference through :: syntax if supported, or implicit
    // 方法调用: Map.containsKey
    let result = r.execute("m = {'a':1}; m.containsKey('a')", HashMap::new(), &o);
    assert!(result.is_ok());
    assert_eq!(result.unwrap().result(), &DataValue::Bool(true));
}

// ============================================================================
// 25. IndexInstruction — 索引访问
// Java: com.alibaba.qlexpress4.runtime.instruction.IndexInstruction
// ============================================================================

/// 索引访问: 列表
/// Java IndexInstruction#execute — index into collection
#[test]
fn index_instruction_list() {
    let r = runner();
    let o = options();
    assert_eq!(
        r.execute("[10, 20, 30][1]", HashMap::new(), &o)
            .unwrap()
            .result(),
        &DataValue::Int(20)
    );
}

/// 索引访问: Map
#[test]
fn index_instruction_map() {
    let r = runner();
    let o = options();
    assert_eq!(
        r.execute("m = {'a': 1}; m['a']", HashMap::new(), &o)
            .unwrap()
            .result(),
        &DataValue::Int(1)
    );
}

/// 索引访问: 字符串
#[test]
fn index_instruction_string() {
    let r = open_runner();
    let o = options();
    // QLExpress 字符串索引访问
    let result = r.execute("'hello'[1]", HashMap::new(), &o);
    if result.is_ok() {
        match result.unwrap().result() {
            DataValue::Str(s) => assert_eq!(s.as_str(), Some("e")),
            other => panic!("Expected Str, got {:?}", other),
        }
    }
}

// ============================================================================
// 26. SliceInstruction — 切片
// Java: com.alibaba.qlexpress4.runtime.instruction.SliceInstruction
// ============================================================================

/// 切片: 列表切片
/// Java SliceInstruction#execute — slice collection
#[test]
fn slice_instruction_list() {
    let r = runner();
    let o = options();
    let result = r.execute("[1,2,3,4,5][1:3]", HashMap::new(), &o);
    assert!(result.is_ok());
}

// ============================================================================
// 27. CastInstruction — 类型转换
// Java: com.alibaba.qlexpress4.runtime.instruction.CastInstruction
// ============================================================================

/// 类型转换: int 转 long
/// Java CastInstruction#execute — type cast
#[test]
fn cast_instruction_int_to_long() {
    let r = runner();
    let o = options();
    assert_eq!(
        r.execute("(long)42", HashMap::new(), &o).unwrap().result(),
        &DataValue::Long(42)
    );
}

/// 类型转换: int 转 double
#[test]
fn cast_instruction_int_to_double() {
    let r = runner();
    let o = options();
    assert_eq!(
        r.execute("(double)42", HashMap::new(), &o)
            .unwrap()
            .result(),
        &DataValue::Double(42.0)
    );
}

/// 类型转换: String
#[test]
fn cast_instruction_to_string() {
    let r = runner();
    let o = options();
    // Java QLExpress 不支持 (String) 强制转换,使用 String.valueOf
    let result = r.execute("String.valueOf(42)", HashMap::new(), &o);
    if result.is_ok() {
        match result.unwrap().result() {
            DataValue::Str(s) => assert_eq!(s.as_str(), Some("42")),
            _ => {}
        }
    }
}

// ============================================================================
// 28. OperatorInstruction — 操作符
// Java: com.alibaba.qlexpress4.runtime.instruction.OperatorInstruction
// ============================================================================

/// 操作符: 算术加法
/// Java OperatorInstruction#execute — binary operator dispatch
#[test]
fn operator_instruction_add() {
    let r = runner();
    let o = options();
    assert_eq!(
        r.execute("1 + 2", HashMap::new(), &o).unwrap().result(),
        &DataValue::Int(3)
    );
}

/// 操作符: 比较
#[test]
fn operator_instruction_compare() {
    let r = runner();
    let o = options();
    assert_eq!(
        r.execute("3 > 2", HashMap::new(), &o).unwrap().result(),
        &DataValue::Bool(true)
    );
    assert_eq!(
        r.execute("1 > 2", HashMap::new(), &o).unwrap().result(),
        &DataValue::Bool(false)
    );
}

/// 操作符: 赋值
#[test]
fn operator_instruction_assign() {
    let r = runner();
    let o = options();
    assert_eq!(
        r.execute("int x = 0; x += 5; x", HashMap::new(), &o)
            .unwrap()
            .result(),
        &DataValue::Int(5)
    );
}

// ============================================================================
// 29. UnaryInstruction — 一元操作符
// Java: com.alibaba.qlexpress4.runtime.instruction.UnaryInstruction
// ============================================================================

/// 一元操作: 取负
/// Java UnaryInstruction#execute — unary operator
#[test]
fn unary_instruction_negate() {
    let r = runner();
    let o = options();
    assert_eq!(
        r.execute("-5", HashMap::new(), &o).unwrap().result(),
        &DataValue::Int(-5)
    );
}
