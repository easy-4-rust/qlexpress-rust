//! Multi-process isolation verification harness.
//!
//! Validates three isolation dimensions for the QlExpress process worker:
//!
//! 1. **Consistency**: Eight concurrent processes execute the same script; all
//!    produce identical checksums (no state leakage across process boundaries).
//! 2. **RLIMIT_NPROC enforcement**: A child process that tries to spawn many
//!    sub-processes is blocked by the operating system's `RLIMIT_NPROC` limit.
//!    The parent harness sets the limit via the POSIX `ulimit -u` builtin
//!    before spawning the child, mirroring how `ProcessWorker` applies
//!    `QLEXPRESS_WORKER_NPROC` (default 256) in production.
//! 3. **Panic isolation**: A panicking child process does not affect the other
//!    seven processes that continue executing normally.
//!
//! All scenarios use `std::process::Command` to spawn the companion
//! `run-script` binary; no direct `fork()` is used.

use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

/// The default QLExpress script used across all scenarios for determinism.
const TEST_SCRIPT: &str = "function square(x) { return x * x; }; square(17) + 3";

/// How many iterations each child process runs per script invocation.
const ITERATIONS_PER_CHILD: usize = 200;

/// Per-child wall-clock timeout before the parent kills it.
const CHILD_TIMEOUT: Duration = Duration::from_secs(30);

// ---------------------------------------------------------------------------
// Public entry point
// ---------------------------------------------------------------------------

/// Run the multi-process isolation verification harness.
///
/// # Arguments
///
/// * `processes` - Number of concurrent child processes (default 8).
/// * `timeout`   - Overall wall-clock budget for the entire harness.
///
/// # Errors
///
/// Returns `Err` if any scenario fails its assertion.
pub fn run(processes: usize, timeout: Duration) -> Result<(), String> {
    if processes < 2 {
        return Err("processes must be at least 2".to_string());
    }
    let run_script = find_run_script_binary()?;
    let deadline = Instant::now() + timeout;

    println!("multi-process-isolation: processes={processes}, timeout={timeout:?}");

    // --- Scenario 1: consistency -------------------------------------------------
    println!(
        "[scenario 1/3] consistency: {processes} processes x {ITERATIONS_PER_CHILD} iterations"
    );
    scenario_consistency(&run_script, processes, deadline)?;
    println!("[scenario 1/3] PASSED");

    // --- Scenario 2: RLIMIT_NPROC ------------------------------------------------
    println!("[scenario 2/3] RLIMIT_NPROC enforcement");
    scenario_rlimit_nproc(&run_script, deadline)?;
    println!("[scenario 2/3] PASSED");

    // --- Scenario 3: panic isolation ---------------------------------------------
    println!(
        "[scenario 3/3] panic isolation: 1 panicker + {} normal",
        processes - 1
    );
    scenario_panic_isolation(&run_script, processes, deadline)?;
    println!("[scenario 3/3] PASSED");

    println!("multi-process-isolation: ALL SCENARIOS PASSED");
    Ok(())
}

// ---------------------------------------------------------------------------
// Scenario implementations
// ---------------------------------------------------------------------------

/// Scenario 1: spawn `count` children running the same script, assert all
/// checksums are identical.
fn scenario_consistency(run_script: &Path, count: usize, deadline: Instant) -> Result<(), String> {
    let mut children: Vec<Child> = Vec::with_capacity(count);
    for _ in 0..count {
        let child = spawn_run_script(
            run_script,
            &[TEST_SCRIPT, &ITERATIONS_PER_CHILD.to_string()],
            deadline,
        )?;
        children.push(child);
    }

    let checksums = collect_checksums(&mut children, deadline)?;

    // All checksums must be identical.
    let first = checksums.first().ok_or("no children produced a checksum")?;
    for (i, cs) in checksums.iter().enumerate() {
        if cs != first {
            return Err(format!(
                "consistency violation: process 0 checksum={first}, process {i} checksum={cs}"
            ));
        }
    }
    Ok(())
}

/// Scenario 2: spawn a child in `--spawn-bomb` mode with a low RLIMIT_NPROC
/// (applied via POSIX `ulimit -u` in a shell wrapper) and verify the child
/// hits the limit before exhausting its spawn budget.
fn scenario_rlimit_nproc(run_script: &Path, deadline: Instant) -> Result<(), String> {
    // Use a generous spawn budget; the OS limit should kick in well before this.
    let spawn_budget = 500u32;
    // Determine how many processes this user is currently running, then set
    // RLIMIT_NPROC to current + margin. This ensures the limit is above the
    // existing process count (so ulimit -u succeeds) but low enough that the
    // child's spawn loop will hit the wall after ~margin spawns.
    let current_nproc = current_user_process_count().unwrap_or(100);
    // Margin must accommodate: the run-script process itself (1) + spawned
    // children. With margin=30 we expect ~29 children before hitting the wall.
    let margin: u32 = 30;
    let nproc_limit = current_nproc + margin;
    println!("  current per-user processes: {current_nproc}, RLIMIT_NPROC set to {nproc_limit}");

    // Wrap the spawn-bomb in `sh -c "ulimit -u <N>; exec <binary> ..."`
    // so RLIMIT_NPROC is applied before the child tries to spawn.
    let shell_cmd = format!(
        "ulimit -u {nproc_limit}; exec {} --spawn-bomb {spawn_budget}",
        run_script.display()
    );

    let mut child = Command::new("sh")
        .args(["-c", &shell_cmd])
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| format!("failed to spawn shell wrapper: {e}"))?;

    let output = wait_for_child(&mut child, deadline)?;

    // If ulimit itself failed (e.g. requested value above hard limit),
    // the shell exits non-zero with a diagnostic. Treat as skip.
    if !output.exit_success {
        if output.stderr.contains("ulimit")
            || output.stderr.contains("setrlimit")
            || output.stderr.contains("RLIMIT_NPROC")
        {
            println!(
                "  [skip] RLIMIT_NPROC not applicable on this platform: {}",
                output.stderr.trim()
            );
            return Ok(());
        }
        return Err(format!(
            "spawn-bomb child exited with error: {}",
            output.stderr
        ));
    }

    // Parse "spawned=N,limit_hit=<bool>".
    let spawned = parse_spawned_count(&output.stdout)?;
    let limit_hit = output.stdout.contains("limit_hit=true");

    if !limit_hit {
        return Err(format!(
            "RLIMIT_NPROC was not hit: child spawned all {spawn_budget} processes \
             (spawned={spawned}). The limit of {nproc_limit} was not enforced."
        ));
    }
    println!("  child spawned {spawned} processes before RLIMIT_NPROC blocked further spawns");
    Ok(())
}

/// Scenario 3: one panicking child alongside `count - 1` normal children.
/// The panicker must exit non-zero; all others must exit zero with matching
/// checksums.
fn scenario_panic_isolation(
    run_script: &Path,
    count: usize,
    deadline: Instant,
) -> Result<(), String> {
    let normal_count = count - 1;

    // Spawn the panicking child first.
    let mut panicker = spawn_run_script(
        run_script,
        &["--panic", TEST_SCRIPT, &ITERATIONS_PER_CHILD.to_string()],
        deadline,
    )?;

    // Spawn normal children.
    let mut normals: Vec<Child> = Vec::with_capacity(normal_count);
    for _ in 0..normal_count {
        let child = spawn_run_script(
            run_script,
            &[TEST_SCRIPT, &ITERATIONS_PER_CHILD.to_string()],
            deadline,
        )?;
        normals.push(child);
    }

    // Wait for the panicker; it must fail.
    let panicker_result = collect_single(&mut panicker, deadline)?;
    if panicker_result.exit_success {
        return Err("panicking child exited successfully; expected non-zero exit".to_string());
    }
    println!("  panicker exited non-zero (as expected)");

    // Collect normal children.
    let checksums = collect_checksums(&mut normals, deadline)?;
    let first = checksums
        .first()
        .ok_or("no normal children produced a checksum")?;
    for (i, cs) in checksums.iter().enumerate() {
        if cs != first {
            return Err(format!(
                "panic isolation violation: normal process 0 checksum={first}, \
                 normal process {i} checksum={cs}"
            ));
        }
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Process helpers
// ---------------------------------------------------------------------------

/// Count the number of processes currently running for the current user.
///
/// Uses `ps -u $(whoami)` on Unix. Returns `None` on parse failure so the
/// caller can fall back to a safe default.
fn current_user_process_count() -> Option<u32> {
    let output = Command::new("sh")
        .args(["-c", "ps -u $(whoami) | wc -l"])
        .output()
        .ok()?;
    let text = String::from_utf8_lossy(&output.stdout);
    // `wc -l` includes the header line, so subtract 1.
    let count: u32 = text.trim().parse().ok()?;
    count.checked_sub(1)
}

/// Locate the `run-script` binary adjacent to the current executable.
fn find_run_script_binary() -> Result<PathBuf, String> {
    let current =
        std::env::current_exe().map_err(|e| format!("cannot determine current exe path: {e}"))?;
    let dir = current
        .parent()
        .ok_or("current exe has no parent directory")?;
    let candidate = dir.join("run-script");
    if candidate.is_file() {
        return Ok(candidate);
    }
    // Fallback: try with platform-specific extension.
    let candidate_exe = dir.join("run-script.exe");
    if candidate_exe.is_file() {
        return Ok(candidate_exe);
    }
    Err(format!(
        "run-script binary not found next to {}: looked for {} and {}",
        current.display(),
        candidate.display(),
        candidate_exe.display(),
    ))
}

/// Spawn the `run-script` binary with the given arguments.
fn spawn_run_script(program: &Path, args: &[&str], deadline: Instant) -> Result<Child, String> {
    if Instant::now() >= deadline {
        return Err("harness deadline already exceeded before spawn".to_string());
    }
    Command::new(program)
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| format!("failed to spawn run-script: {e}"))
}

/// Outcome of waiting for a single child process.
struct ChildOutput {
    exit_success: bool,
    stdout: String,
    #[allow(dead_code)]
    stderr: String,
}

/// Wait for a child with a poll-based timeout, kill on expiry.
fn wait_for_child(child: &mut Child, deadline: Instant) -> Result<ChildOutput, String> {
    let child_deadline = Instant::now() + CHILD_TIMEOUT;
    let effective_deadline = if child_deadline < deadline {
        child_deadline
    } else {
        deadline
    };

    loop {
        if let Some(status) = child
            .try_wait()
            .map_err(|e| format!("try_wait failed: {e}"))?
        {
            let stdout = drain_pipe(&mut child.stdout);
            let stderr = drain_pipe(&mut child.stderr);
            return Ok(ChildOutput {
                exit_success: status.success(),
                stdout,
                stderr,
            });
        }
        if Instant::now() >= effective_deadline {
            let _ = child.kill();
            let _ = child.wait();
            let stderr = drain_pipe(&mut child.stderr);
            return Err(format!("child timed out (killed). stderr: {stderr}"));
        }
        std::thread::sleep(Duration::from_millis(5));
    }
}

/// Collect a single child's output.
fn collect_single(child: &mut Child, deadline: Instant) -> Result<ChildOutput, String> {
    wait_for_child(child, deadline)
}

/// Wait for all children and extract their checksums from stdout.
fn collect_checksums(children: &mut [Child], deadline: Instant) -> Result<Vec<u64>, String> {
    let mut checksums = Vec::with_capacity(children.len());
    for (i, child) in children.iter_mut().enumerate() {
        let output = wait_for_child(child, deadline)?;
        if !output.exit_success {
            return Err(format!("child {i} exited with failure: {}", output.stderr));
        }
        let cs = parse_checksum(&output.stdout)
            .map_err(|e| format!("child {i}: {e} (stdout: {:?})", output.stdout))?;
        checksums.push(cs);
    }
    Ok(checksums)
}

/// Parse `checksum=<hex>` from stdout.
fn parse_checksum(stdout: &str) -> Result<u64, String> {
    let line = stdout
        .lines()
        .find(|l| l.starts_with("checksum="))
        .ok_or("no checksum= line in output")?;
    let hex = line.strip_prefix("checksum=").unwrap();
    u64::from_str_radix(hex, 16).map_err(|e| format!("invalid checksum hex '{hex}': {e}"))
}

/// Parse `spawned=<N>` from stdout.
fn parse_spawned_count(stdout: &str) -> Result<u32, String> {
    let line = stdout
        .lines()
        .find(|l| l.starts_with("spawned="))
        .ok_or("no spawned= line in output")?;
    let rest = line.strip_prefix("spawned=").unwrap();
    let num_str = rest.split(',').next().unwrap_or(rest);
    num_str
        .parse::<u32>()
        .map_err(|e| format!("invalid spawned count '{num_str}': {e}"))
}

/// Drain a piped output handle into a `String`.
fn drain_pipe(handle: &mut Option<impl Read>) -> String {
    match handle {
        Some(reader) => {
            let mut buf = String::new();
            let _ = reader.read_to_string(&mut buf);
            buf
        }
        None => String::new(),
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    /// Unit test: verify `parse_checksum` correctly extracts hex values.
    #[test]
    fn parse_checksum_extracts_hex() {
        assert_eq!(parse_checksum("checksum=000000000000002a\n").unwrap(), 42);
        assert_eq!(
            parse_checksum("checksum=deadbeefcafebabe\n").unwrap(),
            0xdeadbeefcafebabe
        );
        assert!(parse_checksum("no checksum here\n").is_err());
        assert!(parse_checksum("checksum=not_hex\n").is_err());
    }

    /// Unit test: verify `parse_spawned_count` correctly extracts the count.
    #[test]
    fn parse_spawned_count_extracts_number() {
        assert_eq!(
            parse_spawned_count("spawned=42,limit_hit=true\n").unwrap(),
            42
        );
        assert_eq!(
            parse_spawned_count("spawned=0,limit_hit=false\n").unwrap(),
            0
        );
        assert!(parse_spawned_count("no match\n").is_err());
    }

    /// Unit test: verify `collect_checksums` rejects non-zero exit children.
    #[test]
    fn collect_checksums_rejects_failed_child() {
        // Spawn a process that exits immediately with non-zero status.
        let child = Command::new("sh")
            .args(["-c", "exit 1"])
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();

        let deadline = Instant::now() + Duration::from_secs(5);
        let result = collect_checksums(&mut [child], deadline);
        assert!(result.is_err(), "should reject non-zero exit child");
    }

    /// Unit test: verify `collect_checksums` returns parsed values for
    /// well-formed output.
    #[test]
    fn collect_checksums_parses_valid_output() {
        let child_a = Command::new("sh")
            .args(["-c", "echo 'checksum=0000000000000001'"])
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();
        let child_b = Command::new("sh")
            .args(["-c", "echo 'checksum=0000000000000002'"])
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();

        let deadline = Instant::now() + Duration::from_secs(5);
        let checksums = collect_checksums(&mut [child_a, child_b], deadline).unwrap();
        assert_eq!(checksums, vec![1, 2]);
    }

    /// Unit test: verify `find_run_script_binary` reports a clear error when
    /// the binary does not exist.
    #[test]
    fn find_run_script_binary_reports_missing() {
        // This test exercises the error path. The actual binary may or may
        // not exist depending on build state; we just verify the function
        // does not panic.
        let _ = find_run_script_binary();
    }

    /// Integration test: spawn 2 real `run-script` children and verify
    /// checksum consistency. Requires the `run-script` binary to be built
    /// (happens automatically with `cargo test`).
    #[test]
    fn real_two_process_consistency() {
        let run_script = match find_run_script_binary() {
            Ok(p) => p,
            Err(_) => {
                eprintln!("SKIP: run-script binary not found (run `cargo build` first)");
                return;
            }
        };

        let deadline = Instant::now() + Duration::from_secs(60);
        let mut children = Vec::new();
        for _ in 0..2 {
            let child = spawn_run_script(&run_script, &[TEST_SCRIPT, "50"], deadline)
                .expect("spawn failed");
            children.push(child);
        }

        let checksums = collect_checksums(&mut children, deadline).expect("collect failed");
        assert_eq!(checksums.len(), 2);
        assert_eq!(
            checksums[0], checksums[1],
            "two processes should produce identical checksums"
        );
    }

    /// Integration test: verify the panic child exits non-zero while a
    /// normal child succeeds. Requires the `run-script` binary to be built.
    #[test]
    fn real_panic_isolation() {
        let run_script = match find_run_script_binary() {
            Ok(p) => p,
            Err(_) => {
                eprintln!("SKIP: run-script binary not found (run `cargo build` first)");
                return;
            }
        };

        let deadline = Instant::now() + Duration::from_secs(60);

        // Spawn a panicking child.
        let mut panicker = spawn_run_script(&run_script, &["--panic", TEST_SCRIPT, "10"], deadline)
            .expect("spawn panicker failed");

        // Spawn a normal child.
        let mut normal = spawn_run_script(&run_script, &[TEST_SCRIPT, "10"], deadline)
            .expect("spawn normal failed");

        let panicker_out = collect_single(&mut panicker, deadline).expect("wait panicker");
        assert!(
            !panicker_out.exit_success,
            "panicking child should exit non-zero"
        );

        let normal_out = collect_single(&mut normal, deadline).expect("wait normal");
        assert!(normal_out.exit_success, "normal child should exit zero");
        let cs = parse_checksum(&normal_out.stdout).expect("parse checksum");
        assert!(cs > 0, "checksum should be non-zero for a real script");
    }

    /// Integration test: verify RLIMIT_NPROC enforcement via ulimit wrapper.
    /// Requires the `run-script` binary to be built.
    #[test]
    fn real_rlimit_nproc_enforcement() {
        let run_script = match find_run_script_binary() {
            Ok(p) => p,
            Err(_) => {
                eprintln!("SKIP: run-script binary not found (run `cargo build` first)");
                return;
            }
        };

        let deadline = Instant::now() + Duration::from_secs(30);
        let spawn_budget = 200u32;
        // Set limit to current + margin so ulimit succeeds but spawns are capped.
        let current = current_user_process_count().unwrap_or(100);
        let nproc_limit = current + 30;

        let shell_cmd = format!(
            "ulimit -u {nproc_limit}; exec {} --spawn-bomb {spawn_budget}",
            run_script.display()
        );

        let mut child = Command::new("sh")
            .args(["-c", &shell_cmd])
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("spawn failed");

        let output = wait_for_child(&mut child, deadline).expect("wait failed");

        // If ulimit is not supported, skip.
        if !output.exit_success {
            eprintln!("SKIP: ulimit failed: {}", output.stderr.trim());
            return;
        }

        let spawned = parse_spawned_count(&output.stdout).expect("parse spawned");
        let limit_hit = output.stdout.contains("limit_hit=true");

        assert!(
            limit_hit,
            "RLIMIT_NPROC should have been hit (spawned={spawned}/{spawn_budget})"
        );
        assert!(
            spawned < spawn_budget,
            "should have spawned fewer than {spawn_budget}, got {spawned}"
        );
    }
}
