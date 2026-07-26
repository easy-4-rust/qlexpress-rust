//! Stage 6 对齐测试:out var / out function 静态分析用例。
//!
//! 对应 Java: com.alibaba.qlexpress4.Express4RunnerTest 的
//! `getOutVarNamesTest` / `getOutFunctions`。

// 与 lib 一致的架构性豁免:QLException 对齐 Java 单一异常类(见 lib.rs)。
#![allow(clippy::result_large_err)]

use std::collections::HashSet;

use qlexpress_rust::Express4Runner;

fn out_vars(script: &str) -> HashSet<String> {
    Express4Runner::new()
        .get_out_var_names(script)
        .unwrap_or_else(|err| panic!("get_out_var_names failed for {script:?}: {err:?}"))
}

fn set(items: &[&str]) -> HashSet<String> {
    items.iter().map(|s| s.to_string()).collect()
}

/// 对应 Java `Express4RunnerTest#getOutVarNamesTest`(声明与赋值的
/// 出参推断)。
#[test]
fn out_var_names_basic() {
    assert_eq!(
        out_vars("int a = 1, b = 10;\nc = 11\ne = a + b + c + d\nf+e"),
        set(&["d", "f"])
    );
    assert_eq!(out_vars("if (true) {a = 10} else {a}"), set(&["a"]));
    assert_eq!(out_vars("while (a>2) {a++;b=100} a+b"), set(&["a"]));
    assert_eq!(out_vars("resultSet = ''; if (a == 11)true"), set(&["a"]));
}

/// 对应 Java `Express4RunnerTest#getOutVarNamesTest`(自赋值与类型化
/// 声明分支)。
#[test]
fn out_var_names_self_assign() {
    assert_eq!(out_vars("a=a+1"), set(&["a"]));
    assert_eq!(out_vars("a+=1"), set(&["a"]));
    assert_eq!(out_vars("a=1;a=a+1"), set(&[]));
    assert_eq!(out_vars("a=1;a+=1"), set(&[]));
    assert_eq!(out_vars("int a=a+1;"), set(&["a"]));
    assert_eq!(out_vars("int a=1;a=a+1"), set(&[]));
}

/// 对应 Java `Express4RunnerTest#getOutVarNamesTest`(选择器与动态
/// 字符串分支)。
#[test]
fn out_var_names_selector_and_dynamic_string() {
    assert_eq!(out_vars("${0} + ${1}"), set(&["0", "1"]));
    assert_eq!(out_vars("\"Hello ${a+b}\""), set(&["a", "b"]));
}

/// 对应 Java `Express4RunnerTest#getOutVarNamesTest`(函数参数不计入
/// out var)。
#[test]
fn out_var_names_exclude_function_params() {
    let script = "function sub(a, b) {\n    return a-b;\n}\nreturn sub(x, y);";
    assert_eq!(out_vars(script), set(&["x", "y"]));
}

/// 对应 Java `Express4RunnerTest#getOutVarNamesTest`(switch 分支)。
#[test]
fn out_var_names_switch() {
    let script = "int globalVar = 1;\nint x = 2;\nswitch (x) {\n  case 1:\n    int localVar = 10;\n    globalVar = localVar + externalVar;\n    break;\n  case 2:\n    int y = externalVar2 + 10;\n    break;\n}\nreturn globalVar;";
    assert_eq!(out_vars(script), set(&["externalVar", "externalVar2"]));
}

/// 对应 Java `Express4RunnerTest#getOutFunctions`(脚本内函数调用收集)。
#[test]
fn out_functions() {
    let runner = Express4Runner::new();
    let names = runner
        .get_out_function_names("cc(a,bc(2,m,1))\ndd(c)")
        .unwrap();
    assert_eq!(names, set(&["cc", "bc", "dd"]));
}
