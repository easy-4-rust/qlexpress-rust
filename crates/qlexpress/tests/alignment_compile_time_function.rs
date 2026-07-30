//! 逐项对齐 Java `aparser/CompileTimeFunctionTest` 的两个测试。

#![allow(clippy::result_large_err)]

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use qlexpress::aparser::compile_time_function::{CodeGenerator, CompileTimeFunction};
use qlexpress::aparser::operator_factory::OperatorFactory;
use qlexpress::aparser::syntax_tree_factory::Node;
use qlexpress::exception::error_reporter::ErrorReporter;
use qlexpress::exception::QLException;
use qlexpress::init_options::InitOptions;
use qlexpress::ql_options::QLOptions;
use qlexpress::runtime::instruction::{
    Instruction, LoadLambdaInstruction, NewScopeInstruction, QLInstruction,
};
use qlexpress::runtime::q_result::QResult;
use qlexpress::runtime::qcontext::QContext;
use qlexpress::runtime::qlambda_definition_inner::{Param, QLambdaDefinitionInner};
use qlexpress::runtime::value::{DataValue, QValue};
use qlexpress::security::ql_security_strategy::QLSecurityStrategy;
use qlexpress::Express4Runner;

struct ForEachFunction;

impl CompileTimeFunction for ForEachFunction {
    fn create_function_instruction(
        &self,
        _function_name: &str,
        arguments: &[&Node],
        _operator_factory: &dyn OperatorFactory,
        code_generator: &mut dyn CodeGenerator,
    ) {
        if arguments.len() != 2 {
            let _ = code_generator.report_parse_err(
                "INVALID_ARGUMENTS",
                &format!("FOREACH must hava 2 params, but accept {}", arguments.len()),
            );
            return;
        }
        let reporter = code_generator.error_reporter();
        code_generator.add_instruction(Box::new(NewScopeInstruction::new(
            Rc::clone(&reporter),
            "FOR_EACH_FUNCTION",
        )));
        code_generator.add_instructions_by_tree(arguments[0]);
        let definition =
            code_generator.generate_lambda_definition(arguments[1], vec![Param::new("_", None)]);
        code_generator.add_instruction(Box::new(LoadLambdaInstruction::new(
            Rc::clone(&reporter),
            definition,
        )));
        code_generator.add_instruction(Box::new(ForEachInstruction { reporter }));
    }
}

struct ForEachInstruction {
    reporter: Rc<dyn ErrorReporter>,
}

impl QLInstruction for ForEachInstruction {
    fn execute(
        &self,
        q_context: &mut dyn QContext,
        _ql_options: &QLOptions,
    ) -> Result<QResult, QLException> {
        let parameters = q_context.pop_n(2);
        let DataValue::List(items) = parameters.get_value(0) else {
            return Err(self
                .reporter
                .report("INVALID_ARGUMENT", "FOREACH first argument must be a list"));
        };
        let DataValue::Lambda(lambda) = parameters.get_value(1) else {
            return Err(self
                .reporter
                .report("INVALID_ARGUMENT", "FOREACH body must be callable"));
        };
        let mut values = Vec::with_capacity(items.borrow().len());
        for item in items.borrow().iter().cloned() {
            values.push(lambda.call(&[item])?.value());
        }
        q_context.push(QValue::Data(DataValue::list(values)));
        Ok(QResult::NEXT_INSTRUCTION)
    }

    fn stack_input(&self) -> i32 {
        2
    }

    fn stack_output(&self) -> i32 {
        1
    }

    fn println(&self, index: usize, depth: usize, debug: &mut dyn FnMut(String)) {
        debug(format!("{depth}:{index}: FOR_EACH"));
    }

    fn error_reporter(&self) -> &Rc<dyn ErrorReporter> {
        &self.reporter
    }
}

/// Java `CompileTimeFunctionTest#forEachCompileFunctionTest`。
#[test]
fn java_for_each_compile_function_test() {
    let runner = Express4Runner::with_init_options(
        InitOptions::builder()
            .security_strategy(QLSecurityStrategy::open())
            .build(),
    );
    assert!(runner.add_compile_time_function("FOR_EACH", Rc::new(ForEachFunction)));
    assert!(runner.get_compile_time_function("FOR_EACH").is_some());
    let result = runner
        .execute(
            "FOR_EACH([1,2,3,4], _+1)",
            HashMap::new(),
            &QLOptions::builder().build(),
        )
        .expect("compile-time foreach must execute")
        .into_result();
    let DataValue::List(values) = result else {
        panic!("FOREACH must return a list");
    };
    assert_eq!(
        values.borrow().as_slice(),
        &[
            DataValue::Int(2),
            DataValue::Int(3),
            DataValue::Int(4),
            DataValue::Int(5)
        ]
    );
}

struct GenInstructionNumFunction;

impl CompileTimeFunction for GenInstructionNumFunction {
    fn create_function_instruction(
        &self,
        _function_name: &str,
        arguments: &[&Node],
        _operator_factory: &dyn OperatorFactory,
        code_generator: &mut dyn CodeGenerator,
    ) {
        let definition = code_generator.generate_lambda_definition(arguments[0], Vec::new());
        let inner = definition
            .as_any()
            .and_then(|value| value.downcast_ref::<QLambdaDefinitionInner>())
            .expect("generated definition must be an inner lambda");
        assert_eq!(inner.instructions().len(), 2);
    }
}

/// Java `CompileTimeFunctionTest#genInstructionNumTest`。
#[test]
fn java_gen_instruction_num_test() {
    let runner = Express4Runner::with_init_options(
        InitOptions::builder()
            .security_strategy(QLSecurityStrategy::open())
            .build(),
    );
    assert!(runner.add_compile_time_function("GEN_INST_NUM", Rc::new(GenInstructionNumFunction)));
    let instructions: Vec<Instruction> = runner
        .parse_to_instructions("GEN_INST_NUM(1)")
        .expect("compile must invoke instruction counter");
    // 与 Java `parseToLambda` 一致：外层主定义仍会追加 return 指令；
    // 被测断言发生在编译期函数生成的内部 lambda 中。
    assert_eq!(instructions.len(), 1);
}
