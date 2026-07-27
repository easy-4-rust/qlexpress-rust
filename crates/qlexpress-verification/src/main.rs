//! qlexpress-rust 生产验收命令行工具。

mod business_host;
mod canary;
mod concurrency;
mod differential;
mod load;
mod normalization;
mod replay;
mod security_fuzz;

use std::env;
use std::path::PathBuf;
use std::process::ExitCode;
use std::time::Duration;

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("verification failed: {error}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<(), String> {
    let mut args = env::args().skip(1);
    let command = args.next().ok_or_else(usage)?;
    match command.as_str() {
        "differential" => {
            let corpus = required_path(&mut args, "corpus")?;
            let output = required_path(&mut args, "output")?;
            differential::run(&corpus, &output)
        }
        "replay" => {
            let java_repo = required_path(&mut args, "java repository")?;
            replay::run(&java_repo)
        }
        "concurrency" => {
            let threads = optional_usize(args.next(), 8, "threads")?;
            let iterations = optional_usize(args.next(), 2_000, "iterations")?;
            concurrency::run(threads, iterations)
        }
        "load" => {
            let seconds = optional_u64(args.next(), 15, "duration seconds")?;
            let threads = optional_usize(args.next(), 8, "threads")?;
            load::run(Duration::from_secs(seconds), threads)
        }
        "security-fuzz" => {
            let cases = optional_usize(args.next(), 25_000, "cases")?;
            security_fuzz::run(cases)
        }
        "business-host" => business_host::run(),
        "canary" => canary::run(),
        _ => Err(usage()),
    }
}

fn required_path(args: &mut impl Iterator<Item = String>, name: &str) -> Result<PathBuf, String> {
    args.next()
        .map(PathBuf::from)
        .ok_or_else(|| format!("missing {name}\n{}", usage()))
}

fn optional_usize(value: Option<String>, default: usize, name: &str) -> Result<usize, String> {
    value
        .map(|raw| {
            raw.parse::<usize>()
                .map_err(|error| format!("invalid {name} '{raw}': {error}"))
        })
        .unwrap_or(Ok(default))
}

fn optional_u64(value: Option<String>, default: u64, name: &str) -> Result<u64, String> {
    value
        .map(|raw| {
            raw.parse::<u64>()
                .map_err(|error| format!("invalid {name} '{raw}': {error}"))
        })
        .unwrap_or(Ok(default))
}

fn usage() -> String {
    [
        "usage:",
        "  qlexpress-verification differential <corpus.jsonl> <output.jsonl>",
        "  qlexpress-verification replay <java-repository>",
        "  qlexpress-verification concurrency [threads] [iterations]",
        "  qlexpress-verification load [duration-seconds] [threads]",
        "  qlexpress-verification security-fuzz [cases]",
        "  qlexpress-verification business-host",
        "  qlexpress-verification canary",
    ]
    .join("\n")
}
