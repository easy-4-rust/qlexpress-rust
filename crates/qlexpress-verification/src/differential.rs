//! Rust 侧差分语料执行器。

use std::cell::RefCell;
use std::collections::HashMap;
use std::fs::File;
use std::io::{BufRead, BufReader, BufWriter, Write};
use std::path::Path;
use std::rc::Rc;

use qlexpress::aparser::operator_factory::OperatorFactory;
use qlexpress::aparser::parser_operator_manager::{OpType, ParserOperatorManager};
use qlexpress::aparser::{ExistStack, ExistVarStack, MacroDefine};
use qlexpress::api::{BatchAddFunctionResult, QLFunctionalVarargs};
use qlexpress::exception::pure_err_reporter::PureErrReporter;
use qlexpress::exception::{ExceptionType, UserDefineException};
use qlexpress::init_options::InitOptions;
use qlexpress::lsp::{Diagnostic, Position, Range};
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
use qlexpress::security::{
    NativeMember, StrategyBlackList, StrategyIsolation, StrategyOpen, StrategyWhiteList,
};
use qlexpress::utils::ql_string_utils::QLStringUtils;
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
    batch_add_function_result: Option<BatchAddFunctionResultInvocation>,
    #[serde(default)]
    ql_functional_varargs: Option<QLFunctionalVarargsInvocation>,
    #[serde(default)]
    lsp_position: Option<LspPositionInvocation>,
    #[serde(default)]
    lsp_range: Option<LspRangeInvocation>,
    #[serde(default)]
    lsp_diagnostic: Option<LspDiagnosticInvocation>,
    #[serde(default)]
    exist_stack: Option<ExistStackInvocation>,
    #[serde(default)]
    macro_define: Option<MacroDefineInvocation>,
    #[serde(default)]
    user_define_exception: Option<UserDefineExceptionInvocation>,
    #[serde(default)]
    security_strategies: Option<SecurityStrategiesInvocation>,
    #[serde(default)]
    ql_string_utils: Option<QLStringUtilsInvocation>,
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
struct BatchAddFunctionResultInvocation {
    scenario: String,
}

#[derive(Debug, Deserialize)]
struct QLFunctionalVarargsInvocation {
    scenario: String,
}

#[derive(Debug, Deserialize)]
struct LspPositionInvocation {
    scenario: String,
}

#[derive(Debug, Deserialize)]
struct LspRangeInvocation {
    scenario: String,
}

#[derive(Debug, Deserialize)]
struct LspDiagnosticInvocation {
    scenario: String,
}

#[derive(Debug, Deserialize)]
struct ExistStackInvocation {
    scenario: String,
}

#[derive(Debug, Deserialize)]
struct MacroDefineInvocation {
    scenario: String,
}

#[derive(Debug, Deserialize)]
struct UserDefineExceptionInvocation {
    scenario: String,
}

#[derive(Debug, Deserialize)]
struct SecurityStrategiesInvocation {
    scenario: String,
}

#[derive(Debug, Deserialize)]
struct QLStringUtilsInvocation {
    scenario: String,
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
    if let Some(invocation) = case.batch_add_function_result {
        return execute_batch_add_function_result(case.id, invocation);
    }
    if let Some(invocation) = case.ql_functional_varargs {
        return execute_ql_functional_varargs(case.id, invocation);
    }
    if let Some(invocation) = case.lsp_position {
        return execute_lsp_position(case.id, invocation);
    }
    if let Some(invocation) = case.lsp_range {
        return execute_lsp_range(case.id, invocation);
    }
    if let Some(invocation) = case.lsp_diagnostic {
        return execute_lsp_diagnostic(case.id, invocation);
    }
    if let Some(invocation) = case.exist_stack {
        return execute_exist_stack(case.id, invocation);
    }
    if let Some(invocation) = case.macro_define {
        return execute_macro_define(case.id, invocation);
    }
    if let Some(invocation) = case.user_define_exception {
        return execute_user_define_exception(case.id, invocation);
    }
    if let Some(invocation) = case.security_strategies {
        return execute_security_strategies(case.id, invocation);
    }
    if let Some(invocation) = case.ql_string_utils {
        return execute_ql_string_utils(case.id, invocation);
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

include!("differential/api_value_objects.rs");
include!("differential/security_and_lsp.rs");
include!("differential/exception_table.rs");
include!("differential/runtime_and_stack.rs");
include!("differential/delegate_context.rs");
include!("differential/operator_manager.rs");
include!("differential/number_math.rs");
