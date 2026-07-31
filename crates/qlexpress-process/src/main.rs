//! 一次性 QlExpress Rust 隔离进程执行器。
#![warn(missing_docs)]

use std::collections::HashMap;
use std::io::Read;

use qlexpress::{DataValue, Express4Runner, QLOptions, SandboxProfile};
use qlexpress_process::{os_limits, WorkerRequest, WorkerResponse};
use serde_json::{Map, Number, Value};

fn main() {
    let response = run().unwrap_or_else(|reason| WorkerResponse::failure("WORKER_ERROR", reason));
    match serde_json::to_writer(std::io::stdout(), &response) {
        Ok(()) => {}
        Err(error) => {
            eprintln!("failed to encode worker response: {error}");
            std::process::exit(2);
        }
    }
}

fn run() -> Result<WorkerResponse, String> {
    let os_limits = os_limits::limits_from_env();
    os_limits::apply(&os_limits)?;

    let mut input = Vec::new();
    std::io::stdin()
        .take(1024 * 1024)
        .read_to_end(&mut input)
        .map_err(|error| format!("failed to read worker request: {error}"))?;
    let request: WorkerRequest =
        serde_json::from_slice(&input).map_err(|error| format!("invalid request: {error}"))?;

    let context = request
        .context
        .into_iter()
        .map(|(key, value)| json_to_data(value).map(|value| (key, value)))
        .collect::<Result<HashMap<_, _>, _>>()?;
    let mut profile = SandboxProfile::secure();
    profile.tenant_id = request.tenant_id;
    if let Some(resource_limits) = request.resource_limits {
        profile.limits = resource_limits;
    }
    profile.compile_cache.enabled = false;

    match Express4Runner::new().execute_checked(
        &request.script,
        context,
        &QLOptions::default(),
        &profile,
    ) {
        Ok(result) => Ok(WorkerResponse::success(data_to_json(result.result())?)),
        Err(error) => Ok(WorkerResponse::failure(error.error_code(), error.reason())),
    }
}

fn json_to_data(value: Value) -> Result<DataValue, String> {
    match value {
        Value::Null => Ok(DataValue::Null),
        Value::Bool(value) => Ok(DataValue::Bool(value)),
        Value::Number(value) => {
            if let Some(value) = value.as_i64() {
                Ok(DataValue::Long(value))
            } else if let Some(value) = value.as_f64() {
                Ok(DataValue::Double(value))
            } else {
                Err("unsupported JSON number".to_string())
            }
        }
        Value::String(value) => Ok(DataValue::string(value)),
        Value::Array(values) => values
            .into_iter()
            .map(json_to_data)
            .collect::<Result<Vec<_>, _>>()
            .map(DataValue::list),
        Value::Object(values) => {
            let entries = values
                .into_iter()
                .map(|(key, value)| {
                    json_to_data(value).map(|value| (DataValue::string(key), value))
                })
                .collect::<Result<Vec<_>, _>>()?;
            Ok(DataValue::map(
                qlexpress::runtime::data::index_map::IndexMap::from_entries(entries),
            ))
        }
    }
}

fn data_to_json(value: &DataValue) -> Result<Value, String> {
    match value {
        DataValue::Null => Ok(Value::Null),
        DataValue::Bool(value) => Ok(Value::Bool(*value)),
        DataValue::Byte(value) => Ok(Value::Number(Number::from(*value))),
        DataValue::Short(value) => Ok(Value::Number(Number::from(*value))),
        DataValue::Int(value) => Ok(Value::Number(Number::from(*value))),
        DataValue::Long(value) => Ok(Value::Number(Number::from(*value))),
        DataValue::Float(value) => Number::from_f64(f64::from(*value))
            .map(Value::Number)
            .ok_or_else(|| "non-finite float output".to_string()),
        DataValue::Double(value) => Number::from_f64(*value)
            .map(Value::Number)
            .ok_or_else(|| "non-finite double output".to_string()),
        DataValue::BigInt(value) => Ok(Value::String(value.to_string())),
        DataValue::BigDec(value) => Ok(Value::String(value.clone())),
        DataValue::Char(value) => char::from_u32(u32::from(*value))
            .map(|value| Value::String(value.to_string()))
            .ok_or_else(|| {
                "worker JSON output cannot represent an unpaired UTF-16 surrogate".to_string()
            }),
        DataValue::Str(value) => value.to_rust_string().map(Value::String).ok_or_else(|| {
            "worker JSON output cannot represent an unpaired UTF-16 surrogate".to_string()
        }),
        DataValue::List(values) => values
            .borrow()
            .iter()
            .map(data_to_json)
            .collect::<Result<Vec<_>, _>>()
            .map(Value::Array),
        DataValue::Array(values) => values
            .borrow()
            .iter()
            .map(data_to_json)
            .collect::<Result<Vec<_>, _>>()
            .map(Value::Array),
        DataValue::Map(values) => {
            let mut output = Map::new();
            for (key, value) in values.borrow().entries() {
                let DataValue::Str(key) = key else {
                    return Err("worker JSON output requires string map keys".to_string());
                };
                let key = key.to_rust_string().ok_or_else(|| {
                    "worker JSON object key cannot represent an unpaired UTF-16 surrogate"
                        .to_string()
                })?;
                output.insert(key, data_to_json(value)?);
            }
            Ok(Value::Object(output))
        }
        DataValue::Lambda(_) | DataValue::Object(_) => {
            Err("worker cannot serialize lambda or native object output".to_string())
        }
    }
}
