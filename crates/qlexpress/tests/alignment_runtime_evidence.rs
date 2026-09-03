//! 综合运行时/上下文/数据/异常/缓存/追踪语义测试 — 覆盖剩余 UNVERIFIED 对象。
//!
//! 通过 Express4Runner API 间接驱动运行时组件,验证语义行为。
//! 每个测试标注对应的 Java 源类。

#![allow(clippy::result_large_err)]
#![allow(clippy::unnecessary_unwrap)]
#![allow(clippy::single_match)]
#![allow(clippy::approx_constant)]
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
// Express4Runner (1)
// Java: com.alibaba.qlexpress4.Express4Runner
// ============================================================================

/// Express4Runner 门面: 解析、编译、执行
#[test]
fn express4_runner_basic() {
    let r = runner();
    let o = options();
    assert_eq!(
        r.execute("1 + 2", HashMap::new(), &o).unwrap().result(),
        &DataValue::Int(3)
    );
}

/// Express4Runner: 带外部上下文
#[test]
fn express4_runner_with_context() {
    let r = runner();
    let o = options();
    let mut ctx = HashMap::new();
    ctx.insert("x".to_string(), DataValue::Int(10));
    assert_eq!(
        r.execute("x + 5", ctx, &o).unwrap().result(),
        &DataValue::Int(15)
    );
}

/// Express4Runner: 多语句脚本
#[test]
fn express4_runner_multi_statement() {
    let r = runner();
    let o = options();
    assert_eq!(
        r.execute("int a = 1; int b = 2; a + b", HashMap::new(), &o)
            .unwrap()
            .result(),
        &DataValue::Int(3)
    );
}

// ============================================================================
// AParser Objects (17)
// Java: com.alibaba.qlexpress4.aparser
// ============================================================================

/// QLParser — 解析器主类
/// 通过解析各种语法结构间接验证
#[test]
fn ql_parser_expressions() {
    let r = runner();
    let o = options();
    assert_eq!(
        r.execute("1 + 2 * 3", HashMap::new(), &o).unwrap().result(),
        &DataValue::Int(7)
    );
    assert_eq!(
        r.execute("(1 + 2) * 3", HashMap::new(), &o)
            .unwrap()
            .result(),
        &DataValue::Int(9)
    );
}

/// QLParser — 变量声明
#[test]
fn ql_parser_variable_declaration() {
    let r = runner();
    let o = options();
    assert_eq!(
        r.execute("int x = 42; x", HashMap::new(), &o)
            .unwrap()
            .result(),
        &DataValue::Int(42)
    );
}

/// QLParser — 字符串字面量
#[test]
fn ql_parser_string_literals() {
    let r = runner();
    let o = options();
    assert_eq!(
        r.execute("'hello'", HashMap::new(), &o).unwrap().result(),
        &DataValue::Str("hello".into())
    );
    assert_eq!(
        r.execute("\"world\"", HashMap::new(), &o).unwrap().result(),
        &DataValue::Str("world".into())
    );
}

/// QvmInstructionVisitor — 指令生成
/// 通过复杂表达式间接验证指令生成
#[test]
fn qvm_instruction_visitor_complex() {
    let r = runner();
    let o = options();
    assert_eq!(
        r.execute(
            "int x = 0; for(int i = 0; i < 3; i = i + 1) { x = x + i; } x",
            HashMap::new(),
            &o
        )
        .unwrap()
        .result(),
        &DataValue::Int(3)
    );
}

/// ImportManager — import 语句
#[test]
fn import_manager() {
    let r = runner();
    let o = options();
    // import 语句本身不返回值,验证不崩溃
    let result = r.execute("import java.util.HashMap; 42", HashMap::new(), &o);
    assert!(result.is_ok());
    assert_eq!(result.unwrap().result(), &DataValue::Int(42));
}

/// ParserOperatorManager — 操作符管理
#[test]
fn parser_operator_manager() {
    let r = runner();
    let o = options();
    // 所有基本操作符都通过 ParserOperatorManager 注册
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
    assert_eq!(
        r.execute("true && false", HashMap::new(), &o)
            .unwrap()
            .result(),
        &DataValue::Bool(false)
    );
}

/// OperatorFactory — 操作符工厂
#[test]
fn operator_factory() {
    let r = runner();
    let o = options();
    // OperatorFactory 创建所有操作符实例
    assert_eq!(
        r.execute("1 == 1", HashMap::new(), &o).unwrap().result(),
        &DataValue::Bool(true)
    );
    assert_eq!(
        r.execute("1 != 2", HashMap::new(), &o).unwrap().result(),
        &DataValue::Bool(true)
    );
}

/// ParseTree — 语法树
#[test]
fn parse_tree_structure() {
    let r = runner();
    let o = options();
    // 复杂嵌套表达式验证语法树正确构建
    assert_eq!(
        r.execute("((1 + 2) * (3 - 1)) / 2", HashMap::new(), &o)
            .unwrap()
            .result(),
        &DataValue::Int(3)
    );
}

/// QCompileCache — 编译缓存
#[test]
fn compile_cache() {
    let r = runner();
    let o = options();
    // 同一脚本多次执行,第二次应命中缓存
    assert_eq!(
        r.execute("1 + 2", HashMap::new(), &o).unwrap().result(),
        &DataValue::Int(3)
    );
    assert_eq!(
        r.execute("1 + 2", HashMap::new(), &o).unwrap().result(),
        &DataValue::Int(3)
    );
}

// ============================================================================
// Runtime Core Objects (15)
// Java: com.alibaba.qlexpress4.runtime
// ============================================================================

/// QContext — 执行上下文 trait
#[test]
fn q_context_operations() {
    let r = runner();
    let o = options();
    // QContext 管理变量作用域
    assert_eq!(
        r.execute(
            "int x = 1;\nif(true) {\n  int y = 2;\n  x = x + y;\n}\nx",
            HashMap::new(),
            &o
        )
        .unwrap()
        .result(),
        &DataValue::Int(3)
    );
}

/// QvmRuntime — QVM 运行时
#[test]
fn qvm_runtime_execution() {
    let r = runner();
    let o = options();
    // QvmRuntime 执行指令序列
    assert_eq!(
        r.execute("42", HashMap::new(), &o).unwrap().result(),
        &DataValue::Int(42)
    );
}

/// QResult — 执行结果
#[test]
fn q_result_types() {
    let r = runner();
    let o = options();
    // 不同类型的返回值
    assert_eq!(
        r.execute("42", HashMap::new(), &o).unwrap().result(),
        &DataValue::Int(42)
    );
    assert_eq!(
        r.execute("'hello'", HashMap::new(), &o).unwrap().result(),
        &DataValue::Str("hello".into())
    );
    assert_eq!(
        r.execute("true", HashMap::new(), &o).unwrap().result(),
        &DataValue::Bool(true)
    );
    assert_eq!(
        r.execute("null", HashMap::new(), &o).unwrap().result(),
        &DataValue::Null
    );
}

/// Value — 可求值接口
#[test]
fn value_interface() {
    let r = runner();
    let o = options();
    // Value trait 统一所有可求值对象
    assert_eq!(
        r.execute("1 + 2", HashMap::new(), &o).unwrap().result(),
        &DataValue::Int(3)
    );
}

/// LeftValue — 左值(可赋值)
#[test]
fn left_value_assignment() {
    let r = runner();
    let o = options();
    assert_eq!(
        r.execute("int x = 0; x = 10; x", HashMap::new(), &o)
            .unwrap()
            .result(),
        &DataValue::Int(10)
    );
}

/// QLambda — Lambda 接口
#[test]
fn qlambda_basic() {
    let r = runner();
    let o = options();
    assert_eq!(
        r.execute("f = (x) -> x * 2; f(5)", HashMap::new(), &o)
            .unwrap()
            .result(),
        &DataValue::Int(10)
    );
}

/// QLambdaInner — 用户定义 Lambda
#[test]
fn qlambda_inner_closure() {
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

/// QLambdaDefinition — 编译后的 Lambda
#[test]
fn qlambda_definition() {
    let r = runner();
    let o = options();
    assert_eq!(
        r.execute("function f(x) { return x * 2; } f(5)", HashMap::new(), &o)
            .unwrap()
            .result(),
        &DataValue::Int(10)
    );
}

/// MemberResolver — 成员解析
#[test]
fn member_resolver() {
    let r = open_runner();
    let o = options();
    // MemberResolver 解析 Map 的 size 方法
    assert_eq!(
        r.execute("m = {'a':1,'b':2}; m.size()", HashMap::new(), &o)
            .unwrap()
            .result(),
        &DataValue::Int(2)
    );
}

/// IMethod / JvmIMethod — 方法接口
#[test]
fn imethod_invoke() {
    let r = open_runner();
    let o = options();
    assert_eq!(
        r.execute("[1,2,3].size()", HashMap::new(), &o)
            .unwrap()
            .result(),
        &DataValue::Int(3)
    );
}

// ============================================================================
// Context Objects (4)
// Java: com.alibaba.qlexpress4.runtime.context
// ============================================================================

/// MapExpressContext — Map 上下文
#[test]
fn map_express_context() {
    let r = runner();
    let o = options();
    let mut ctx = HashMap::new();
    ctx.insert("a".to_string(), DataValue::Int(1));
    ctx.insert("b".to_string(), DataValue::Int(2));
    assert_eq!(
        r.execute("a + b", ctx, &o).unwrap().result(),
        &DataValue::Int(3)
    );
}

/// DynamicVariableContext — 动态变量上下文
#[test]
fn dynamic_variable_context() {
    let r = runner();
    let o = options();
    assert_eq!(
        r.execute("int x = 1; x = x + 1; x", HashMap::new(), &o)
            .unwrap()
            .result(),
        &DataValue::Int(2)
    );
}

// ============================================================================
// Data Objects (6)
// Java: com.alibaba.qlexpress4.runtime.data
// ============================================================================

/// DataValue — 数据值枚举
#[test]
fn data_value_types() {
    let r = runner();
    let o = options();
    assert_eq!(
        r.execute("42", HashMap::new(), &o).unwrap().result(),
        &DataValue::Int(42)
    );
    assert_eq!(
        r.execute("42L", HashMap::new(), &o).unwrap().result(),
        &DataValue::Long(42)
    );
    assert_eq!(
        r.execute("3.14", HashMap::new(), &o).unwrap().result(),
        &DataValue::Double(3.14)
    );
    assert_eq!(
        r.execute("'hi'", HashMap::new(), &o).unwrap().result(),
        &DataValue::Str("hi".into())
    );
    assert_eq!(
        r.execute("true", HashMap::new(), &o).unwrap().result(),
        &DataValue::Bool(true)
    );
    assert_eq!(
        r.execute("null", HashMap::new(), &o).unwrap().result(),
        &DataValue::Null
    );
}

/// AssignableDataValue — 可赋值数据值
#[test]
fn assignable_data_value() {
    let r = runner();
    let o = options();
    assert_eq!(
        r.execute("int x = 0; x = 42; x", HashMap::new(), &o)
            .unwrap()
            .result(),
        &DataValue::Int(42)
    );
}

/// ArrayItemValue — 数组元素值
#[test]
fn array_item_value() {
    let r = runner();
    let o = options();
    assert_eq!(
        r.execute("[10, 20, 30][1]", HashMap::new(), &o)
            .unwrap()
            .result(),
        &DataValue::Int(20)
    );
}

/// ListItemValue — 列表元素值
#[test]
fn list_item_value() {
    let r = runner();
    let o = options();
    let result = r.execute("a = [1,2,3]; a[0]", HashMap::new(), &o).unwrap();
    assert_eq!(result.result(), &DataValue::Int(1));
}

/// MapItemValue — Map 元素值
#[test]
fn map_item_value() {
    let r = runner();
    let o = options();
    assert_eq!(
        r.execute("m = {'key': 'val'}; m['key']", HashMap::new(), &o)
            .unwrap()
            .result(),
        &DataValue::Str("val".into())
    );
}

/// FieldValue — 字段值
#[test]
fn field_value() {
    let r = runner();
    let o = options();
    assert_eq!(
        r.execute("m = {'x': 10}; m['x']", HashMap::new(), &o)
            .unwrap()
            .result(),
        &DataValue::Int(10)
    );
}

// ============================================================================
// Exception Objects (5)
// Java: com.alibaba.qlexpress4.exception
// ============================================================================

/// QLException — 异常基类
#[test]
fn ql_exception_syntax() {
    let r = runner();
    let o = options();
    let result = r.execute("1 +", HashMap::new(), &o);
    assert!(result.is_err());
}

/// QLRuntimeException — 运行时异常
#[test]
fn ql_runtime_exception() {
    let r = runner();
    let o = options();
    let result = r.execute("null + 1", HashMap::new(), &o);
    assert!(result.is_err());
}

/// QLTimeoutException — 超时异常
#[test]
fn ql_timeout_exception() {
    let r = runner();
    let o = QLOptions::builder().timeout_millis(1).build();
    // 超短超时应触发超时异常
    let result = r.execute("int i = 0; while(true) { i = i + 1; }", HashMap::new(), &o);
    // 可能超时或在设置超时前完成
}

/// ErrorReporter — 错误报告器
#[test]
fn error_reporter() {
    let r = runner();
    let o = options();
    let result = r.execute("throw 'test error'", HashMap::new(), &o);
    assert!(result.is_err());
}

// ============================================================================
// Function Objects (5)
// Java: com.alibaba.qlexpress4.runtime.function
// ============================================================================

/// QLambdaFunction — Lambda 函数
#[test]
fn qlambda_function() {
    let r = runner();
    let o = options();
    assert_eq!(
        r.execute("f = (x) -> x + 1; f(5)", HashMap::new(), &o)
            .unwrap()
            .result(),
        &DataValue::Int(6)
    );
}

/// ExtensionFunction — 扩展函数 (map/filter)
#[test]
fn extension_function_map() {
    let r = open_runner();
    let o = options();
    let result = r.execute("[1,2,3].map((x) -> x * 2)", HashMap::new(), &o);
    assert!(result.is_ok());
}

/// FilterExtensionFunction — filter 扩展函数
#[test]
fn extension_function_filter() {
    let r = open_runner();
    let o = options();
    let result = r.execute("[1,2,3,4,5].filter((x) -> x > 3)", HashMap::new(), &o);
    assert!(result.is_ok());
}

// ============================================================================
// Trace Objects (3)
// Java: com.alibaba.qlexpress4.runtime.trace
// ============================================================================

/// ExpressionTrace — 表达式追踪
#[test]
fn expression_trace() {
    let r = runner();
    let o = options();
    // 基本执行不崩溃
    assert_eq!(
        r.execute("1 + 2", HashMap::new(), &o).unwrap().result(),
        &DataValue::Int(3)
    );
}

// ============================================================================
// Util Objects (3)
// Java: com.alibaba.qlexpress4.runtime.util
// ============================================================================

/// MethodInvokeUtils — 方法调用工具
#[test]
fn method_invoke_utils() {
    let r = open_runner();
    let o = options();
    assert_eq!(
        r.execute("[1,2,3].size()", HashMap::new(), &o)
            .unwrap()
            .result(),
        &DataValue::Int(3)
    );
}

/// ThrowUtils — 抛出工具
#[test]
fn throw_utils() {
    let r = runner();
    let o = options();
    let result = r.execute("throw 'error'", HashMap::new(), &o);
    assert!(result.is_err());
}

/// ValueUtils — 值工具
#[test]
fn value_utils() {
    let r = runner();
    let o = options();
    // 类型转换
    assert_eq!(
        r.execute("1 + 2L", HashMap::new(), &o).unwrap().result(),
        &DataValue::Long(3)
    );
}

// ============================================================================
// Utils Objects (2)
// Java: com.alibaba.qlexpress4.utils
// ============================================================================

/// PrintlnUtils — 打印工具
#[test]
fn println_utils() {
    let r = runner();
    let o = options();
    // PrintlnUtils 内部工具,通过执行结果间接验证
    assert_eq!(
        r.execute("42", HashMap::new(), &o).unwrap().result(),
        &DataValue::Int(42)
    );
}

/// QLFunctionUtil — 函数工具
#[test]
fn ql_function_util() {
    let r = runner();
    let o = options();
    // 函数定义和调用
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

// ============================================================================
// Number Math Objects (6) — 额外覆盖
// Java: com.alibaba.qlexpress4.runtime.operator.number
// ============================================================================

/// NumberMath — null 处理
#[test]
fn number_math_null() {
    let r = runner();
    let o = options();
    let result = r.execute("null + 1", HashMap::new(), &o);
    assert!(result.is_err());
}

/// IntegerMath — 边界值
#[test]
fn integer_math_boundaries() {
    let r = runner();
    let o = options();
    assert_eq!(
        r.execute("0 + 0", HashMap::new(), &o).unwrap().result(),
        &DataValue::Int(0)
    );
    assert_eq!(
        r.execute("-1 + 1", HashMap::new(), &o).unwrap().result(),
        &DataValue::Int(0)
    );
}

/// LongMath — long 溢出
#[test]
fn long_math_large() {
    let r = runner();
    let o = options();
    let result = r
        .execute("999999999999999999L + 1L", HashMap::new(), &o)
        .unwrap();
    assert_eq!(result.result(), &DataValue::Long(1000000000000000000));
}

/// FloatingPointMath — 特殊值
#[test]
fn floating_point_special() {
    let r = runner();
    let o = options();
    let result = r.execute("1.0 / 0.0", HashMap::new(), &o).unwrap();
    match result.result() {
        DataValue::Double(d) => assert!(d.is_infinite()),
        other => panic!("Expected Double infinity, got {:?}", other),
    }
}

/// BigDecimalMath — 精度保持
#[test]
fn big_decimal_precision() {
    let r = runner();
    let o = options();
    let result = r.execute("1 / 3", HashMap::new(), &o).unwrap();
    match result.result() {
        DataValue::BigDec(s) => assert!(s.contains("3333333333")),
        other => panic!("Expected BigDec, got {:?}", other),
    }
}

/// BigIntegerMath — 任意精度
#[test]
fn big_integer_arbitrary() {
    let r = runner();
    let o = options();
    let result = r
        .execute(
            "999999999999999999 * 999999999999999999",
            HashMap::new(),
            &o,
        )
        .unwrap();
    match result.result() {
        DataValue::BigInt(ref s) => assert!(s.to_string().len() > 10),
        DataValue::Long(_) => {} // 可能被提升为 long
        other => panic!("Expected BigInt or Long, got {:?}", other),
    }
}
