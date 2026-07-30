//! QlExpress Rust 隔离进程的父进程监督器。

use std::io::{Read, Write};
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::thread;
use std::time::Instant;

use crate::{WorkerLimits, WorkerRequest, WorkerResponse};

/// 为每个规则请求启动一次性子进程并负责有界 I/O 与强制回收。
///
/// 该类型提供进程边界和 Unix 资源限制，但不宣称具备 namespace、
/// seccomp、网络或文件系统沙箱能力。生产中的敌对输入仍应运行在容器
/// 或等价的操作系统隔离环境中。
/// 对应 Java: 无（Rust 可选部署组件）。
pub struct ProcessWorker {
    program: PathBuf,
    limits: WorkerLimits,
}

impl ProcessWorker {
    /// 使用指定执行器二进制和操作系统限制创建监督器。
    ///
    /// # Arguments
    ///
    /// * `program` - `qlexpress-process` 二进制路径。
    /// * `limits` - 墙钟时间、内存、CPU、文件大小和文件描述符限制。
    ///
    /// # Returns
    ///
    /// 返回可重复提交请求的父进程监督器；每次提交仍会创建新子进程。
    pub fn new(program: impl Into<PathBuf>, limits: WorkerLimits) -> Self {
        Self {
            program: program.into(),
            limits,
        }
    }

    /// 在全新子进程中执行一个规则请求。
    ///
    /// # Arguments
    ///
    /// * `request` - 包含脚本、JSON 上下文、租户标识和可选引擎预算的请求。
    ///
    /// # Returns
    ///
    /// 子进程正常完成或被墙钟超时终止时返回结构化响应；父进程无法启动、
    /// I/O 失败或响应协议无效时返回监督器错误。
    ///
    /// # Errors
    ///
    /// 二进制无法启动、请求无法编码、标准流不可用、输出超过上限、读取线程
    /// 异常或响应不是合法 JSON 时返回 `Err(String)`。
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
