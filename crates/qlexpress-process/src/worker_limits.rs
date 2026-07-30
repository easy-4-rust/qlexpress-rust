//! Worker 操作系统资源限制。

use std::time::Duration;

/// 父进程和 Worker 共同执行的硬资源限制。
#[derive(Clone, Debug)]
pub struct WorkerLimits {
    /// 父进程墙钟超时；到期后强制杀死 Worker。
    pub wall_timeout: Duration,
    /// Worker 地址空间上限。
    pub memory_bytes: u64,
    /// Worker CPU 秒数上限。
    pub cpu_seconds: u64,
    /// Worker 可写文件大小上限。
    pub file_size_bytes: u64,
    /// Worker 最大文件描述符数量。
    pub open_files: u64,
}

impl Default for WorkerLimits {
    fn default() -> Self {
        Self {
            wall_timeout: Duration::from_millis(1_500),
            memory_bytes: 256 * 1024 * 1024,
            cpu_seconds: 2,
            file_size_bytes: 2 * 1024 * 1024,
            open_files: 32,
        }
    }
}
