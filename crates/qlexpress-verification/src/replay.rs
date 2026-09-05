//! 直接回放 Java 基线仓库的官方 `.ql` 测试脚本。
//!
//! 覆盖 `testsuite/independent/`（纯脚本，历史 151 个）与
//! `testsuite/java/`（需要 Java 测试 fixture 宿主对象，77 个）。两类
//! 脚本共用同一套回放语义：正常脚本断言执行成功，带 `errCode` 标注
//! 的错误脚本逐条比对 Rust 抛出的错误码与 Java 标注是否一致。

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use qlexpress::ql_options::{QLOptions, QLOptionsBuilder};

#[path = "../../qlexpress/tests/alignment_util/mod.rs"]
mod alignment_util;

/// 对应 Java: 无（Rust 原生适配）。
pub fn run(java_repo: &Path) -> Result<(), String> {
    let suites = [
        (
            "independent",
            java_repo.join("src/test/resources/testsuite/independent"),
        ),
        (
            "java-fixtures",
            java_repo.join("src/test/resources/testsuite/java"),
        ),
    ];
    let mut total_files = 0usize;
    let mut all_failures = Vec::new();
    for (name, root) in suites {
        if !root.is_dir() {
            return Err(format!("{name} suite not found: {}", root.display()));
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
        println!(
            "{{\"suite\":\"{name}\",\"source\":\"{}\",\"scripts\":{},\"failed\":{}}}",
            root.display(),
            files.len(),
            failures.len()
        );
        total_files += files.len();
        all_failures.extend(failures);
    }
    if !all_failures.is_empty() {
        return Err(format!(
            "replay failed: {} of {}\n{}",
            all_failures.len(),
            total_files,
            all_failures.join("\n")
        ));
    }
    println!(
        "{{\"source\":\"all-suites\",\"scripts\":{},\"passed\":{},\"failed\":0}}",
        total_files, total_files
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
    // java/property 的 private_member_attr_access_* 脚本在头部标注
    // `InitOptions.builder().allowPrivateAccess(true)`；此时须以开启私有
    // 访问的 Runner 回放（suite_runner 默认关闭以保持 FIELD_NOT_FOUND
    // 语义，见 private_member_set_not_accessible.ql）。
    let allow_private = script.contains(".allowPrivateAccess(true)");
    let runner = if allow_private {
        alignment_util::suite_runner_with_init_options(
            alignment_util::jdk_init_options_with_private_access(),
        )
    } else {
        alignment_util::suite_runner()
    };
    let result = runner.execute(&script, HashMap::new(), &options);
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
