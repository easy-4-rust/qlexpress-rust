//! 直接回放 Java 基线仓库的 151 个 independent `.ql` 脚本。

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use qlexpress::ql_options::{QLOptions, QLOptionsBuilder};

#[path = "../../qlexpress/tests/alignment_util/mod.rs"]
mod alignment_util;

/// 对应 Java: 无（Rust 原生适配）。

pub fn run(java_repo: &Path) -> Result<(), String> {
    let root = java_repo.join("src/test/resources/testsuite/independent");
    if !root.is_dir() {
        return Err(format!("independent suite not found: {}", root.display()));
    }
    let mut files = Vec::new();
    collect_ql_files(&root, &mut files)?;
    files.sort();
    let mut failures = Vec::new();
    for file in &files {
        if let Err(error) = replay_file(file) {
            failures.push(format!("{}: {error}", file.display()));
        }
    }
    if !failures.is_empty() {
        return Err(format!(
            "replay failed: {} of {}\n{}",
            failures.len(),
            files.len(),
            failures.join("\n")
        ));
    }
    println!(
        "{{\"source\":\"{}\",\"scripts\":{},\"passed\":{},\"failed\":0}}",
        root.display(),
        files.len(),
        files.len()
    );
    Ok(())
}

fn collect_ql_files(directory: &Path, files: &mut Vec<PathBuf>) -> Result<(), String> {
    for entry in fs::read_dir(directory)
        .map_err(|error| format!("read directory {}: {error}", directory.display()))?
    {
        let path = entry
            .map_err(|error| format!("read directory entry: {error}"))?
            .path();
        if path.is_dir() {
            collect_ql_files(&path, files)?;
        } else if path.extension().is_some_and(|extension| extension == "ql") {
            files.push(path);
        }
    }
    Ok(())
}

fn replay_file(path: &Path) -> Result<(), String> {
    let script = fs::read_to_string(path).map_err(|error| format!("read script: {error}"))?;
    let options = parse_options(&script);
    let expected_error = quoted_option(&script, "errCode");
    let no_return = script.contains("\"noReturn\": true");
    let result = alignment_util::suite_runner().execute(&script, HashMap::new(), &options);
    match (expected_error, result) {
        (Some(expected), Err(error)) if error.error_code() == expected => Ok(()),
        (Some(expected), Err(error)) => Err(format!(
            "expected error {expected}, got {} ({error})",
            error.error_code()
        )),
        (Some(expected), Ok(value)) => Err(format!(
            "expected error {expected}, got value {:?}",
            value.result()
        )),
        (None, Err(error)) => Err(format!("unexpected error {} ({error})", error.error_code())),
        (None, Ok(value)) if no_return && !value.result().is_null() => {
            Err(format!("expected null, got {:?}", value.result()))
        }
        (None, Ok(_)) => Ok(()),
    }
}

fn parse_options(script: &str) -> QLOptions {
    let mut builder: QLOptionsBuilder = QLOptions::builder();
    if script.contains(".avoidNullPointer(true)") {
        builder = builder.avoid_null_pointer(true);
    }
    if script.contains(".precise(true)") {
        builder = builder.precise(true);
    }
    if script.contains(".shortCircuitDisable(true)") {
        builder = builder.short_circuit_disable(true);
    }
    if let Some(value) = numeric_builder_option(script, "maxArrLength") {
        builder = builder.max_arr_length(value as i32);
    }
    if let Some(value) = numeric_builder_option(script, "timeoutMillis") {
        builder = builder.timeout_millis(value);
    }
    builder.build()
}

fn quoted_option<'a>(script: &'a str, name: &str) -> Option<&'a str> {
    let marker = format!("\"{name}\"");
    let tail = script.split_once(&marker)?.1;
    let tail = tail.split_once(':')?.1.trim_start();
    let tail = tail.strip_prefix('"')?;
    tail.split_once('"').map(|(value, _)| value)
}

fn numeric_builder_option(script: &str, method: &str) -> Option<i64> {
    let marker = format!(".{method}(");
    let tail = script.split_once(&marker)?.1;
    let number = tail.split_once(')')?.0.trim();
    number.parse().ok()
}
