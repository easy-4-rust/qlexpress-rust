//! Worker 操作系统资源限制。

use std::time::Duration;

/// 父进程监督器与一次性执行子进程共同使用的操作系统资源限制。
///
/// Linux 可应用地址空间、CPU、文件大小与文件描述符限制；macOS 需要
/// 由容器或服务管理器额外提供内存上限。
/// 对应 Java: 无（Rust 可选部署组件）。
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
    /// Worker 最大子进程数（仅 Unix，防止 fork-bomb）。
    #[cfg(unix)]
    pub nproc: u64,
}

impl Default for WorkerLimits {
    fn default() -> Self {
        Self {
            wall_timeout: Duration::from_millis(1_500),
            memory_bytes: 256 * 1024 * 1024,
            cpu_seconds: 2,
            file_size_bytes: 2 * 1024 * 1024,
            open_files: 32,
            #[cfg(unix)]
            nproc: 256,
        }
    }
}
