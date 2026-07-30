//! 父进程 Worker 监督器。

use std::io::{Read, Write};
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::thread;
use std::time::Instant;

use crate::{WorkerLimits, WorkerRequest, WorkerResponse};

/// 启动一次性隔离进程并在超时后强制回收。
pub struct ProcessWorker {
    program: PathBuf,
    limits: WorkerLimits,
}

impl ProcessWorker {
    /// 使用指定 Worker 二进制和限制创建监督器。
    pub fn new(program: impl Into<PathBuf>, limits: WorkerLimits) -> Self {
        Self {
            program: program.into(),
            limits,
        }
    }

    /// 执行单个请求；每次调用创建全新进程。
    pub fn execute(&self, request: &WorkerRequest) -> Result<WorkerResponse, String> {
        let mut child = Command::new(&self.program)
            .env(
                "QLEXPRESS_WORKER_MEMORY_BYTES",
                self.limits.memory_bytes.to_string(),
            )
            .env(
                "QLEXPRESS_WORKER_CPU_SECONDS",
                self.limits.cpu_seconds.to_string(),
            )
            .env(
                "QLEXPRESS_WORKER_FILE_SIZE_BYTES",
                self.limits.file_size_bytes.to_string(),
            )
            .env(
                "QLEXPRESS_WORKER_OPEN_FILES",
                self.limits.open_files.to_string(),
            )
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|error| format!("failed to spawn qlexpress process worker: {error}"))?;

        let request_bytes =
            serde_json::to_vec(request).map_err(|error| format!("invalid request: {error}"))?;
        let mut stdin = child
            .stdin
            .take()
            .ok_or_else(|| "worker stdin is unavailable".to_string())?;
        stdin
            .write_all(&request_bytes)
            .map_err(|error| format!("failed to write worker request: {error}"))?;
        drop(stdin);

        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| "worker stdout is unavailable".to_string())?;
        let stderr = child
            .stderr
            .take()
            .ok_or_else(|| "worker stderr is unavailable".to_string())?;
        let stdout_reader = thread::spawn(move || read_bounded(stdout, 2 * 1024 * 1024));
        let stderr_reader = thread::spawn(move || read_bounded(stderr, 256 * 1024));

        let started = Instant::now();
        let status = loop {
            if let Some(status) = child
                .try_wait()
                .map_err(|error| format!("failed to poll worker: {error}"))?
            {
                break status;
            }
            if started.elapsed() >= self.limits.wall_timeout {
                let _ = child.kill();
                let _ = child.wait();
                let _ = stdout_reader.join();
                let _ = stderr_reader.join();
                return Ok(WorkerResponse::failure(
                    "WORKER_WALL_TIMEOUT",
                    "qlexpress process worker exceeded wall-clock timeout and was killed",
                ));
            }
            thread::sleep(std::time::Duration::from_millis(2));
        };

        let stdout = stdout_reader
            .join()
            .map_err(|_| "worker stdout reader panicked".to_string())??;
        let stderr = stderr_reader
            .join()
            .map_err(|_| "worker stderr reader panicked".to_string())??;
        if !status.success() {
            return Ok(WorkerResponse::failure(
                "WORKER_PROCESS_FAILED",
                String::from_utf8_lossy(&stderr).into_owned(),
            ));
        }
        serde_json::from_slice(&stdout).map_err(|error| format!("invalid worker response: {error}"))
    }
}

fn read_bounded(mut reader: impl Read, max_bytes: usize) -> Result<Vec<u8>, String> {
    let mut output = Vec::new();
    reader
        .by_ref()
        .take(u64::try_from(max_bytes).unwrap_or(u64::MAX) + 1)
        .read_to_end(&mut output)
        .map_err(|error| format!("failed to read worker output: {error}"))?;
    if output.len() > max_bytes {
        return Err("worker output exceeded supervisor limit".to_string());
    }
    Ok(output)
}
