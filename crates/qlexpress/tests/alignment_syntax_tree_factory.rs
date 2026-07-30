//! 逐项对齐 Java `aparser/SyntaxTreeFactoryTest` 的 29 项编译产物契约。

#![allow(clippy::result_large_err)]

use std::rc::Rc;

use qlexpress::aparser::interpolation_mode::InterpolationMode;
use qlexpress::default_class_supplier::DefaultClassSupplier;
use qlexpress::init_options::InitOptions;
use qlexpress::runtime::data::convert::obj_type_convertor::TargetType;
use qlexpress::runtime::instruction::{
    CastInstruction, ConstInstruction, GetFieldInstruction, IndexInstruction, Instruction,
    LoadInstruction, LoadLambdaInstruction, MethodInvokeInstruction, NewArrayInstruction,
    NewFilledInstanceInstruction, OperatorInstruction, StringJoinInstruction,
};
use qlexpress::runtime::member::as_meta_class;
use qlexpress::runtime::qlambda_definition_inner::QLambdaDefinitionInner;
use qlexpress::runtime::value::DataValue;
use qlexpress::runtime::value::QValue;
use qlexpress::Express4Runner;

fn runner(mode: InterpolationMode) -> Express4Runner {
    let mut supplier = DefaultClassSupplier::instance();
    for name in [
        "java.lang.Object",
        "java.lang.Exception",
        "java.lang.NullPointerException",
        "java.lang.Runnable",
        "java.lang.String",
        "java.util.function.Function",
        "com.alibaba.qlexpress4.aparser.ImportManagerTest",
        "com.alibaba.qlexpress4.aparser.ImportManagerTest$TestImportInner",
        "com.alibaba.qlexpress4.aparser.ImportManagerTest$TestImportInner$TestImportInner2",
    ] {
        supplier.register(name);
    }
    let mut runner = Express4Runner::with_init_options(
        InitOptions::builder()
            .class_supplier(Rc::new(supplier))
            .interpolation_mode(mode)
            .build(),
    );
    runner.add_operator_with_precedence(
        ".*",
        Rc::new(|_: &QValue, _: &QValue| Ok(DataValue::Null)),
        qlexpress::ql_precedences::GROUP,
    );
    runner
}

fn compile(script: &str, mode: InterpolationMode) -> Vec<Instruction> {
    runner(mode)
        .parse_to_instructions(script)
        .unwrap_or_else(|error| panic!("compile failed for {script:?}: {error:?}"))
}

fn instruction<T: 'static>(instructions: &[Instruction], index: usize) -> &T {
    instructions[index]
        .as_any()
        .and_then(|value| value.downcast_ref::<T>())
        .unwrap_or_else(|| panic!("unexpected instruction type at index {index}"))
}

fn assert_operator(script: &str, index: usize, expected: &str) {
    let instructions = compile(script, InterpolationMode::Variable);
    assert_eq!(
        instruction::<OperatorInstruction>(&instructions, index)
            .operator()
            .operator(),
        expected
    );
}

#[test]
/// Java `SyntaxTreeFactoryTest#visitPathExprTestWhenMix`。
fn java_visit_path_expr_test_when_mix() {
    let instructions = compile(
        "java.util.function.Function.a.b.cc()",
        InterpolationMode::Variable,
    );
    assert_eq!(instructions.len(), 6);
    let meta =
        as_meta_class(instruction::<ConstInstruction>(&instructions, 0).const_obj()).unwrap();
    assert_eq!(meta.java_name(), "java.util.function.Function");
    assert_eq!(
        instruction::<GetFieldInstruction>(&instructions, 1).field_name(),
        "a"
    );
    assert_eq!(
        instruction::<GetFieldInstruction>(&instructions, 2).field_name(),
        "b"
    );
    assert_eq!(
        instruction::<MethodInvokeInstruction>(&instructions, 3).method_name(),
        "cc"
    );
}

#[test]
/// Java `SyntaxTreeFactoryTest#visitPathExprTestWhenInnerCls`。
fn java_visit_path_expr_test_when_inner_cls() {
    let instructions = compile(
        "com.alibaba.qlexpress4.aparser.ImportManagerTest.TestImportInner.TestImportInner2.pp[m]",
        InterpolationMode::Variable,
    );
    assert_eq!(instructions.len(), 5);
    let meta =
        as_meta_class(instruction::<ConstInstruction>(&instructions, 0).const_obj()).unwrap();
    assert_eq!(
        meta.java_name(),
        "com.alibaba.qlexpress4.aparser.ImportManagerTest$TestImportInner$TestImportInner2"
    );
    assert_eq!(
        instruction::<GetFieldInstruction>(&instructions, 1).field_name(),
        "pp"
    );
    assert_eq!(instruction::<LoadInstruction>(&instructions, 2).name(), "m");
    let _: &IndexInstruction = instruction(&instructions, 3);
}

#[test]
/// Java `SyntaxTreeFactoryTest#visitCallTest`。
fn java_visit_call_test() {
    let instructions = compile("call(mm)", InterpolationMode::Variable);
    assert_eq!(instructions.len(), 4);
    assert_eq!(
        instruction::<LoadInstruction>(&instructions, 0).name(),
        "mm"
    );
}

#[test]
/// Java `SyntaxTreeFactoryTest#numberTest`。
fn java_number_test() {
    let instructions = compile("10_0_0l", InterpolationMode::Variable);
    assert_eq!(instructions.len(), 2);
    assert_eq!(
        instruction::<ConstInstruction>(&instructions, 0).const_obj(),
        &DataValue::Long(1000)
    );
}

#[test]
/// Java `SyntaxTreeFactoryTest#macroDefineTest`。
fn java_macro_define_test() {
    assert_eq!(
        compile(
            "macro add {a+b} add;int c = 10;",
            InterpolationMode::Variable
        )
        .len(),
        7
    );
}

#[test]
/// Java `SyntaxTreeFactoryTest#stringEscapeTest`。
fn java_string_escape_test() {
    let instructions = compile("\"\\r\\n\\p\"", InterpolationMode::Variable);
    assert_eq!(
        instruction::<ConstInstruction>(&instructions, 0).const_obj(),
        &DataValue::Str("\r\n".to_string())
    );
}

#[test]
/// Java `SyntaxTreeFactoryTest#castTest`。
fn java_cast_test() {
    let instructions = compile("1+(int)3L", InterpolationMode::Variable);
    let _: &CastInstruction = instruction(&instructions, 3);
}

#[test]
/// Java `SyntaxTreeFactoryTest#cusOpTest`。
fn java_custom_operator_test() {
    compile("c.*d", InterpolationMode::Variable);
    compile("c>>>d", InterpolationMode::Variable);
}

#[test]
/// Java `SyntaxTreeFactoryTest#pathPartTest`。
fn java_path_part_test() {
    let instructions = compile(
        "assert((java.lang.Object) a == 1)",
        InterpolationMode::Variable,
    );
    let meta =
        as_meta_class(instruction::<ConstInstruction>(&instructions, 0).const_obj()).unwrap();
    assert_eq!(meta.java_name(), "java.lang.Object");
    assert_eq!(instruction::<LoadInstruction>(&instructions, 1).name(), "a");
    let _: &CastInstruction = instruction(&instructions, 2);
}

#[test]
/// Java `SyntaxTreeFactoryTest#fieldExpressionTest`。
fn java_field_expression_test() {
    let instructions = compile("\"null\".equals(b)", InterpolationMode::Variable);
    let _: &ConstInstruction = instruction(&instructions, 0);
}

#[test]
/// Java `SyntaxTreeFactoryTest#lambdaExprBodyTest`。
fn java_lambda_expr_body_test() {
    let instructions = compile(
        "f = e -> try {\n  throw e;\n} catch (java.lang.NullPointerException n) {\n  100\n} catch (java.lang.Exception e) {\n  10\n};",
        InterpolationMode::Variable,
    );
    let definition = instruction::<LoadLambdaInstruction>(&instructions, 1).lambda_definition();
    let inner = definition
        .as_any()
        .and_then(|value| value.downcast_ref::<QLambdaDefinitionInner>())
        .unwrap();
    assert_eq!(inner.instructions().len(), 2);
}

#[test]
/// Java `SyntaxTreeFactoryTest#lambdaBlockBodyTest`。
fn java_lambda_block_body_test() {
    let instructions = compile("f = e -> {10};", InterpolationMode::Variable);
    let definition = instruction::<LoadLambdaInstruction>(&instructions, 1).lambda_definition();
    let inner = definition
        .as_any()
        .and_then(|value| value.downcast_ref::<QLambdaDefinitionInner>())
        .unwrap();
    assert_eq!(inner.instructions().len(), 2);
    let _: &ConstInstruction = instruction(inner.instructions(), 0);
}

#[test]
/// Java `SyntaxTreeFactoryTest#lambdaMapBodyTest`。
fn java_lambda_map_body_test() {
    let instructions = compile("f = e -> {'test': 1234};", InterpolationMode::Variable);
    let definition = instruction::<LoadLambdaInstruction>(&instructions, 1).lambda_definition();
    let inner = definition
        .as_any()
        .and_then(|value| value.downcast_ref::<QLambdaDefinitionInner>())
        .unwrap();
    assert_eq!(inner.instructions().len(), 3);
}

#[test]
/// Java `SyntaxTreeFactoryTest#newArrayTest`。
fn java_new_array_test() {
    let instructions = compile(
        "new int[][] {new int[] {1,2}, new int[] {3,4}}",
        InterpolationMode::Variable,
    );
    let arrays = instructions
        .iter()
        .filter_map(|item| {
            item.as_any()
                .and_then(|value| value.downcast_ref::<NewArrayInstruction>())
        })
        .collect::<Vec<_>>();
    assert_eq!(arrays.len(), 3);
    assert_eq!(
        arrays
            .iter()
            .map(|array| array.clz().clone())
            .collect::<Vec<_>>(),
        vec![
            ClassRef::Boxed(TargetType::Int),
            ClassRef::Boxed(TargetType::Int),
            ClassRef::from_name("java.lang.Integer[]"),
        ]
    );
    assert_eq!(arrays.last().unwrap().length(), 2);
}

fn assert_instanceof_meta(script: &str, expected: &str) {
    let instructions = compile(script, InterpolationMode::Variable);
    let meta =
        as_meta_class(instruction::<ConstInstruction>(&instructions, 1).const_obj()).unwrap();
    assert_eq!(meta.java_name(), expected);
}

#[test]
/// Java `SyntaxTreeFactoryTest#instanceOfTest`。
fn java_instance_of_test() {
    assert_instanceof_meta("1 instanceof int", "java.lang.Integer");
}

#[test]
/// Java `SyntaxTreeFactoryTest#instanceOfStrArrTest`。
fn java_instance_of_string_array_test() {
    assert_instanceof_meta(
        "1 instanceof java.lang.String[][][]",
        "[[[Ljava.lang.String;",
    );
}

#[test]
/// Java `SyntaxTreeFactoryTest#instanceOfIntArrTest`。
fn java_instance_of_int_array_test() {
    assert_instanceof_meta("1 instanceof int[][][]", "[[[Ljava.lang.Integer;");
}

#[test]
/// Java `SyntaxTreeFactoryTest#bitOperatorTest`。
fn java_bit_operator_test() {
    assert_operator("true & true", 2, "&");
    assert_operator("true | true", 2, "|");
    assert_operator("2 % 3", 2, "%");
}

#[test]
/// Java `SyntaxTreeFactoryTest#opPrecedencesTest`。
fn java_operator_precedences_test() {
    let instructions = compile("a = 1+2*3+10", InterpolationMode::Variable);
    assert_eq!(
        instruction::<OperatorInstruction>(&instructions, 4)
            .operator()
            .operator(),
        "*"
    );
    assert_eq!(
        instruction::<OperatorInstruction>(&instructions, 9)
            .operator()
            .operator(),
        "="
    );
}

#[test]
/// Java `SyntaxTreeFactoryTest#ternaryTest`。
fn java_ternary_test() {
    let instructions = compile("l = (x) -> x > 10 ? 11 : 100", InterpolationMode::Variable);
    assert_eq!(instruction::<LoadInstruction>(&instructions, 0).name(), "l");
    let _: &LoadLambdaInstruction = instruction(&instructions, 1);
    assert_eq!(
        instruction::<OperatorInstruction>(&instructions, 2)
            .operator()
            .operator(),
        "="
    );
}

#[test]
/// Java `SyntaxTreeFactoryTest#functionInterfaceTest`。
fn java_function_interface_test() {
    compile(
        "java.lang.Runnable r = () -> a = 8;",
        InterpolationMode::Variable,
    );
}

#[test]
/// Java `SyntaxTreeFactoryTest#groupPriorityTest`。
fn java_group_priority_test() {
    compile("a.*b.*c[2]+d.*e[1:2]", InterpolationMode::Variable);
}

#[test]
/// Java `SyntaxTreeFactoryTest#numberAmbiguousValueTest`。
fn java_number_ambiguous_value_test() {
    let instructions = compile("1.doubleValue()", InterpolationMode::Variable);
    assert_eq!(
        instruction::<MethodInvokeInstruction>(&instructions, 1).method_name(),
        "doubleValue"
    );
}

#[test]
/// Java `SyntaxTreeFactoryTest#classifiedJsonTest`。
fn java_classified_json_test() {
    let instructions = compile(
        "{'@class':'java.lang.Object', 'a': 'cccc'}",
        InterpolationMode::Variable,
    );
    let _: &NewFilledInstanceInstruction = instruction(&instructions, 1);
}

#[test]
/// Java `SyntaxTreeFactoryTest#selectorTest`。
fn java_selector_test() {
    let instructions = compile("${ TextField-AXXE } + ${v231}", InterpolationMode::Script);
    assert_eq!(
        instruction::<LoadInstruction>(&instructions, 0).name(),
        "TextField-AXXE"
    );
    assert_eq!(
        instruction::<LoadInstruction>(&instructions, 1).name(),
        "v231"
    );
    let instructions = compile("${ TextField-A} + ${v2}", InterpolationMode::Variable);
    assert_eq!(
        instruction::<LoadInstruction>(&instructions, 0).name(),
        "TextField-A"
    );
    assert_eq!(
        instruction::<LoadInstruction>(&instructions, 1).name(),
        "v2"
    );
}

#[test]
/// Java `SyntaxTreeFactoryTest#doubleQuoteStringScriptTest`。
fn java_double_quote_string_script_test() {
    let instructions = compile("\"a ${v-1}\"", InterpolationMode::Script);
    assert_eq!(
        instruction::<ConstInstruction>(&instructions, 0).const_obj(),
        &DataValue::Str("a ".to_string())
    );
    assert_eq!(instruction::<LoadInstruction>(&instructions, 1).name(), "v");
    assert_eq!(
        instruction::<ConstInstruction>(&instructions, 2).const_obj(),
        &DataValue::Int(1)
    );
    assert_eq!(
        instruction::<OperatorInstruction>(&instructions, 3)
            .operator()
            .operator(),
        "-"
    );
    assert_eq!(
        instruction::<StringJoinInstruction>(&instructions, 4).n(),
        2
    );
}

#[test]
/// Java `SyntaxTreeFactoryTest#doubleQuoteStringScriptTest2`。
fn java_double_quote_string_script_test2() {
    compile("\"Hello ${a} ccc\"", InterpolationMode::Script);
}

#[test]
/// Java `SyntaxTreeFactoryTest#doubleQuoteStringVariableTest`。
fn java_double_quote_string_variable_test() {
    let instructions = compile("\"a ${ v-1 } b\"", InterpolationMode::Variable);
    assert_eq!(
        instruction::<ConstInstruction>(&instructions, 0).const_obj(),
        &DataValue::Str("a ".to_string())
    );
    assert_eq!(
        instruction::<LoadInstruction>(&instructions, 1).name(),
        "v-1"
    );
    assert_eq!(
        instruction::<ConstInstruction>(&instructions, 2).const_obj(),
        &DataValue::Str(" b".to_string())
    );
    assert_eq!(
        instruction::<StringJoinInstruction>(&instructions, 3).n(),
        3
    );
}

#[test]
/// Java `SyntaxTreeFactoryTest#ifTest`。
fn java_if_test() {
    let instructions = compile(
        "if (a > 0 && a < 5) {\n  true\n} else if (a > 5 && a < 10) {\n  false\n} else if (a > 10 && a < 15) {\n  true\n} == true",
        InterpolationMode::Variable,
    );
    assert_eq!(
        instruction::<OperatorInstruction>(&instructions, instructions.len() - 2,)
            .operator()
            .operator(),
        "=="
    );
}
