//! Stage 6 对齐测试:out var / out function 静态分析用例。
//!
//! 对应 Java: com.alibaba.qlexpress4.Express4RunnerTest 的
//! `getOutVarNamesTest` / `getOutFunctions`。

// 与 lib 一致的架构性豁免:QLException 对齐 Java 单一异常类(见 lib.rs)。
#![allow(clippy::result_large_err)]

use std::collections::HashSet;
use std::rc::Rc;

use qlexpress::default_class_supplier::DefaultClassSupplier;
use qlexpress::init_options::InitOptions;
use qlexpress::Express4Runner;

fn out_vars(script: &str) -> HashSet<String> {
    Express4Runner::new()
        .get_out_var_names(script)
        .unwrap_or_else(|err| panic!("get_out_var_names failed for {script:?}: {err:?}"))
}

fn set(items: &[&str]) -> HashSet<String> {
    items.iter().map(|s| s.to_string()).collect()
}

fn out_attrs(script: &str) -> Vec<String> {
    let mut supplier = DefaultClassSupplier::instance();
    supplier.register("java.lang.Math");
    let runner = Express4Runner::with_init_options(
        InitOptions::builder()
            .class_supplier(Rc::new(supplier))
            .build(),
    );
    let mut attrs = runner
        .get_out_var_attrs(script)
        .unwrap_or_else(|err| panic!("get_out_var_attrs failed for {script:?}: {err:?}"))
        .into_iter()
        .map(|parts| parts.join("."))
        .collect::<Vec<_>>();
    attrs.sort();
    attrs
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

    assert_eq!(
        runner
            .get_out_function_names("time('2025-09-8')+sum(1,sub(3,2))")
            .unwrap(),
        set(&["time", "sum", "sub"])
    );
    assert_eq!(
        runner
            .get_out_function_names("function add(a,b) {a+b}\n add(1,2)+sub(3,1)")
            .unwrap(),
        set(&["sub"])
    );
    assert!(runner
        .get_out_function_names(
            "function add(a,b) {\n function sub(a,b) { a-b }\n add(1,2)+sub(3,1) \n}\n",
        )
        .unwrap()
        .is_empty());
    assert_eq!(
        runner
            .get_out_function_names(
                "function add(a,b) {\n function sub(a,b) { a-b }\n add(1,2)+sub(3,1) \n}\nsub(3,1)",
            )
            .unwrap(),
        set(&["sub"])
    );
    assert!(runner
        .get_out_function_names("function recur(a,b) {\n recur(1,2) \n}\nrecur(3,1)")
        .unwrap()
        .is_empty());
    assert!(runner
        .get_out_function_names("add(1,2); function add(a,b) {\n a+b \n}")
        .unwrap()
        .is_empty());
}

/// 完整移植 Java `Express4RunnerTest#getOutVarAttrsTest`，覆盖属性链去重、
/// 赋值左值、已声明变量、类限定名、函数作用域和两种 switch 分支。
#[test]
fn out_var_attrs_matches_java_contract_matrix() {
    assert_eq!(
        out_attrs("a.b.c+a.b.c-a.b.d*c.m"),
        ["a.b.c", "a.b.d", "c.m"]
    );
    assert_eq!(out_attrs("a=2;test(a.b.c,c.m)"), ["c.m"]);
    assert_eq!(out_attrs("a.b=2;test(c.m)"), ["a.b", "c.m"]);
    assert_eq!(out_attrs("java.lang.Math.abs(c)"), ["c"]);
    assert!(out_attrs("hello()").is_empty());
    assert_eq!(out_attrs("a=a+1"), ["a"]);
    assert_eq!(out_attrs("a+=1"), ["a"]);
    assert!(out_attrs("a=1;a=a+1").is_empty());
    assert!(out_attrs("a=1;a+=1").is_empty());
    assert_eq!(out_attrs("int a=a+1;"), ["a"]);
    assert!(out_attrs("int a=1;a=a+1").is_empty());

    assert_eq!(
        out_attrs(
            "function sub(a, b) {\n    return a.field - b.name;\n}\nreturn sub(x.prop, y.value);",
        ),
        ["x.prop", "y.value"]
    );
    assert_eq!(
        out_attrs(
            "int globalVar = 1;\nint x = 2;\nswitch (x) {\n\
             case 1:\nint localVar = 10;\nglobalVar = localVar + e0.externalVar;\nbreak;\n\
             case 2:\nint y = e1.externalVar2 + 10;\nbreak;\n}\nreturn globalVar;",
        ),
        ["e0.externalVar", "e1.externalVar2"]
    );
    assert_eq!(
        out_attrs(
            "int x = 2;\nresult = switch (x) {\n\
             case 1 -> e0.prop1 + 10\n\
             case 2 -> e1.prop2 * 2\n\
             default -> e2.prop3\n}",
        ),
        ["e0.prop1", "e1.prop2", "e2.prop3"]
    );
}
