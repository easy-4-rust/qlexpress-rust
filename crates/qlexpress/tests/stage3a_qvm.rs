//! Stage 3a integration tests: hand-written instruction sequences executed
//! by the QVM (`QvmRuntime`/`run_instructions`), mirroring how Java tests
//! drive `QLInstruction[]` through a `QContext`.

#![allow(clippy::result_large_err)]

use std::rc::Rc;

use qlexpress::exception::error_codes;
use qlexpress::exception::error_reporter::ErrorReporter;
use qlexpress::exception::pure_err_reporter::PureErrReporter;
use qlexpress::exception::QLException;
use qlexpress::ql_options::QLOptions;
use qlexpress::runtime::data::convert::{math_domain, promote, MathDomain};
use qlexpress::runtime::delegate_qcontext::DelegateQContext;
use qlexpress::runtime::instruction::ReturnResultType;
use qlexpress::runtime::instruction::{
    BreakContinueInstruction, CallFunctionInstruction, CheckTimeOutInstruction, ConstInstruction,
    DefineFunctionInstruction, DefineLocalInstruction, ForInstruction, GetFieldInstruction,
    IndexInstruction, Instruction, JumpIfPopInstruction, JumpInstruction, LoadInstruction,
    MethodInvokeInstruction, NewListInstruction, OperatorInstruction, PopInstruction,
    QLInstruction, ReturnInstruction, WhileInstruction,
};
use qlexpress::runtime::member::NativeRegistry;
use qlexpress::runtime::operator::base::BinaryOperator;
use qlexpress::runtime::q_result::QResult;
use qlexpress::runtime::qcontext::QContext;
use qlexpress::runtime::qlambda_definition::QLambdaDefinition;
use qlexpress::runtime::qlambda_definition_inner::{Param, QLambdaDefinitionInner};
use qlexpress::runtime::qvm_global_scope::QvmGlobalScope;
use qlexpress::runtime::qvm_runtime::{current_time_millis, run_instructions, QvmRuntime};
use qlexpress::runtime::scope::QScope;
use qlexpress::runtime::value::{DataValue, QValue};

// ---- helpers -------------------------------------------------------------

fn reporter() -> Rc<dyn ErrorReporter> {
    Rc::new(PureErrReporter::INSTANCE)
}

fn runtime() -> Rc<QvmRuntime> {
    Rc::new(QvmRuntime::for_test(Rc::new(
        NativeRegistry::with_builtins(),
    )))
}

fn opts() -> QLOptions {
    QLOptions::builder().build()
}

/// Run instructions as a top-level script (Java: root lambda with
/// `newEnv=true` over the global scope).
fn run_top(
    runtime: &Rc<QvmRuntime>,
    instructions: Vec<Instruction>,
) -> Result<QResult, QLException> {
    let root: Rc<dyn QLambdaDefinition> = Rc::new(QLambdaDefinitionInner::new(
        "root",
        instructions,
        vec![],
        16,
    ));
    runtime.execute(QvmGlobalScope::empty(), root, &opts())
}

/// Run instructions against a manually built context and return it (for
/// stack/scope inspection).
fn run_with_ctx(
    runtime: &Rc<QvmRuntime>,
    instructions: &[Instruction],
) -> Result<DelegateQContext, QLException> {
    let global_scope = QScope::global(QvmGlobalScope::empty());
    let instruction_scope =
        QScope::block_fresh_stack(&global_scope, Default::default(), instructions.len() * 2);
    let mut ctx = DelegateQContext::new(Rc::clone(runtime), instruction_scope);
    run_instructions(&mut ctx, instructions, &opts())?;
    Ok(ctx)
}

fn def(
    name: &str,
    instructions: Vec<Instruction>,
    params: Vec<Param>,
) -> Rc<dyn QLambdaDefinition> {
    Rc::new(QLambdaDefinitionInner::new(name, instructions, params, 16))
}

fn ret() -> Instruction {
    Box::new(ReturnInstruction::new(
        reporter(),
        ReturnResultType::Return,
        None,
    ))
}

fn konst(v: DataValue) -> Instruction {
    Box::new(ConstInstruction::new(reporter(), v, None))
}

fn load(name: &str) -> Instruction {
    Box::new(LoadInstruction::new(reporter(), name, None))
}

// ---- test binary operators (Stage 4 delivers the real ones) --------------

enum NumOp {
    Add,
    Sub,
    Mul,
    Lt,
    Le,
}

impl BinaryOperator for NumOp {
    fn execute(
        &self,
        left: &QValue,
        right: &QValue,
        _ctx: &mut dyn QContext,
        _opts: &QLOptions,
        reporter: &dyn ErrorReporter,
    ) -> Result<DataValue, QLException> {
        let l = left.get();
        let r = right.get();
        if !l.is_number() || !r.is_number() {
            return Err(reporter.report(error_codes::INVALID_BINARY_OPERAND, "not numbers"));
        }
        let domain = math_domain(&l, &r).unwrap_or(MathDomain::Long);
        let (lp, rp) = promote(&l, &r, domain);
        let ordering = qlexpress::runtime::data::convert::number_compare(&lp, &rp);
        Ok(match self {
            NumOp::Lt => DataValue::Bool(ordering == Some(std::cmp::Ordering::Less)),
            NumOp::Le => DataValue::Bool(matches!(
                ordering,
                Some(std::cmp::Ordering::Less) | Some(std::cmp::Ordering::Equal)
            )),
            NumOp::Add | NumOp::Sub | NumOp::Mul => match (&lp, &rp) {
                (DataValue::Double(a), DataValue::Double(b)) => DataValue::Double(match self {
                    NumOp::Add => a + b,
                    NumOp::Sub => a - b,
                    _ => a * b,
                }),
                _ => {
                    let (a, b) = (
                        qlexpress::runtime::data::convert::to_i64(&lp),
                        qlexpress::runtime::data::convert::to_i64(&rp),
                    );
                    let v = match self {
                        NumOp::Add => a + b,
                        NumOp::Sub => a - b,
                        _ => a * b,
                    };
                    match (&lp, &rp) {
                        (DataValue::Int(_), DataValue::Int(_)) => DataValue::Int(v as i32),
                        _ => DataValue::Long(v),
                    }
                }
            },
        })
    }

    fn operator(&self) -> &str {
        match self {
            NumOp::Add => "+",
            NumOp::Sub => "-",
            NumOp::Mul => "*",
            NumOp::Lt => "<",
            NumOp::Le => "<=",
        }
    }

    fn priority(&self) -> i32 {
        0
    }
}

fn op(o: NumOp) -> Instruction {
    Box::new(OperatorInstruction::new(reporter(), Rc::new(o), None))
}

// ---- tests ---------------------------------------------------------------

#[test]
fn const_arithmetic_return() {
    // 常量加载 + 二元算术 + 返回: 1 + 2 → 3
    let result = run_top(
        &runtime(),
        vec![
            konst(DataValue::Int(1)),
            konst(DataValue::Int(2)),
            op(NumOp::Add),
            ret(),
        ],
    )
    .expect("run");
    assert_eq!(result, QResult::Return(DataValue::Int(3)));
}

#[test]
fn define_and_load_in_scope() {
    // 作用域 define/lookup
    let result = run_top(
        &runtime(),
        vec![
            konst(DataValue::Int(5)),
            Box::new(DefineLocalInstruction::new(reporter(), "x", None)),
            load("x"),
            ret(),
        ],
    )
    .expect("run");
    assert_eq!(result, QResult::Return(DataValue::Int(5)));
}

#[test]
fn if_jump_selects_branch() {
    // if 跳转: cond false → else branch (value 2)
    let result = run_top(
        &runtime(),
        vec![
            konst(DataValue::Bool(false)),                             // 0
            Box::new(JumpIfPopInstruction::new(reporter(), false, 2)), // 1 → 4
            konst(DataValue::Int(1)),                                  // 2
            Box::new(JumpInstruction::new(reporter(), 1)),             // 3 → 5
            konst(DataValue::Int(2)),                                  // 4
            ret(),                                                     // 5
        ],
    )
    .expect("run");
    assert_eq!(result, QResult::Return(DataValue::Int(2)));
}

#[test]
fn for_loop_collects_items() {
    // for 循环指令序列: for (i=0, acc=[]; i<3; i++) acc.add(i)
    let init = def(
        "init",
        vec![
            konst(DataValue::Int(0)),
            Box::new(DefineLocalInstruction::new(reporter(), "i", None)),
            Box::new(NewListInstruction::new(reporter(), 0)),
            Box::new(DefineLocalInstruction::new(reporter(), "acc", None)),
        ],
        vec![],
    );
    let condition = def(
        "cond",
        vec![load("i"), konst(DataValue::Int(3)), op(NumOp::Lt), ret()],
        vec![],
    );
    let update = def(
        "update",
        vec![
            load("i"),
            konst(DataValue::Int(1)),
            op(NumOp::Add),
            Box::new(DefineLocalInstruction::new(reporter(), "i", None)),
        ],
        vec![],
    );
    let body = def(
        "body",
        vec![
            load("acc"),
            load("i"),
            Box::new(MethodInvokeInstruction::new(reporter(), "add", 1, false)),
            Box::new(PopInstruction::new(reporter())),
        ],
        vec![],
    );
    let result = run_top(
        &runtime(),
        vec![
            Box::new(NewListInstruction::new(reporter(), 0)),
            Box::new(DefineLocalInstruction::new(reporter(), "accOuter", None)),
            Box::new(ForInstruction::new(
                reporter(),
                Some(init),
                Some(condition),
                reporter(),
                Some(update),
                8,
                body,
            )),
        ],
    );
    assert!(result.is_ok(), "for loop failed: {:?}", result.err());
}

#[test]
fn for_loop_body_can_access_outer_scope_and_accumulate() {
    // The for scope chains to the defining scope: accumulate into an outer
    // variable and read it back after the loop.
    let init = def(
        "init",
        vec![
            konst(DataValue::Int(0)),
            Box::new(DefineLocalInstruction::new(reporter(), "i", None)),
        ],
        vec![],
    );
    let condition = def(
        "cond",
        vec![load("i"), konst(DataValue::Int(3)), op(NumOp::Lt), ret()],
        vec![],
    );
    let update = def(
        "update",
        vec![
            load("i"),
            konst(DataValue::Int(1)),
            op(NumOp::Add),
            Box::new(DefineLocalInstruction::new(reporter(), "i", None)),
        ],
        vec![],
    );
    // body: acc.add(i) — `acc` resolved through the scope chain
    let body = def(
        "body",
        vec![
            load("acc"),
            load("i"),
            Box::new(MethodInvokeInstruction::new(reporter(), "add", 1, false)),
            Box::new(PopInstruction::new(reporter())),
        ],
        vec![],
    );
    let result = run_top(
        &runtime(),
        vec![
            Box::new(NewListInstruction::new(reporter(), 0)),
            Box::new(DefineLocalInstruction::new(reporter(), "acc", None)),
            Box::new(ForInstruction::new(
                reporter(),
                Some(init),
                Some(condition),
                reporter(),
                Some(update),
                8,
                body,
            )),
            load("acc"),
            ret(),
        ],
    )
    .expect("run");
    let expected = DataValue::list(vec![
        DataValue::Int(0),
        DataValue::Int(1),
        DataValue::Int(2),
    ]);
    assert_eq!(result, QResult::Return(expected));
}

#[test]
fn while_loop_with_break() {
    let condition = def("cond", vec![konst(DataValue::Bool(true)), ret()], vec![]);
    let body = def(
        "body",
        vec![Box::new(BreakContinueInstruction::new(reporter(), true))],
        vec![],
    );
    let result = run_top(
        &runtime(),
        vec![
            Box::new(WhileInstruction::new(reporter(), condition, body, 8)),
            konst(DataValue::Int(9)),
            ret(),
        ],
    )
    .expect("run");
    assert_eq!(result, QResult::Return(DataValue::Int(9)));
}

#[test]
fn lambda_define_and_call() {
    // lambda 定义+调用: function addOne(x) { return x + 1 }; addOne(41)
    let lambda_def = def(
        "addOne",
        vec![load("x"), konst(DataValue::Int(1)), op(NumOp::Add), ret()],
        vec![Param::new("x", None)],
    );
    let result = run_top(
        &runtime(),
        vec![
            Box::new(DefineFunctionInstruction::new(
                reporter(),
                "addOne",
                lambda_def,
            )),
            konst(DataValue::Int(41)),
            Box::new(CallFunctionInstruction::new(reporter(), "addOne", 1, None)),
            ret(),
        ],
    )
    .expect("run");
    assert_eq!(result, QResult::Return(DataValue::Int(42)));
}

#[test]
fn lambda_recursive_self_reference() {
    // lambda 递归引用自身: fact(n) = n <= 1 ? 1 : n * fact(n-1)
    let fact_body = def(
        "fact",
        vec![
            load("n"),                                                           // 0
            konst(DataValue::Int(1)),                                            // 1
            op(NumOp::Le),                                                       // 2
            Box::new(JumpIfPopInstruction::new(reporter(), false, 2)),           // 3 → 6
            konst(DataValue::Int(1)),                                            // 4
            ret(),                                                               // 5
            load("n"),                                                           // 6
            konst(DataValue::Int(1)),                                            // 7
            op(NumOp::Sub),                                                      // 8
            Box::new(CallFunctionInstruction::new(reporter(), "fact", 1, None)), // 9
            load("n"),                                                           // 10
            op(NumOp::Mul),                                                      // 11
            ret(),                                                               // 12
        ],
        vec![Param::new("n", None)],
    );
    let result = run_top(
        &runtime(),
        vec![
            Box::new(DefineFunctionInstruction::new(
                reporter(),
                "fact",
                fact_body,
            )),
            konst(DataValue::Int(5)),
            Box::new(CallFunctionInstruction::new(reporter(), "fact", 1, None)),
            ret(),
        ],
    )
    .expect("run");
    assert_eq!(result, QResult::Return(DataValue::Int(120)));
}

#[test]
fn method_invoke_dispatches_to_native_registry() {
    // 方法调用分派到 NativeRegistry(内建 String 方法)
    let result = run_top(
        &runtime(),
        vec![
            konst(DataValue::Str("hello".into())),
            Box::new(MethodInvokeInstruction::new(
                reporter(),
                "toUpperCase",
                0,
                false,
            )),
            ret(),
        ],
    )
    .expect("run");
    assert_eq!(result, QResult::Return(DataValue::Str("HELLO".into())));
}

#[test]
fn method_not_found_error_code() {
    let err = run_top(
        &runtime(),
        vec![
            konst(DataValue::Str("hello".into())),
            Box::new(MethodInvokeInstruction::new(
                reporter(),
                "noSuchMethod",
                0,
                false,
            )),
            ret(),
        ],
    )
    .expect_err("must fail");
    assert_eq!(err.error_code(), error_codes::METHOD_NOT_FOUND);
}

#[test]
fn timeout_instruction_fires_and_is_disabled_by_zero() {
    // 超时指令: start time in the past + timeout 1ms → SCRIPT_TIME_OUT
    let rt = Rc::new(QvmRuntime::new(
        qlexpress::runtime::trace::QTraces::empty(),
        Default::default(),
        Rc::new(NativeRegistry::with_builtins()),
        current_time_millis() - 10_000,
    ));
    let instructions: Vec<Instruction> = vec![Box::new(CheckTimeOutInstruction::new(reporter()))];
    let mut ctx = DelegateQContext::new(Rc::clone(&rt), QScope::global(QvmGlobalScope::empty()));
    let timeout_opts = QLOptions::builder().timeout_millis(1).build();
    let err = run_instructions(&mut ctx, &instructions, &timeout_opts).expect_err("timeout");
    assert_eq!(err.error_code(), error_codes::SCRIPT_TIME_OUT);
    assert!(err.is_timeout());

    // timeoutMillis <= 0 disables the check
    let mut ctx2 = DelegateQContext::new(Rc::clone(&rt), QScope::global(QvmGlobalScope::empty()));
    let no_timeout = QLOptions::builder().timeout_millis(0).build();
    assert!(run_instructions(&mut ctx2, &instructions, &no_timeout).is_ok());
}

#[test]
fn null_field_access_error_code() {
    let err = run_top(
        &runtime(),
        vec![
            konst(DataValue::Null),
            Box::new(GetFieldInstruction::new(reporter(), "name", false)),
            ret(),
        ],
    )
    .expect_err("must fail");
    assert_eq!(err.error_code(), error_codes::NULL_FIELD_ACCESS);
}

#[test]
fn index_out_of_bound_error_code() {
    let err = run_top(
        &runtime(),
        vec![
            Box::new(NewListInstruction::new(reporter(), 0)),
            konst(DataValue::Int(5)),
            Box::new(IndexInstruction::new(reporter())),
            ret(),
        ],
    )
    .expect_err("must fail");
    assert_eq!(err.error_code(), error_codes::INDEX_OUT_BOUND);
}

#[test]
fn index_pushes_aliased_left_value() {
    // IndexInstruction pushes an l-value aliasing the live list (Java
    // ListItemValue); writes through it mutate the list.
    let list = DataValue::list(vec![DataValue::Int(1), DataValue::Int(2)]);
    let instructions: Vec<Instruction> = vec![
        konst(list.clone()),
        konst(DataValue::Int(0)),
        Box::new(IndexInstruction::new(reporter())),
    ];
    let ctx = run_with_ctx(&runtime(), &instructions).expect("run");
    let top = ctx.peek();
    let left = top.as_left().expect("index result is a left value");
    left.borrow_mut()
        .set(DataValue::Int(99), &PureErrReporter::INSTANCE)
        .expect("set");
    if let DataValue::List(items) = &list {
        assert_eq!(items.borrow()[0], DataValue::Int(99));
    } else {
        panic!("expected list");
    }
}

#[test]
fn stack_input_output_matches_java() {
    // 抽查 stack_input()/stack_output() 与 Java 一致
    let r = reporter();
    assert_eq!(
        ConstInstruction::new(Rc::clone(&r), DataValue::Null, None).stack_input(),
        0
    );
    assert_eq!(
        ConstInstruction::new(r.clone(), DataValue::Null, None).stack_output(),
        1
    );
    assert_eq!(IndexInstruction::new(r.clone()).stack_input(), 2);
    assert_eq!(
        JumpIfPopInstruction::new(r.clone(), true, 0).stack_input(),
        1
    );
    assert_eq!(JumpInstruction::new(r.clone(), 0).stack_output(), 0);
    assert_eq!(
        CallFunctionInstruction::new(r.clone(), "f", 3, None).stack_input(),
        3
    );
    assert_eq!(
        MethodInvokeInstruction::new(r.clone(), "m", 2, false).stack_input(),
        3
    );
    assert_eq!(CheckTimeOutInstruction::new(r.clone()).stack_input(), 0);
}

#[test]
fn println_debug_output_migrated() {
    let r = reporter();
    let mut out = Vec::new();
    ConstInstruction::new(r.clone(), DataValue::Int(7), None).println(3, 0, &mut |s| out.push(s));
    JumpIfPopInstruction::new(r.clone(), false, 5).println(1, 0, &mut |s| out.push(s));
    LoadInstruction::new(r.clone(), "abc", None).println(0, 0, &mut |s| out.push(s));
    assert_eq!(
        out,
        vec!["3: LoadConst 7", "1: JumpIfPop false 5", "0: Load abc"]
    );
}
