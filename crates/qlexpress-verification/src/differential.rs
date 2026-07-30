//! Rust 侧差分语料执行器。

use std::collections::HashMap;
use std::fs::File;
use std::io::{BufRead, BufReader, BufWriter, Write};
use std::path::Path;

use qlexpress::init_options::InitOptions;
use qlexpress::ql_options::{QLOptions, QLOptionsBuilder};
use qlexpress::runtime::data::index_map::IndexMap;
use qlexpress::runtime::value::DataValue;
use qlexpress::security::ql_security_strategy::QLSecurityStrategy;
use qlexpress::Express4Runner;
use serde::{Deserialize, Serialize};

use crate::normalization::normalize;

#[derive(Debug, Deserialize)]
struct DifferentialCase {
    id: String,
    script: String,
    #[serde(default)]
    context: serde_json::Map<String, serde_json::Value>,
    #[serde(default)]
    options: DifferentialOptions,
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
    match runner.execute(&case.script, context, &options) {
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
            normalized: None,
            error_code: Some(error.error_code().to_string()),
            line: Some(error.line_no()),
            column: Some(error.col_no()),
            trace_count: 0,
        }),
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
        serde_json::Value::String(value) => Ok(DataValue::Str(value)),
        serde_json::Value::Array(values) => values
            .into_iter()
            .map(json_to_data_value)
            .collect::<Result<Vec<_>, _>>()
            .map(DataValue::list),
        serde_json::Value::Object(values) => {
            let entries = values
                .into_iter()
                .map(|(key, value)| {
                    json_to_data_value(value).map(|value| (DataValue::Str(key), value))
                })
                .collect::<Result<Vec<_>, _>>()?;
            Ok(DataValue::map(IndexMap::from_entries(entries)))
        }
    }
}
