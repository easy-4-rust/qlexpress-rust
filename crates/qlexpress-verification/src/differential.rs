//! Rust 侧差分语料执行器。

use std::collections::HashMap;
use std::fs::File;
use std::io::{BufRead, BufReader, BufWriter, Write};
use std::path::Path;
use std::rc::Rc;

use qlexpress::aparser::operator_factory::OperatorFactory;
use qlexpress::aparser::parser_operator_manager::{OpType, ParserOperatorManager};
use qlexpress::exception::pure_err_reporter::PureErrReporter;
use qlexpress::init_options::InitOptions;
use qlexpress::number::big_decimal_math::BigDecimalMath;
use qlexpress::number::big_integer_math::BigIntegerMath;
use qlexpress::number::floating_point_math::FloatingPointMath;
use qlexpress::number::integer_math::IntegerMath;
use qlexpress::number::long_math::LongMath;
use qlexpress::number::number_math::NumberMath;
use qlexpress::operator::operator_manager::OperatorManager;
use qlexpress::ql_options::{Attachments, QLOptions, QLOptionsBuilder, SharedAttachments};
use qlexpress::runtime::class_ref::ClassRef;
use qlexpress::runtime::context::EmptyContext;
use qlexpress::runtime::data::convert::{self, MathDomain};
use qlexpress::runtime::data::index_map::IndexMap;
use qlexpress::runtime::delegate_qcontext::DelegateQContext;
use qlexpress::runtime::exception_table::ExceptionTable;
use qlexpress::runtime::member::NativeRegistry;
use qlexpress::runtime::opaque_native_object::OpaqueNativeObject;
use qlexpress::runtime::q_runtime::QRuntime;
use qlexpress::runtime::qcontext::QContext;
use qlexpress::runtime::qvm_global_scope::QvmGlobalScope;
use qlexpress::runtime::qvm_runtime::QvmRuntime;
use qlexpress::runtime::scope::QScope;
use qlexpress::runtime::trace::QTraces;
use qlexpress::runtime::value::{DataValue, QValue};
use qlexpress::security::ql_security_strategy::QLSecurityStrategy;
use qlexpress::Express4Runner;
use serde::{Deserialize, Serialize};

use crate::normalization::normalize;

#[derive(Debug, Deserialize)]
struct DifferentialCase {
    id: String,
    #[serde(default)]
    script: Option<String>,
    #[serde(default)]
    number_math: Option<NumberMathInvocation>,
    #[serde(default)]
    operator_manager: Option<OperatorManagerInvocation>,
    #[serde(default)]
    delegate_context: Option<DelegateContextInvocation>,
    #[serde(default)]
    fixed_size_stack: Option<FixedSizeStackInvocation>,
    #[serde(default)]
    runtime_core: Option<RuntimeCoreInvocation>,
    #[serde(default)]
    exception_table: Option<ExceptionTableInvocation>,
    #[serde(default)]
    context: serde_json::Map<String, serde_json::Value>,
    #[serde(default)]
    options: DifferentialOptions,
}

#[derive(Debug, Deserialize)]
struct NumberMathInvocation {
    #[serde(default)]
    implementation: Option<String>,
    operation: String,
    left: TypedNumber,
    #[serde(default)]
    right: Option<TypedNumber>,
}

#[derive(Debug, Deserialize)]
struct TypedNumber {
    #[serde(rename = "type")]
    number_type: String,
    value: String,
}

#[derive(Debug, Deserialize)]
struct OperatorManagerInvocation {
    operation: String,
    #[serde(default)]
    lexeme: Option<String>,
    #[serde(default)]
    origin: Option<String>,
    #[serde(default)]
    keyword: Option<String>,
    #[serde(default)]
    op_type: Option<String>,
    #[serde(default)]
    priority: Option<i32>,
    #[serde(default)]
    left: Option<TypedNumber>,
    #[serde(default)]
    right: Option<TypedNumber>,
    #[serde(default)]
    setup: Vec<OperatorManagerSetup>,
}

#[derive(Debug, Deserialize)]
struct OperatorManagerSetup {
    action: String,
    lexeme: String,
    #[serde(default)]
    origin: Option<String>,
    #[serde(default)]
    keyword: Option<String>,
    #[serde(default)]
    priority: Option<i32>,
}

#[derive(Debug, Deserialize)]
struct DelegateContextInvocation {
    scenario: String,
}

#[derive(Debug, Deserialize)]
struct FixedSizeStackInvocation {
    scenario: String,
}

#[derive(Debug, Deserialize)]
struct RuntimeCoreInvocation {
    scenario: String,
}

#[derive(Debug, Deserialize)]
struct ExceptionTableInvocation {
    scenario: String,
}

#[derive(Clone, Debug, Default, Deserialize)]
struct DifferentialOptions {
    precise: Option<bool>,
    cache: Option<bool>,
    avoid_null_pointer: Option<bool>,
    max_arr_length: Option<i32>,
    trace_expression: Option<bool>,
    short_circuit_disable: Option<bool>,
    timeout_millis: Option<i64>,
}

#[derive(Debug, Serialize)]
struct DifferentialRecord {
    id: String,
    outcome: &'static str,
    normalized: Option<String>,
    error_code: Option<String>,
    line: Option<i32>,
    column: Option<i32>,
    trace_count: usize,
}

/// 对应 Java: 无（Rust 原生适配）。
pub fn run(corpus: &Path, output: &Path) -> Result<(), String> {
    let input =
        File::open(corpus).map_err(|error| format!("open corpus {}: {error}", corpus.display()))?;
    let output_file = File::create(output)
        .map_err(|error| format!("create output {}: {error}", output.display()))?;
    let mut writer = BufWriter::new(output_file);
    let mut count = 0usize;
    for (index, line) in BufReader::new(input).lines().enumerate() {
        let line = line.map_err(|error| format!("read corpus line {}: {error}", index + 1))?;
        if line.trim().is_empty() || line.trim_start().starts_with('#') {
            continue;
        }
        let case: DifferentialCase = serde_json::from_str(&line)
            .map_err(|error| format!("parse corpus line {}: {error}", index + 1))?;
        let record = execute_case(case)?;
        serde_json::to_writer(&mut writer, &record)
            .map_err(|error| format!("write result: {error}"))?;
        writer
            .write_all(b"\n")
            .map_err(|error| format!("write result newline: {error}"))?;
        count += 1;
    }
    writer
        .flush()
        .map_err(|error| format!("flush result: {error}"))?;
    eprintln!("rust differential cases completed: {count}");
    Ok(())
}

fn execute_case(case: DifferentialCase) -> Result<DifferentialRecord, String> {
    if let Some(invocation) = case.number_math {
        return execute_number_math(case.id, invocation);
    }
    if let Some(invocation) = case.operator_manager {
        return execute_operator_manager(case.id, invocation);
    }
    if let Some(invocation) = case.delegate_context {
        return execute_delegate_context(case.id, invocation);
    }
    if let Some(invocation) = case.fixed_size_stack {
        return execute_fixed_size_stack(case.id, invocation);
    }
    if let Some(invocation) = case.runtime_core {
        return execute_runtime_core(case.id, invocation);
    }
    if let Some(invocation) = case.exception_table {
        return execute_exception_table(case.id, invocation);
    }
    let script = case
        .script
        .ok_or_else(|| format!("differential case {} has no supported invocation", case.id))?;
    let init_options = InitOptions::builder()
        .security_strategy(QLSecurityStrategy::open())
        .trace_expression(true)
        .build();
    let runner = Express4Runner::with_init_options(init_options);
    let context = case
        .context
        .into_iter()
        .map(|(key, value)| json_to_data_value(value).map(|value| (key, value)))
        .collect::<Result<HashMap<_, _>, _>>()?;
    let options = build_options(&case.options);
    match runner.execute(&script, context, &options) {
        Ok(result) => Ok(DifferentialRecord {
            id: case.id,
            outcome: "ok",
            normalized: Some(normalize(result.result())),
            error_code: None,
            line: None,
            column: None,
            trace_count: result.expression_traces().len(),
        }),
        Err(error) => Ok(DifferentialRecord {
            id: case.id,
            outcome: "error",
            normalized: Some(format!("error:{}:{}", error.error_code(), error.reason())),
            error_code: Some(error.error_code().to_string()),
            line: Some(error.line_no()),
            column: Some(error.col_no()),
            trace_count: 0,
        }),
    }
}

fn execute_exception_table(
    id: String,
    invocation: ExceptionTableInvocation,
) -> Result<DifferentialRecord, String> {
    if invocation.scenario != "full_contract" {
        return Err(format!(
            "unsupported exception_table scenario: {}",
            invocation.scenario
        ));
    }

    let table = ExceptionTable::from_handler_positions(
        vec![
            (ClassRef::from_name("java.lang.Number"), 11),
            (ClassRef::from_name("java.lang.RuntimeException"), 22),
            (ClassRef::from_name("java.lang.Object"), 33),
        ],
        Some(44),
    );
    let object_first = ExceptionTable::from_handler_positions(
        vec![
            (ClassRef::from_name("java.lang.Object"), 5),
            (ClassRef::from_name("java.lang.Number"), 6),
        ],
        None,
    );
    let illegal_argument =
        OpaqueNativeObject::new("java.lang.IllegalArgumentException").into_data_value();
    let mut observed = IndexMap::new();
    observed.insert(
        DataValue::string("null_to_first"),
        table
            .get_relative_pos(&DataValue::Null)
            .map(DataValue::Int)
            .unwrap_or(DataValue::Null),
    );
    observed.insert(
        DataValue::string("integer_to_number"),
        table
            .get_relative_pos(&DataValue::Int(1))
            .map(DataValue::Int)
            .unwrap_or(DataValue::Null),
    );
    observed.insert(
        DataValue::string("long_to_number"),
        table
            .get_relative_pos(&DataValue::Long(1))
            .map(DataValue::Int)
            .unwrap_or(DataValue::Null),
    );
    observed.insert(
        DataValue::string("runtime_subclass"),
        table
            .get_relative_pos(&illegal_argument)
            .map(DataValue::Int)
            .unwrap_or(DataValue::Null),
    );
    observed.insert(
        DataValue::string("string_to_object"),
        table
            .get_relative_pos(&DataValue::string("fallback"))
            .map(DataValue::Int)
            .unwrap_or(DataValue::Null),
    );
    observed.insert(
        DataValue::string("final_pos"),
        table
            .get_final_pos()
            .map(DataValue::Int)
            .unwrap_or(DataValue::Null),
    );
    observed.insert(
        DataValue::string("declaration_order"),
        object_first
            .get_relative_pos(&DataValue::Int(1))
            .map(DataValue::Int)
            .unwrap_or(DataValue::Null),
    );
    observed.insert(
        DataValue::string("empty_relative"),
        ExceptionTable::new()
            .get_relative_pos(&DataValue::Int(1))
            .map(DataValue::Int)
            .unwrap_or(DataValue::Null),
    );
    observed.insert(
        DataValue::string("empty_final"),
        ExceptionTable::new()
            .get_final_pos()
            .map(DataValue::Int)
            .unwrap_or(DataValue::Null),
    );

    Ok(DifferentialRecord {
        id,
        outcome: "ok",
        normalized: Some(normalize(&DataValue::map(observed))),
        error_code: None,
        line: None,
        column: None,
        trace_count: 0,
    })
}

fn execute_runtime_core(
    id: String,
    invocation: RuntimeCoreInvocation,
) -> Result<DifferentialRecord, String> {
    if invocation.scenario != "full_contract" {
        return Err(format!(
            "unsupported runtime_core scenario: {}",
            invocation.scenario
        ));
    }

    let mut attachments = Attachments::new();
    attachments.insert("tenant".to_string(), DataValue::string("acme"));
    let shared_attachments: SharedAttachments = Rc::new(std::cell::RefCell::new(attachments));
    let registry = Rc::new(NativeRegistry::with_builtins());
    let traces = QTraces::empty();
    let runtime = Rc::new(QvmRuntime::new(
        traces,
        Rc::clone(&shared_attachments),
        Rc::clone(&registry),
        424_242,
    ));
    let mut observed = IndexMap::new();

    observed.insert(
        DataValue::string("runtime_start"),
        DataValue::Long(runtime.script_start_time_stamp()),
    );
    observed.insert(
        DataValue::string("runtime_attachment_initial"),
        runtime
            .attachment()
            .get("tenant")
            .cloned()
            .unwrap_or(DataValue::Null),
    );
    observed.insert(
        DataValue::string("runtime_registry_same"),
        DataValue::Bool(Rc::ptr_eq(runtime.registry(), &registry)),
    );
    observed.insert(
        DataValue::string("runtime_trace_count"),
        DataValue::Int(runtime.traces().snapshot().len() as i32),
    );

    shared_attachments
        .borrow_mut()
        .insert("external_write".to_string(), DataValue::Int(7));
    observed.insert(
        DataValue::string("external_write_visible"),
        runtime
            .attachment()
            .get("external_write")
            .cloned()
            .unwrap_or(DataValue::Null),
    );
    runtime
        .attachment_mut()
        .insert("runtime_write".to_string(), DataValue::Int(8));
    observed.insert(
        DataValue::string("runtime_write_visible_external"),
        shared_attachments
            .borrow()
            .get("runtime_write")
            .cloned()
            .unwrap_or(DataValue::Null),
    );

    let global_scope = QScope::global(QvmGlobalScope::empty());
    let block_scope = QScope::block_fresh_stack(&global_scope, Default::default(), 1);
    let mut context = DelegateQContext::new(Rc::clone(&runtime), Rc::clone(&block_scope));
    observed.insert(
        DataValue::string("context_runtime_same"),
        DataValue::Bool(Rc::ptr_eq(context.q_runtime(), &runtime)),
    );
    observed.insert(
        DataValue::string("context_start"),
        DataValue::Long(context.script_start_time_stamp()),
    );
    observed.insert(
        DataValue::string("context_registry_same"),
        DataValue::Bool(Rc::ptr_eq(context.registry(), &registry)),
    );
    observed.insert(
        DataValue::string("context_traces_same"),
        DataValue::Bool(std::ptr::eq(context.traces(), runtime.traces())),
    );
    observed.insert(
        DataValue::string("context_current_initial"),
        DataValue::Bool(Rc::ptr_eq(&context.current_scope(), &block_scope)),
    );
    context
        .attachment_mut()
        .insert("context_write".to_string(), DataValue::Int(9));
    observed.insert(
        DataValue::string("context_write_visible_runtime"),
        runtime
            .attachment()
            .get("context_write")
            .cloned()
            .unwrap_or(DataValue::Null),
    );
    let child = context.new_scope();
    observed.insert(
        DataValue::string("context_current_child"),
        DataValue::Bool(Rc::ptr_eq(&context.current_scope(), &child)),
    );
    context.close_scope();
    observed.insert(
        DataValue::string("context_closed_to_parent"),
        DataValue::Bool(Rc::ptr_eq(&context.current_scope(), &block_scope)),
    );

    Ok(DifferentialRecord {
        id,
        outcome: "ok",
        normalized: Some(normalize(&DataValue::map(observed))),
        error_code: None,
        line: None,
        column: None,
        trace_count: 0,
    })
}

fn execute_fixed_size_stack(
    id: String,
    invocation: FixedSizeStackInvocation,
) -> Result<DifferentialRecord, String> {
    if invocation.scenario != "full_contract" {
        return Err(format!(
            "unsupported fixed_size_stack scenario: {}",
            invocation.scenario
        ));
    }

    let mut stack = qlexpress::runtime::fixed_size_stack::FixedSizeStack::new(4);
    let mut observed = IndexMap::new();
    observed.insert(
        DataValue::string("capacity"),
        DataValue::Int(stack.capacity() as i32),
    );
    for value in 1..=4 {
        stack.push(DataValue::Int(value).into());
    }
    observed.insert(DataValue::string("peak"), stack.peak().get());
    observed.insert(DataValue::string("pop_4"), stack.pop().get());
    observed.insert(DataValue::string("pop_3"), stack.pop().get());
    stack.push(DataValue::Int(5).into());
    stack.push(DataValue::Int(6).into());

    let parameters = stack.pop_n(3);
    observed.insert(
        DataValue::string("parameters_size"),
        DataValue::Int(parameters.size() as i32),
    );
    observed.insert(
        DataValue::string("parameters_present_0"),
        DataValue::Bool(parameters.get(0).is_some()),
    );
    observed.insert(
        DataValue::string("parameters_values"),
        DataValue::list(parameters.values()),
    );
    observed.insert(
        DataValue::string("parameters_oob_present"),
        DataValue::Bool(parameters.get(3).is_some()),
    );
    observed.insert(
        DataValue::string("parameters_oob_value"),
        parameters.get_value(3),
    );
    observed.insert(DataValue::string("remaining_peak"), stack.peak().get());

    stack.push(DataValue::Int(9).into());
    observed.insert(
        DataValue::string("live_after_one_push"),
        DataValue::list(parameters.values()),
    );
    stack.push(DataValue::Int(8).into());
    observed.insert(
        DataValue::string("live_after_two_pushes"),
        DataValue::list(parameters.values()),
    );
    stack.push(DataValue::Int(7).into());
    observed.insert(
        DataValue::string("live_after_three_pushes"),
        DataValue::list(parameters.values()),
    );
    observed.insert(DataValue::string("pop_reused_top"), stack.pop().get());
    observed.insert(DataValue::string("peak_after_pop"), stack.peak().get());
    let empty_parameters = stack.pop_n(0);
    observed.insert(
        DataValue::string("zero_pop_size"),
        DataValue::Int(empty_parameters.size() as i32),
    );
    observed.insert(
        DataValue::string("zero_pop_get"),
        DataValue::Bool(empty_parameters.get(0).is_some()),
    );

    let mut null_stack = qlexpress::runtime::fixed_size_stack::FixedSizeStack::new(1);
    null_stack.push(DataValue::Null.into());
    let null_parameters = null_stack.pop_n(1);
    observed.insert(
        DataValue::string("null_slot_present"),
        DataValue::Bool(null_parameters.get(0).is_some()),
    );
    observed.insert(
        DataValue::string("null_slot_value"),
        null_parameters.get_value(0),
    );

    Ok(DifferentialRecord {
        id,
        outcome: "ok",
        normalized: Some(normalize(&DataValue::map(observed))),
        error_code: None,
        line: None,
        column: None,
        trace_count: 0,
    })
}

fn execute_delegate_context(
    id: String,
    invocation: DelegateContextInvocation,
) -> Result<DifferentialRecord, String> {
    if invocation.scenario == "close_global" {
        return execute_delegate_close_global(id);
    }
    if invocation.scenario != "full_contract" {
        return Err(format!(
            "unsupported delegate_context scenario: {}",
            invocation.scenario
        ));
    }

    let mut attachment_values = Attachments::new();
    attachment_values.insert("tenant".to_string(), DataValue::string("acme"));
    let shared_attachments: SharedAttachments = Rc::new(std::cell::RefCell::new(attachment_values));
    let registry = Rc::new(NativeRegistry::with_builtins());
    let runtime = Rc::new(QvmRuntime::new(
        QTraces::empty(),
        Rc::clone(&shared_attachments),
        Rc::clone(&registry),
        123_456,
    ));
    let shared_functions = Rc::new(std::cell::RefCell::new(HashMap::new()));
    let global_scope = QScope::global(QvmGlobalScope::with_shared_context(
        Rc::new(EmptyContext::new()),
        Rc::clone(&shared_functions),
        Rc::clone(&shared_attachments),
        false,
    ));
    let block_scope = QScope::block_fresh_stack(&global_scope, Default::default(), 8);
    let mut context = DelegateQContext::new(Rc::clone(&runtime), Rc::clone(&block_scope));
    let mut observed = IndexMap::new();

    observed.insert(
        DataValue::string("start_time"),
        DataValue::Long(context.script_start_time_stamp()),
    );
    observed.insert(
        DataValue::string("attachment"),
        context
            .attachment()
            .get("tenant")
            .cloned()
            .unwrap_or(DataValue::Null),
    );
    observed.insert(
        DataValue::string("reflect_same"),
        DataValue::Bool(Rc::ptr_eq(context.get_reflect_loader(), &registry)),
    );
    observed.insert(
        DataValue::string("traces_same"),
        DataValue::Bool(std::ptr::eq(context.traces(), runtime.traces())),
    );
    observed.insert(
        DataValue::string("trace_count"),
        DataValue::Int(context.traces().snapshot().len() as i32),
    );
    observed.insert(
        DataValue::string("current_initial"),
        DataValue::Bool(Rc::ptr_eq(&context.current_scope(), &block_scope)),
    );
    observed.insert(
        DataValue::string("parent_initial"),
        DataValue::Bool(
            context
                .parent_scope()
                .is_some_and(|parent| Rc::ptr_eq(&parent, &global_scope)),
        ),
    );

    context.define_local_symbol(
        "x",
        Some(ClassRef::from_name("java.lang.Integer")),
        DataValue::Int(7),
    );
    let symbol = context
        .get_symbol("x")
        .map_err(|error| error.to_string())?
        .ok_or_else(|| "DelegateQContext lost local symbol x".to_string())?;
    observed.insert(DataValue::string("symbol_present"), DataValue::Bool(true));
    observed.insert(DataValue::string("symbol_value"), symbol.borrow().get());
    observed.insert(
        DataValue::string("missing_value"),
        context
            .get_symbol_value("missing")
            .map_err(|error| error.to_string())?
            .unwrap_or(DataValue::Null),
    );

    let function = delegate_contract_function();
    context.define_function("f", Rc::clone(&function));
    observed.insert(
        DataValue::string("function_get"),
        DataValue::Bool(context.get_function("f").is_some()),
    );
    let function_table = context.function_table();
    function_table
        .borrow_mut()
        .insert("g".to_string(), Rc::clone(&function));
    observed.insert(
        DataValue::string("function_table_size"),
        DataValue::Int(function_table.borrow().len() as i32),
    );
    observed.insert(
        DataValue::string("function_table_write_through"),
        DataValue::Bool(context.get_function("g").is_some()),
    );

    context.push(DataValue::Int(1).into());
    context.push(DataValue::Int(2).into());
    let child_scope = context.new_scope();
    context.push(DataValue::Int(3).into());
    observed.insert(DataValue::string("stack_peek"), context.peek().get());
    let popped = context.pop_n(2);
    observed.insert(
        DataValue::string("pop_n_size"),
        DataValue::Int(popped.size() as i32),
    );
    observed.insert(DataValue::string("pop_n_0"), popped.get_value(0));
    observed.insert(DataValue::string("pop_n_1"), popped.get_value(1));
    observed.insert(DataValue::string("stack_after_pop_n"), context.peek().get());
    context.push(DataValue::Int(4).into());
    observed.insert(DataValue::string("pop_single"), context.pop().get());
    observed.insert(
        DataValue::string("child_current"),
        DataValue::Bool(Rc::ptr_eq(&context.current_scope(), &child_scope)),
    );
    observed.insert(
        DataValue::string("child_parent"),
        DataValue::Bool(
            context
                .parent_scope()
                .is_some_and(|parent| Rc::ptr_eq(&parent, &block_scope)),
        ),
    );
    observed.insert(
        DataValue::string("child_inherits_function"),
        DataValue::Bool(context.get_function("f").is_some()),
    );
    observed.insert(
        DataValue::string("child_function_table_size"),
        DataValue::Int(context.function_table().borrow().len() as i32),
    );
    observed.insert(
        DataValue::string("child_inherited_symbol"),
        context
            .get_symbol_value("x")
            .map_err(|error| error.to_string())?
            .unwrap_or(DataValue::Null),
    );
    context.define_local_symbol(
        "x",
        Some(ClassRef::from_name("java.lang.Integer")),
        DataValue::Int(9),
    );
    observed.insert(
        DataValue::string("child_shadow"),
        context
            .get_symbol_value("x")
            .map_err(|error| error.to_string())?
            .unwrap_or(DataValue::Null),
    );
    context.close_scope();
    observed.insert(
        DataValue::string("closed_to_parent"),
        DataValue::Bool(Rc::ptr_eq(&context.current_scope(), &block_scope)),
    );
    observed.insert(
        DataValue::string("parent_symbol_after_close"),
        context
            .get_symbol_value("x")
            .map_err(|error| error.to_string())?
            .unwrap_or(DataValue::Null),
    );
    context.close_scope();
    observed.insert(
        DataValue::string("closed_to_global"),
        DataValue::Bool(Rc::ptr_eq(&context.current_scope(), &global_scope)),
    );

    Ok(DifferentialRecord {
        id,
        outcome: "ok",
        normalized: Some(normalize(&DataValue::map(observed))),
        error_code: None,
        line: None,
        column: None,
        trace_count: 0,
    })
}

fn execute_delegate_close_global(id: String) -> Result<DifferentialRecord, String> {
    let registry = Rc::new(NativeRegistry::with_builtins());
    let runtime = Rc::new(QvmRuntime::for_test(registry));
    let mut context = DelegateQContext::new(runtime, QScope::global(QvmGlobalScope::empty()));
    let previous_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(|_| {}));
    let panic = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        context.close_scope();
    }));
    std::panic::set_hook(previous_hook);
    let Err(payload) = panic else {
        return Err("DelegateQContext.close_scope silently accepted global scope".to_string());
    };
    let reason = payload
        .downcast_ref::<&str>()
        .copied()
        .or_else(|| payload.downcast_ref::<String>().map(String::as_str))
        .unwrap_or_default();
    if !reason.contains("QvmGlobalScope.getParent is unsupported") {
        return Err(format!(
            "DelegateQContext.close_scope panicked for an unexpected reason: {reason}"
        ));
    }
    Ok(DifferentialRecord {
        id,
        outcome: "error",
        normalized: Some(
            "error:UNSUPPORTED_OPERATION:QvmGlobalScope.getParent is unsupported".to_string(),
        ),
        error_code: Some("UNSUPPORTED_OPERATION".to_string()),
        line: Some(0),
        column: Some(0),
        trace_count: 0,
    })
}

fn delegate_contract_function() -> Rc<dyn qlexpress::runtime::CustomFunction> {
    Rc::new(DelegateContractFunction)
}

struct DelegateContractFunction;

impl qlexpress::runtime::CustomFunction for DelegateContractFunction {
    #[allow(clippy::result_large_err)]
    fn call(
        &self,
        _q_context: &mut dyn QContext,
        parameters: &qlexpress::runtime::parameters::Parameters,
    ) -> Result<DataValue, qlexpress::exception::QLException> {
        Ok(DataValue::Int(parameters.size() as i32))
    }
}

fn execute_operator_manager(
    id: String,
    invocation: OperatorManagerInvocation,
) -> Result<DifferentialRecord, String> {
    let mut manager = OperatorManager::new();
    for setup in &invocation.setup {
        if !apply_operator_manager_setup(&mut manager, setup)? {
            return Err(format!(
                "operator_manager setup failed: {} {}",
                setup.action, setup.lexeme
            ));
        }
    }

    let lexeme = || {
        invocation
            .lexeme
            .as_deref()
            .ok_or_else(|| format!("operator_manager {} requires lexeme", invocation.operation))
    };
    let normalized =
        match invocation.operation.as_str() {
            "addBinaryOperator" => normalize(&DataValue::Bool(manager.add_binary_operator(
                lexeme()?,
                additive_custom_operator(),
                invocation.priority.unwrap_or(300),
            ))),
            "replaceDefaultOperator" => normalize(&DataValue::Bool(
                manager.replace_default_operator(lexeme()?, additive_custom_operator()),
            )),
            "addOperatorAlias" => normalize(&DataValue::Bool(manager.add_operator_alias(
                lexeme()?,
                invocation.origin.as_deref().ok_or_else(|| {
                    "operator_manager addOperatorAlias requires origin".to_string()
                })?,
            ))),
            "addKeyWordAlias" => normalize(&DataValue::Bool(manager.add_key_word_alias(
                lexeme()?,
                invocation.keyword.as_deref().ok_or_else(|| {
                    "operator_manager addKeyWordAlias requires keyword".to_string()
                })?,
            ))),
            "getBinaryOperator" => operator_metadata(manager.get_binary_operator(lexeme()?)),
            "getPrefixUnaryOperator" => manager
                .get_prefix_unary_operator(lexeme()?)
                .map(|operator| {
                    normalize(&DataValue::string(format!(
                        "{}|{}",
                        operator.operator(),
                        operator.priority()
                    )))
                })
                .unwrap_or_else(|| normalize(&DataValue::Null)),
            "getSuffixUnaryOperator" => manager
                .get_suffix_unary_operator(lexeme()?)
                .map(|operator| {
                    normalize(&DataValue::string(format!(
                        "{}|{}",
                        operator.operator(),
                        operator.priority()
                    )))
                })
                .unwrap_or_else(|| normalize(&DataValue::Null)),
            "isOpType" => {
                let op_type = match invocation.op_type.as_deref() {
                    Some("MIDDLE") => OpType::Middle,
                    Some("PREFIX") => OpType::Prefix,
                    Some("SUFFIX") => OpType::Suffix,
                    value => {
                        return Err(format!("unsupported operator_manager op_type: {value:?}"))
                    }
                };
                normalize(&DataValue::Bool(manager.is_op_type(lexeme()?, op_type)))
            }
            "precedence" => manager
                .precedence(lexeme()?)
                .map(DataValue::Int)
                .map(|value| normalize(&value))
                .unwrap_or_else(|| normalize(&DataValue::Null)),
            "getAlias" => manager
                .get_alias(lexeme()?)
                .map(DataValue::Int)
                .map(|value| normalize(&value))
                .unwrap_or_else(|| normalize(&DataValue::Null)),
            "executeBinary" => {
                let left =
                    typed_number_to_data_value(invocation.left.as_ref().ok_or_else(|| {
                        "operator_manager executeBinary requires left".to_string()
                    })?)?;
                let right =
                    typed_number_to_data_value(invocation.right.as_ref().ok_or_else(|| {
                        "operator_manager executeBinary requires right".to_string()
                    })?)?;
                let operator_lexeme = lexeme()?;
                let operator = manager
                    .get_binary_operator(operator_lexeme)
                    .ok_or_else(|| {
                        format!("operator_manager binary operator not found: {operator_lexeme}")
                    })?;
                let runtime = QvmRuntime::for_test(Rc::new(NativeRegistry::with_builtins()));
                let global_scope = QScope::global(QvmGlobalScope::empty());
                let instruction_scope =
                    QScope::block_fresh_stack(&global_scope, Default::default(), 4);
                let mut context = DelegateQContext::new(Rc::new(runtime), instruction_scope);
                match operator.execute(
                    &QValue::Data(left),
                    &QValue::Data(right),
                    &mut context,
                    &QLOptions::builder().build(),
                    &PureErrReporter::INSTANCE,
                ) {
                    Ok(value) => normalize(&value),
                    Err(error) => {
                        return Ok(DifferentialRecord {
                            id,
                            outcome: "error",
                            normalized: Some(format!(
                                "error:{}:{}",
                                error.error_code(),
                                error.reason()
                            )),
                            error_code: Some(error.error_code().to_string()),
                            line: Some(error.line_no()),
                            column: Some(error.col_no()),
                            trace_count: 0,
                        });
                    }
                }
            }
            operation => {
                return Err(format!(
                    "unsupported operator_manager operation: {operation}"
                ))
            }
        };

    Ok(DifferentialRecord {
        id,
        outcome: "ok",
        normalized: Some(normalized),
        error_code: None,
        line: None,
        column: None,
        trace_count: 0,
    })
}

fn additive_custom_operator() -> Rc<dyn qlexpress::operator::CustomBinaryOperator> {
    Rc::new(AdditiveCustomOperator)
}

struct AdditiveCustomOperator;

impl qlexpress::operator::CustomBinaryOperator for AdditiveCustomOperator {
    #[allow(clippy::result_large_err)]
    fn execute(
        &self,
        left: &QValue,
        right: &QValue,
    ) -> Result<DataValue, qlexpress::exception::QLException> {
        let left = left.get();
        let right = right.get();
        NumberMath::add(&left, &right)
    }
}

fn apply_operator_manager_setup(
    manager: &mut OperatorManager,
    setup: &OperatorManagerSetup,
) -> Result<bool, String> {
    match setup.action.as_str() {
        "add" => Ok(manager.add_binary_operator(
            &setup.lexeme,
            additive_custom_operator(),
            setup.priority.unwrap_or(300),
        )),
        "replace" => {
            Ok(manager.replace_default_operator(&setup.lexeme, additive_custom_operator()))
        }
        "operator_alias" => Ok(manager.add_operator_alias(
            &setup.lexeme,
            setup.origin.as_deref().ok_or_else(|| {
                "operator_manager operator_alias setup requires origin".to_string()
            })?,
        )),
        "keyword_alias" => Ok(manager.add_key_word_alias(
            &setup.lexeme,
            setup.keyword.as_deref().ok_or_else(|| {
                "operator_manager keyword_alias setup requires keyword".to_string()
            })?,
        )),
        action => Err(format!(
            "unsupported operator_manager setup action: {action}"
        )),
    }
}

fn operator_metadata(operator: Option<Rc<dyn qlexpress::operator::BinaryOperator>>) -> String {
    operator
        .map(|operator| {
            normalize(&DataValue::string(format!(
                "{}|{}",
                operator.operator(),
                operator.priority()
            )))
        })
        .unwrap_or_else(|| normalize(&DataValue::Null))
}

fn execute_number_math(
    id: String,
    invocation: NumberMathInvocation,
) -> Result<DifferentialRecord, String> {
    let left = typed_number_to_data_value(&invocation.left)?;
    let right = invocation
        .right
        .as_ref()
        .map(typed_number_to_data_value)
        .transpose()?;
    let result = if let Some(implementation) = invocation.implementation.as_deref() {
        execute_concrete_number_math(implementation, &invocation.operation, &left, right.as_ref())?
    } else {
        execute_number_math_facade(&id, &invocation.operation, &left, right.as_ref())?
    };
    match result {
        Ok(value) => Ok(DifferentialRecord {
            id,
            outcome: "ok",
            normalized: Some(normalize(&value)),
            error_code: None,
            line: None,
            column: None,
            trace_count: 0,
        }),
        Err(error) => {
            let category = number_math_error_category(&error);
            Ok(DifferentialRecord {
                id,
                outcome: "error",
                normalized: Some(format!("error:{category}:{}", error.reason())),
                error_code: Some(category.to_string()),
                line: Some(0),
                column: Some(0),
                trace_count: 0,
            })
        }
    }
}

fn execute_number_math_facade(
    id: &str,
    operation: &str,
    left: &DataValue,
    right: Option<&DataValue>,
) -> Result<Result<DataValue, qlexpress::exception::QLException>, String> {
    let binary = || right.ok_or_else(|| format!("number_math {operation} requires right operand"));
    let result = match operation {
        "abs" => NumberMath::abs(left),
        "add" => NumberMath::add(left, binary()?),
        "subtract" => NumberMath::subtract(left, binary()?),
        "multiply" => NumberMath::multiply(left, binary()?),
        "divide" => NumberMath::divide(left, binary()?),
        "compareTo" => NumberMath::compare_to(left, binary()?).map(DataValue::Int),
        "or" => NumberMath::or(left, binary()?),
        "and" => NumberMath::and(left, binary()?),
        "xor" => NumberMath::xor(left, binary()?),
        "intDiv" => NumberMath::int_div(left, binary()?),
        "mod" => NumberMath::mod_op(left, binary()?),
        "remainder" => NumberMath::remainder(left, binary()?),
        "leftShift" => NumberMath::left_shift(left, binary()?),
        "rightShift" => NumberMath::right_shift(left, binary()?),
        "rightShiftUnsigned" => NumberMath::right_shift_unsigned(left, binary()?),
        "bitwiseNegate" => NumberMath::bitwise_negate(left),
        "unaryMinus" => NumberMath::unary_minus(left),
        "unaryPlus" => NumberMath::unary_plus(left),
        "toBigDecimal" => NumberMath::to_big_decimal(left),
        "toBigInteger" => NumberMath::to_big_integer(left),
        "isFloatingPoint" => Ok(DataValue::Bool(NumberMath::is_floating_point(left))),
        "isInteger" => Ok(DataValue::Bool(NumberMath::is_integer(left))),
        "isShort" => Ok(DataValue::Bool(NumberMath::is_short(left))),
        "isByte" => Ok(DataValue::Bool(NumberMath::is_byte(left))),
        "isLong" => Ok(DataValue::Bool(NumberMath::is_long(left))),
        "isBigDecimal" => Ok(DataValue::Bool(NumberMath::is_big_decimal(left))),
        "isBigInteger" => Ok(DataValue::Bool(NumberMath::is_big_integer(left))),
        "getMath" => Ok(DataValue::string(math_domain_name(
            convert::math_domain(left, binary()?).ok_or_else(|| {
                format!("number_math getMath received non-number operand in {id}")
            })?,
        ))),
        operation => return Err(format!("unsupported number_math operation: {operation}")),
    };
    Ok(result)
}

fn execute_concrete_number_math(
    implementation: &str,
    operation: &str,
    left: &DataValue,
    right: Option<&DataValue>,
) -> Result<Result<DataValue, qlexpress::exception::QLException>, String> {
    let binary = || right.ok_or_else(|| format!("number_math {operation} requires right operand"));
    let result = match (implementation, operation) {
        ("IntegerMath", "absImpl") => IntegerMath::abs_impl(left),
        ("IntegerMath", "addImpl") => IntegerMath::add_impl(left, binary()?),
        ("IntegerMath", "subtractImpl") => IntegerMath::subtract_impl(left, binary()?),
        ("IntegerMath", "multiplyImpl") => IntegerMath::multiply_impl(left, binary()?),
        ("IntegerMath", "divideImpl") => IntegerMath::divide_impl(left, binary()?),
        ("IntegerMath", "compareToImpl") => Ok(DataValue::Int(IntegerMath::compare_to_impl(
            left,
            binary()?,
        ))),
        ("IntegerMath", "orImpl") => IntegerMath::or_impl(left, binary()?),
        ("IntegerMath", "andImpl") => IntegerMath::and_impl(left, binary()?),
        ("IntegerMath", "xorImpl") => IntegerMath::xor_impl(left, binary()?),
        ("IntegerMath", "intDivImpl") => IntegerMath::int_div_impl(left, binary()?),
        ("IntegerMath", "modImpl") => IntegerMath::mod_impl(left, binary()?),
        ("IntegerMath", "remainderImpl") => IntegerMath::remainder_impl(left, binary()?),
        ("IntegerMath", "unaryMinusImpl") => IntegerMath::unary_minus_impl(left),
        ("IntegerMath", "unaryPlusImpl") => IntegerMath::unary_plus_impl(left),
        ("IntegerMath", "bitwiseNegateImpl") => IntegerMath::bitwise_negate_impl(left),
        ("IntegerMath", "leftShiftImpl") => IntegerMath::left_shift_impl(left, binary()?),
        ("IntegerMath", "rightShiftImpl") => IntegerMath::right_shift_impl(left, binary()?),
        ("IntegerMath", "rightShiftUnsignedImpl") => {
            IntegerMath::right_shift_unsigned_impl(left, binary()?)
        }
        ("LongMath", "absImpl") => LongMath::abs_impl(left),
        ("LongMath", "addImpl") => LongMath::add_impl(left, binary()?),
        ("LongMath", "subtractImpl") => LongMath::subtract_impl(left, binary()?),
        ("LongMath", "multiplyImpl") => LongMath::multiply_impl(left, binary()?),
        ("LongMath", "divideImpl") => LongMath::divide_impl(left, binary()?),
        ("LongMath", "compareToImpl") => {
            Ok(DataValue::Int(LongMath::compare_to_impl(left, binary()?)))
        }
        ("LongMath", "intDivImpl") => LongMath::int_div_impl(left, binary()?),
        ("LongMath", "remainderImpl") => LongMath::remainder_impl(left, binary()?),
        ("LongMath", "modImpl") => LongMath::mod_impl(left, binary()?),
        ("LongMath", "unaryMinusImpl") => LongMath::unary_minus_impl(left),
        ("LongMath", "unaryPlusImpl") => LongMath::unary_plus_impl(left),
        ("LongMath", "bitwiseNegateImpl") => LongMath::bitwise_negate_impl(left),
        ("LongMath", "orImpl") => LongMath::or_impl(left, binary()?),
        ("LongMath", "andImpl") => LongMath::and_impl(left, binary()?),
        ("LongMath", "bitAndImpl") => LongMath::bit_and_impl(left, binary()?),
        ("LongMath", "xorImpl") => LongMath::xor_impl(left, binary()?),
        ("LongMath", "leftShiftImpl") => LongMath::left_shift_impl(left, binary()?),
        ("LongMath", "rightShiftImpl") => LongMath::right_shift_impl(left, binary()?),
        ("LongMath", "rightShiftUnsignedImpl") => {
            LongMath::right_shift_unsigned_impl(left, binary()?)
        }
        ("BigIntegerMath", "absImpl") => BigIntegerMath::abs_impl(left),
        ("BigIntegerMath", "addImpl") => BigIntegerMath::add_impl(left, binary()?),
        ("BigIntegerMath", "subtractImpl") => BigIntegerMath::subtract_impl(left, binary()?),
        ("BigIntegerMath", "multiplyImpl") => BigIntegerMath::multiply_impl(left, binary()?),
        ("BigIntegerMath", "divideImpl") => BigIntegerMath::divide_impl(left, binary()?),
        ("BigIntegerMath", "compareToImpl") => Ok(DataValue::Int(BigIntegerMath::compare_to_impl(
            left,
            binary()?,
        ))),
        ("BigIntegerMath", "intDivImpl") => BigIntegerMath::int_div_impl(left, binary()?),
        ("BigIntegerMath", "modImpl") => BigIntegerMath::mod_impl(left, binary()?),
        ("BigIntegerMath", "remainderImpl") => BigIntegerMath::remainder_impl(left, binary()?),
        ("BigIntegerMath", "unaryMinusImpl") => BigIntegerMath::unary_minus_impl(left),
        ("BigIntegerMath", "unaryPlusImpl") => BigIntegerMath::unary_plus_impl(left),
        ("BigIntegerMath", "bitwiseNegateImpl") => BigIntegerMath::bitwise_negate_impl(left),
        ("BigIntegerMath", "orImpl") => BigIntegerMath::or_impl(left, binary()?),
        ("BigIntegerMath", "andImpl") => BigIntegerMath::and_impl(left, binary()?),
        ("BigIntegerMath", "xorImpl") => BigIntegerMath::xor_impl(left, binary()?),
        ("BigIntegerMath", "leftShiftImpl") => BigIntegerMath::left_shift_impl(left, binary()?),
        ("BigIntegerMath", "rightShiftImpl") => BigIntegerMath::right_shift_impl(left, binary()?),
        ("BigDecimalMath", "absImpl") => BigDecimalMath::abs_impl(left),
        ("BigDecimalMath", "addImpl") => BigDecimalMath::add_impl(left, binary()?),
        ("BigDecimalMath", "subtractImpl") => BigDecimalMath::subtract_impl(left, binary()?),
        ("BigDecimalMath", "multiplyImpl") => BigDecimalMath::multiply_impl(left, binary()?),
        ("BigDecimalMath", "divideImpl") => BigDecimalMath::divide_impl(left, binary()?),
        ("BigDecimalMath", "compareToImpl") => Ok(DataValue::Int(BigDecimalMath::compare_to_impl(
            left,
            binary()?,
        ))),
        ("BigDecimalMath", "unaryMinusImpl") => BigDecimalMath::unary_minus_impl(left),
        ("BigDecimalMath", "unaryPlusImpl") => BigDecimalMath::unary_plus_impl(left),
        ("BigDecimalMath", "remainderImpl") => BigDecimalMath::remainder_impl(left, binary()?),
        ("BigDecimalMath", "modImpl") => BigDecimalMath::mod_impl(left, binary()?),
        ("FloatingPointMath", "absImpl") => FloatingPointMath::abs_impl(left),
        ("FloatingPointMath", "addImpl") => FloatingPointMath::add_impl(left, binary()?),
        ("FloatingPointMath", "subtractImpl") => FloatingPointMath::subtract_impl(left, binary()?),
        ("FloatingPointMath", "multiplyImpl") => FloatingPointMath::multiply_impl(left, binary()?),
        ("FloatingPointMath", "divideImpl") => FloatingPointMath::divide_impl(left, binary()?),
        ("FloatingPointMath", "compareToImpl") => Ok(DataValue::Int(
            FloatingPointMath::compare_to_impl(left, binary()?),
        )),
        ("FloatingPointMath", "remainderImpl") => {
            FloatingPointMath::remainder_impl(left, binary()?)
        }
        ("FloatingPointMath", "modImpl") => FloatingPointMath::mod_impl(left, binary()?),
        ("FloatingPointMath", "unaryMinusImpl") => FloatingPointMath::unary_minus_impl(left),
        ("FloatingPointMath", "unaryPlusImpl") => FloatingPointMath::unary_plus_impl(left),
        _ => {
            return Err(format!(
                "unsupported concrete number_math operation: {implementation}.{operation}"
            ));
        }
    };
    Ok(result)
}

fn typed_number_to_data_value(number: &TypedNumber) -> Result<DataValue, String> {
    let parse_error = |error: &dyn std::fmt::Display| {
        format!(
            "invalid {} number value {:?}: {error}",
            number.number_type, number.value
        )
    };
    match number.number_type.as_str() {
        "byte" => number
            .value
            .parse::<i8>()
            .map(DataValue::Byte)
            .map_err(|error| parse_error(&error)),
        "short" => number
            .value
            .parse::<i16>()
            .map(DataValue::Short)
            .map_err(|error| parse_error(&error)),
        "int" => number
            .value
            .parse::<i32>()
            .map(DataValue::Int)
            .map_err(|error| parse_error(&error)),
        "long" => number
            .value
            .parse::<i64>()
            .map(DataValue::Long)
            .map_err(|error| parse_error(&error)),
        "float" => number
            .value
            .parse::<f32>()
            .map(DataValue::Float)
            .map_err(|error| parse_error(&error)),
        "double" => number
            .value
            .parse::<f64>()
            .map(DataValue::Double)
            .map_err(|error| parse_error(&error)),
        "bigint" => number
            .value
            .parse::<num_bigint::BigInt>()
            .map(DataValue::BigInt)
            .map_err(|error| parse_error(&error)),
        "bigdec" => Ok(DataValue::BigDec(number.value.clone())),
        number_type => Err(format!("unsupported number type: {number_type}")),
    }
}

fn math_domain_name(domain: MathDomain) -> &'static str {
    match domain {
        MathDomain::Integer => "IntegerMath",
        MathDomain::Long => "LongMath",
        MathDomain::FloatingPoint => "FloatingPointMath",
        MathDomain::BigInteger => "BigIntegerMath",
        MathDomain::BigDecimal => "BigDecimalMath",
    }
}

fn number_math_error_category(error: &qlexpress::exception::QLException) -> &'static str {
    if error.error_code() == "java.lang.NumberFormatException" {
        "NUMBER_FORMAT_EXCEPTION"
    } else if error.reason().starts_with("Cannot use")
        || error
            .reason()
            .starts_with("Shift distance must be an integral type")
    {
        "UNSUPPORTED_OPERATION"
    } else {
        "ARITHMETIC_EXCEPTION"
    }
}

fn build_options(options: &DifferentialOptions) -> QLOptions {
    let mut builder: QLOptionsBuilder = QLOptions::builder();
    if let Some(value) = options.precise {
        builder = builder.precise(value);
    }
    if let Some(value) = options.cache {
        builder = builder.cache(value);
    }
    if let Some(value) = options.avoid_null_pointer {
        builder = builder.avoid_null_pointer(value);
    }
    if let Some(value) = options.max_arr_length {
        builder = builder.max_arr_length(value);
    }
    if let Some(value) = options.trace_expression {
        builder = builder.trace_expression(value);
    }
    if let Some(value) = options.short_circuit_disable {
        builder = builder.short_circuit_disable(value);
    }
    if let Some(value) = options.timeout_millis {
        builder = builder.timeout_millis(value);
    }
    builder.build()
}

fn json_to_data_value(value: serde_json::Value) -> Result<DataValue, String> {
    match value {
        serde_json::Value::Null => Ok(DataValue::Null),
        serde_json::Value::Bool(value) => Ok(DataValue::Bool(value)),
        serde_json::Value::Number(value) => {
            if let Some(value) = value.as_i64() {
                if let Ok(value) = i32::try_from(value) {
                    Ok(DataValue::Int(value))
                } else {
                    Ok(DataValue::Long(value))
                }
            } else if let Some(value) = value.as_f64() {
                Ok(DataValue::Double(value))
            } else {
                Err(format!("unsupported JSON number: {value}"))
            }
        }
        serde_json::Value::String(value) => Ok(DataValue::string(value)),
        serde_json::Value::Array(values) => values
            .into_iter()
            .map(json_to_data_value)
            .collect::<Result<Vec<_>, _>>()
            .map(DataValue::list),
        serde_json::Value::Object(values) => {
            let entries = values
                .into_iter()
                .map(|(key, value)| {
                    json_to_data_value(value).map(|value| (DataValue::string(key), value))
                })
                .collect::<Result<Vec<_>, _>>()?;
            Ok(DataValue::map(IndexMap::from_entries(entries)))
        }
    }
}
