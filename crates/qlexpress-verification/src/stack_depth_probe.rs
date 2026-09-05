//! Stack depth and sandbox budget boundary probes.
//!
//! Three independent probe functions that exercise the QLExpress engine's
//! resource limits without depending on any business script:
//!
//! 1. Recursive function call depth -- which N triggers
//!    `SANDBOX_CALL_DEPTH_EXCEEDED` or `SANDBOX_FUEL_EXCEEDED`?
//! 2. Operand stack pressure -- does a deeply nested expression trigger
//!    `OPERAND_STACK_OVERFLOW`, or does the compiler always compute the
//!    correct `max_stack_size`?
//! 3. Nested try-catch -- does deeply nested error handling trigger any
//!    budget limit?
//!
//! Run via the `stack-depth-probe` verification subcommand.
//!
//! Probes 2 and 3 use subprocess isolation because deeply nested expressions
//! or try-catch blocks can overflow the Rust process's own call stack during
//! parsing, causing a hard abort that `catch_unwind` cannot intercept.

use std::collections::HashMap;
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::process::Command;
use std::time::Instant;

use qlexpress::ql_options::QLOptions;
use qlexpress::security::{
    CancellationToken, CapabilityPolicy, CompileCachePolicy, ResourceLimits, SandboxProfile,
};
use qlexpress::Express4Runner;

/// Result of a single probe run at a given depth N.
#[derive(serde::Serialize)]
struct ProbeRow {
    /// The nesting depth tested.
    n: usize,
    /// `Ok` or the stable error code emitted by the engine.
    outcome: String,
    /// Human-readable detail (value or reason).
    detail: String,
    /// Wall-clock elapsed microseconds.
    elapsed_us: u64,
}

/// Top-level output for a single probe category.
#[derive(serde::Serialize)]
struct ProbeReport {
    probe: String,
    description: String,
    results: Vec<ProbeRow>,
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Build a [`SandboxProfile`] with generous defaults, optionally overriding
/// specific [`ResourceLimits`] fields.
fn sandbox_profile(overrides: impl FnOnce(&mut ResourceLimits)) -> SandboxProfile {
    let mut limits = ResourceLimits {
        max_source_bytes: 1024 * 1024,
        max_tokens: 200_000,
        max_ast_depth: 10_000,
        max_ast_nodes: 500_000,
        max_instructions: 500_000,
        max_fuel: 10_000_000,
        max_call_depth: 10_000,
        timeout_millis: 60_000,
        ..ResourceLimits::default()
    };
    overrides(&mut limits);
    SandboxProfile {
        limits,
        capability_policy: CapabilityPolicy::deny_all(),
        compile_cache: CompileCachePolicy {
            enabled: false,
            ..CompileCachePolicy::default()
        },
        tenant_id: "probe".to_string(),
        cancellation_token: CancellationToken::new(),
        ..SandboxProfile::default()
    }
}

/// Execute a script through `execute_checked` and return the outcome as
/// `(code, detail)` pair.  Catches unwinding panics (e.g. from `assert!`
/// in the operand stack).  Hard aborts (process stack overflow) cannot
/// be caught; use subprocess isolation for those paths.
///
/// `QLException` is intentionally large (architectural choice in qlexpress).
#[allow(clippy::result_large_err)]
fn run_checked(
    runner: &Express4Runner,
    script: &str,
    profile: &SandboxProfile,
) -> (String, String) {
    let caught = catch_unwind(AssertUnwindSafe(|| {
        runner.execute_checked(script, HashMap::new(), &QLOptions::default(), profile)
    }));
    match caught {
        Ok(Ok(ql_result)) => {
            let val = format!("{:?}", ql_result.result());
            ("Ok".to_string(), val)
        }
        Ok(Err(err)) => {
            let code = err.error_code().to_string();
            let detail = err.reason().to_string();
            (code, detail)
        }
        Err(_) => (
            "PANIC".to_string(),
            "process panic (likely assertion failure in operand stack)".to_string(),
        ),
    }
}

/// Run a script in a subprocess using the `stack-depth-probe-single`
/// subcommand.  The script is passed via the `STACK_PROBE_SCRIPT`
/// environment variable.
///
/// Exit code 0 = Ok, 1 = engine error, other = crash (e.g. stack overflow).
fn run_in_subprocess(script: &str) -> (String, String) {
    let exe = std::env::current_exe().unwrap_or_else(|_| "qlexpress-verification".into());
    let output = Command::new(&exe)
        .arg("stack-depth-probe-single")
        .env("STACK_PROBE_SCRIPT", script)
        .output();
    match output {
        Ok(out) => {
            let stdout = String::from_utf8_lossy(&out.stdout).trim().to_string();
            let stderr = String::from_utf8_lossy(&out.stderr).trim().to_string();
            if out.status.success() {
                ("Ok".to_string(), stdout)
            } else if stderr.contains("stack overflow") || stderr.contains("overflowed") {
                (
                    "PROCESS_STACK_OVERFLOW".to_string(),
                    "Rust process call stack overflow during parsing".to_string(),
                )
            } else if stdout.is_empty() {
                (
                    format!("EXIT_{}", out.status.code().unwrap_or(-1)),
                    if stderr.is_empty() {
                        "subprocess failed with no output".to_string()
                    } else {
                        stderr
                    },
                )
            } else {
                // Try to parse {"code":"...","detail":"..."} from stdout.
                if let Ok(v) = serde_json::from_str::<serde_json::Value>(&stdout) {
                    let code = v["code"].as_str().unwrap_or("UNKNOWN").to_string();
                    let detail = v["detail"].as_str().unwrap_or("").to_string();
                    (code, detail)
                } else {
                    (stdout, stderr)
                }
            }
        }
        Err(e) => (
            "SPAWN_FAILED".to_string(),
            format!("failed to spawn subprocess: {e}"),
        ),
    }
}

// ---------------------------------------------------------------------------
// Probe 1: Recursive function call depth
// ---------------------------------------------------------------------------

/// Generate a QLExpress script that defines a recursive function and calls it
/// with depth `n`.
///
/// ```text
/// function depth(n) { if (n <= 0) { return 0; }; return depth(n - 1) + 1; }; depth(N)
/// ```
fn recursive_script(n: usize) -> String {
    format!(
        "function depth(n) {{ if (n <= 0) {{ return 0; }}; return depth(n - 1) + 1; }}; depth({n})"
    )
}

/// Probe which recursive depth triggers `SANDBOX_CALL_DEPTH_EXCEEDED`,
/// `SANDBOX_FUEL_EXCEEDED`, or `SANDBOX_DEADLINE_EXCEEDED`.
///
/// Runs with the default `max_call_depth` (128) and a generous fuel budget
/// so the call depth limit is hit first.
pub fn probe_function_call_depth(max_depth: usize) -> Result<(), String> {
    let runner = Express4Runner::new();
    let profile = sandbox_profile(|limits| {
        limits.max_call_depth = 128;
        limits.max_fuel = 10_000_000;
        limits.timeout_millis = 30_000;
        limits.max_ast_depth = 10_000;
    });

    let depths: Vec<usize> = choose_depths(max_depth, &[10, 50, 100, 128, 200, 500, 1000, 5000]);
    let mut rows = Vec::new();

    for &n in &depths {
        let script = recursive_script(n);
        let start = Instant::now();
        let (outcome, detail) = run_checked(&runner, &script, &profile);
        let elapsed_us = start.elapsed().as_micros() as u64;
        rows.push(ProbeRow {
            n,
            outcome,
            detail,
            elapsed_us,
        });
    }

    let report = ProbeReport {
        probe: "function_call_depth".to_string(),
        description: "Recursive function depth; default max_call_depth=128. \
             Measures which N triggers SANDBOX_CALL_DEPTH_EXCEEDED / \
             SANDBOX_FUEL_EXCEEDED / SANDBOX_DEADLINE_EXCEEDED."
            .to_string(),
        results: rows,
    };
    println!(
        "{}",
        serde_json::to_string_pretty(&report).map_err(|e| e.to_string())?
    );
    Ok(())
}

// ---------------------------------------------------------------------------
// Probe 2: Operand stack pressure (subprocess isolated)
// ---------------------------------------------------------------------------

/// Generate a deeply nested arithmetic expression:
///
/// ```text
/// (((...(1 + 1) + 1) + 1)... + 1)
/// ```
///
/// Depth `n` produces `n` nested additions. The compiler must compute
/// `max_stack_size >= 2` for all of these (binary operator needs 2 slots).
fn nested_addition_script(n: usize) -> String {
    let mut script = String::new();
    for _ in 0..n.saturating_sub(1) {
        script.push('(');
    }
    script.push('1');
    for i in 1..n {
        script.push_str(&format!(" + {i})"));
    }
    script
}

/// Probe whether deeply nested arithmetic expressions trigger
/// `OPERAND_STACK_OVERFLOW`.  Uses subprocess isolation because very large
/// depths overflow the Rust process's own call stack during parsing.
pub fn probe_operand_stack_depth(max_depth: usize) -> Result<(), String> {
    let depths: Vec<usize> = choose_depths(
        max_depth,
        &[10, 50, 100, 105, 110, 115, 120, 130, 150, 200, 500],
    );
    let mut rows = Vec::new();
    let mut hit_process_limit = false;

    for &n in &depths {
        if hit_process_limit {
            rows.push(ProbeRow {
                n,
                outcome: "SKIPPED".to_string(),
                detail: "skipped: previous depth overflowed the Rust process call stack"
                    .to_string(),
                elapsed_us: 0,
            });
            continue;
        }
        let script = nested_addition_script(n);
        let start = Instant::now();
        let (outcome, detail) = run_in_subprocess(&script);
        let elapsed_us = start.elapsed().as_micros() as u64;
        if outcome == "PROCESS_STACK_OVERFLOW" {
            hit_process_limit = true;
        }
        rows.push(ProbeRow {
            n,
            outcome,
            detail,
            elapsed_us,
        });
    }

    let report = ProbeReport {
        probe: "operand_stack_depth".to_string(),
        description: "Deeply nested arithmetic expression (((...(1+1)+1)...)). \
             Tests whether the compiler's max_stack_size calculation is \
             correct. If OPERAND_STACK_OVERFLOW never fires before the \
             Rust process call stack overflows, the compiler is proven \
             correct for this class of expressions. Uses subprocess \
             isolation to safely detect process-level stack overflow."
            .to_string(),
        results: rows,
    };
    println!(
        "{}",
        serde_json::to_string_pretty(&report).map_err(|e| e.to_string())?
    );
    Ok(())
}

// ---------------------------------------------------------------------------
// Probe 3: Nested try-catch (subprocess isolated)
// ---------------------------------------------------------------------------

/// Generate a script with `n` nested try-catch blocks:
///
/// ```text
/// try { try { try { ... throw "err" ... } catch (e) { 1 } }
/// catch (e) { 2 } } catch (e) { 3 }
/// ```
///
/// Uses `catch (e)` (without explicit Exception type) to avoid
/// CLASS_NOT_FOUND in the sandbox.  Each try-catch adds ~4 AST depth
/// levels.  At depth 100 that is ~400 AST depth -- well within default
/// sandbox limits -- but the Rust parser's recursive descent can
/// overflow the process call stack.
fn nested_try_catch_script(n: usize) -> String {
    let mut script = String::new();
    for _ in 0..n {
        script.push_str("try { ");
    }
    script.push_str("throw \"err\"");
    for i in 0..n {
        script.push_str(&format!(" }} catch (e) {{ {} }}", i + 1));
    }
    script
}

/// Probe whether nested try-catch blocks trigger any sandbox budget limit.
/// Uses subprocess isolation because the parser's recursive descent can
/// overflow the Rust process call stack at large nesting depths.
pub fn probe_nested_try_catch(max_depth: usize) -> Result<(), String> {
    let depths: Vec<usize> = choose_depths(max_depth, &[5, 10, 20, 50, 100, 105, 120, 200, 500]);
    let mut rows = Vec::new();
    let mut hit_process_limit = false;

    for &n in &depths {
        if hit_process_limit {
            rows.push(ProbeRow {
                n,
                outcome: "SKIPPED".to_string(),
                detail: "skipped: previous depth overflowed the Rust process call stack"
                    .to_string(),
                elapsed_us: 0,
            });
            continue;
        }
        let script = nested_try_catch_script(n);
        let start = Instant::now();
        let (outcome, detail) = run_in_subprocess(&script);
        let elapsed_us = start.elapsed().as_micros() as u64;
        if outcome == "PROCESS_STACK_OVERFLOW" {
            hit_process_limit = true;
        }
        rows.push(ProbeRow {
            n,
            outcome,
            detail,
            elapsed_us,
        });
    }

    let report = ProbeReport {
        probe: "nested_try_catch".to_string(),
        description: "Nested try-catch blocks with throw at the center. \
             Measures AST depth, token count, and call depth consumption. \
             Uses subprocess isolation for safety."
            .to_string(),
        results: rows,
    };
    println!(
        "{}",
        serde_json::to_string_pretty(&report).map_err(|e| e.to_string())?
    );
    Ok(())
}

// ---------------------------------------------------------------------------
// Subprocess single-run helper
// ---------------------------------------------------------------------------

/// Single-run entry point for subprocess-isolated probes.
///
/// Reads the script from the `STACK_PROBE_SCRIPT` environment variable,
/// executes it with generous sandbox limits, and prints the result to stdout.
///
/// Exit codes:
/// - 0: Ok (result printed to stdout)
/// - 1: Engine error (JSON `{"code":"...","detail":"..."}` to stdout)
pub fn run_single_probe() -> Result<(), String> {
    let script = std::env::var("STACK_PROBE_SCRIPT")
        .map_err(|_| "STACK_PROBE_SCRIPT environment variable not set".to_string())?;
    let runner = Express4Runner::new();
    let profile = sandbox_profile(|limits| {
        limits.max_call_depth = 10_000;
        limits.max_fuel = 100_000_000;
        limits.max_ast_depth = 100_000;
        limits.max_ast_nodes = 1_000_000;
        limits.max_instructions = 1_000_000;
        limits.max_source_bytes = 10 * 1024 * 1024;
        limits.max_tokens = 2_000_000;
        limits.timeout_millis = 60_000;
    });
    let result = runner.execute_checked(&script, HashMap::new(), &QLOptions::default(), &profile);
    match result {
        Ok(ql_result) => {
            println!("{:?}", ql_result.result());
            Ok(())
        }
        Err(err) => {
            let code = err.error_code().to_string();
            let detail = err.reason().to_string();
            println!("{}", serde_json::json!({"code": code, "detail": detail}));
            Err(code)
        }
    }
}

// ---------------------------------------------------------------------------
// Combined entry point
// ---------------------------------------------------------------------------

/// Run all three probes up to `max_depth` and emit JSON to stdout.
pub fn run(max_depth: usize) -> Result<(), String> {
    eprintln!("[stack-depth-probe] max_depth={max_depth}");
    eprintln!("[stack-depth-probe] running probe_function_call_depth ...");
    probe_function_call_depth(max_depth)?;
    eprintln!("[stack-depth-probe] running probe_operand_stack_depth ...");
    probe_operand_stack_depth(max_depth)?;
    eprintln!("[stack-depth-probe] running probe_nested_try_catch ...");
    probe_nested_try_catch(max_depth)?;
    eprintln!("[stack-depth-probe] done.");
    Ok(())
}

// ---------------------------------------------------------------------------
// Internal helpers
// ---------------------------------------------------------------------------

/// Choose probe depths: include all `candidates` that are <= `max_depth`,
/// plus `max_depth` itself if not already present.
fn choose_depths(max_depth: usize, candidates: &[usize]) -> Vec<usize> {
    let mut depths: Vec<usize> = candidates
        .iter()
        .copied()
        .filter(|&d| d <= max_depth)
        .collect();
    if !depths.contains(&max_depth) {
        depths.push(max_depth);
    }
    depths.sort_unstable();
    depths.dedup();
    depths
}
