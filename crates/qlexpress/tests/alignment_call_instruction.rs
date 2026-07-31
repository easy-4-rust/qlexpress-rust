//! Java `CallInstructionTest#case1` 的指令级对齐测试。
//!
//! Java 用 `GetMethodInstruction` 取得绑定方法 Lambda，再由
//! `CallInstruction` 消费 Lambda 和参数。Rust 采用相同栈协议，并额外
//! 验证 Java `avoidNullPointer` 与精确错误码分支。

#![allow(clippy::result_large_err)]

use std::collections::HashMap;
use std::rc::Rc;

use qlexpress::exception::error_codes;
use qlexpress::exception::error_reporter::ErrorReporter;
use qlexpress::exception::pure_err_reporter::PureErrReporter;
use qlexpress::ql_options::QLOptions;
use qlexpress::runtime::data::lambda::QLambdaMethod;
use qlexpress::runtime::delegate_qcontext::DelegateQContext;
use qlexpress::runtime::instruction::{
    CallConstInstruction, CallInstruction, GetMethodInstruction, QLInstruction,
};
use qlexpress::runtime::member::NativeRegistry;
use qlexpress::runtime::native_type::NativeType;
use qlexpress::runtime::qcontext::QContext;
use qlexpress::runtime::qlambda::QLambda;
use qlexpress::runtime::qvm_global_scope::QvmGlobalScope;
use qlexpress::runtime::qvm_runtime::QvmRuntime;
use qlexpress::runtime::scope::QScope;
use qlexpress::runtime::value::{DataValue, QValue};

fn reporter() -> Rc<dyn ErrorReporter> {
    Rc::new(PureErrReporter::INSTANCE)
}

fn context(registry: Rc<NativeRegistry>) -> DelegateQContext {
    let runtime = Rc::new(QvmRuntime::for_test(registry));
    let global = QScope::global(QvmGlobalScope::empty());
    let block = QScope::block_fresh_stack(&global, HashMap::new(), 8);
    DelegateQContext::new(runtime, block)
}

fn substring_lambda(registry: Rc<NativeRegistry>) -> Rc<QLambda> {
    Rc::new(QLambda::Method(QLambdaMethod::new(
        "substring",
        registry,
        DataValue::Str("qlexpress".into()),
    )))
}

#[test]
/// Java `CallInstructionTest#case1`。
fn java_case1_get_method_then_call_with_two_arguments() {
    let registry = NativeRegistry::with_builtins();
    let mut child = NativeType::named("java.lang.Integer");
    child.methods.insert(
        "getMethod1".to_string(),
        Rc::new(|_, arguments| match arguments {
            [DataValue::Int(left), DataValue::Int(right)] => Ok(DataValue::Int(left + right)),
            _ => Ok(DataValue::Null),
        }),
    );
    registry.register_type(child);
    let registry = Rc::new(registry);
    let mut context = context(Rc::clone(&registry));
    context.push(QValue::Data(DataValue::Int(0)));

    let get_method = GetMethodInstruction::new(reporter(), "getMethod1");
    get_method
        .execute(&mut context, &QLOptions::builder().build())
        .expect("getMethod1 must produce a bound lambda");
    assert!(matches!(context.peek().get(), DataValue::Lambda(_)));

    context.push(QValue::Data(DataValue::Int(1)));
    context.push(QValue::Data(DataValue::Int(2)));

    let instruction = CallInstruction::new(reporter(), 2);
    let result = instruction
        .execute(&mut context, &QLOptions::builder().build())
        .expect("bound method call");

    assert!(result.is_next_instruction());
    assert_eq!(context.pop().get(), DataValue::Int(3));
    assert_eq!(instruction.arg_num(), 2);
    assert_eq!(instruction.stack_input(), 3);
    assert_eq!(instruction.stack_output(), 1);
}

#[test]
fn call_instruction_preserves_null_and_not_callable_contracts() {
    let registry = Rc::new(NativeRegistry::with_builtins());
    let mut nullable = context(Rc::clone(&registry));
    nullable.push(QValue::Data(DataValue::Null));
    let instruction = CallInstruction::new(reporter(), 0);
    instruction
        .execute(
            &mut nullable,
            &QLOptions::builder().avoid_null_pointer(true).build(),
        )
        .expect("avoid-null call returns null");
    assert_eq!(nullable.pop().get(), DataValue::Null);

    let mut strict = context(registry);
    strict.push(QValue::Data(DataValue::Int(1)));
    let error = instruction
        .execute(&mut strict, &QLOptions::builder().build())
        .expect_err("integer is not callable");
    assert_eq!(error.error_code(), error_codes::OBJECT_NOT_CALLABLE);
}

#[test]
fn call_const_instruction_uses_same_argument_order_and_result_contract() {
    let registry = Rc::new(NativeRegistry::with_builtins());
    let lambda = substring_lambda(Rc::clone(&registry));
    let mut context = context(registry);
    context.push(QValue::Data(DataValue::Int(0)));
    context.push(QValue::Data(DataValue::Int(2)));

    let instruction = CallConstInstruction::new(reporter(), lambda, 2, "substring");
    instruction
        .execute(&mut context, &QLOptions::builder().build())
        .expect("const lambda call");

    assert_eq!(context.pop().get(), DataValue::Str("ql".into()));
    assert_eq!(instruction.arg_num(), 2);
    assert_eq!(instruction.lambda_name(), "substring");
    assert_eq!(instruction.stack_input(), 2);
    assert_eq!(instruction.stack_output(), 1);
}
