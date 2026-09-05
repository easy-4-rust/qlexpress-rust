//! Worker 启动期操作系统资源限制。

use crate::worker_limits::WorkerLimits;

/// 从父进程设置的环境变量读取限制。
pub fn limits_from_env() -> WorkerLimits {
    let mut limits = WorkerLimits::default();
    limits.memory_bytes = env_u64("QLEXPRESS_WORKER_MEMORY_BYTES", limits.memory_bytes);
    limits.cpu_seconds = env_u64("QLEXPRESS_WORKER_CPU_SECONDS", limits.cpu_seconds);
    limits.file_size_bytes = env_u64("QLEXPRESS_WORKER_FILE_SIZE_BYTES", limits.file_size_bytes);
    limits.open_files = env_u64("QLEXPRESS_WORKER_OPEN_FILES", limits.open_files);
    #[cfg(unix)]
    {
        limits.nproc = env_u64("QLEXPRESS_WORKER_NPROC", limits.nproc);
    }
    limits
}

fn env_u64(name: &str, default: u64) -> u64 {
    std::env::var(name)
        .ok()
        .and_then(|value| value.parse().ok())
        .filter(|value| *value > 0)
        .unwrap_or(default)
}

/// 在执行任何脚本前应用操作系统限制。
#[cfg(unix)]
pub fn apply(limits: &WorkerLimits) -> Result<(), String> {
    set_memory_limit(limits.memory_bytes)?;
    set_limit(libc::RLIMIT_CPU as _, limits.cpu_seconds, "cpu")?;
    set_limit(libc::RLIMIT_FSIZE as _, limits.file_size_bytes, "file size")?;
    set_limit(libc::RLIMIT_NOFILE as _, limits.open_files, "open files")?;
    set_nproc_limit(limits.nproc)?;
    Ok(())
}

#[cfg(all(unix, target_os = "macos"))]
fn set_memory_limit(_memory_bytes: u64) -> Result<(), String> {
    // macOS 对 RLIMIT_AS/RLIMIT_DATA 的降限返回 EINVAL。macOS 部署必须
    // 由容器/launchd 沙箱提供内存上限；监督器仍强制墙钟终止，并设置
    // CPU、文件大小和文件描述符限制。Linux 使用下方 RLIMIT_AS。
    Ok(())
}

#[cfg(all(unix, not(target_os = "macos")))]
fn set_memory_limit(memory_bytes: u64) -> Result<(), String> {
    set_limit(libc::RLIMIT_AS as _, memory_bytes, "address space")
}

/// 设置 RLIMIT_NPROC（最大子进程数），防止 fork-bomb 耗尽 PID 表。
///
/// 仅 Linux 与 macOS 支持 `RLIMIT_NPROC`；其他 Unix 平台跳过。
#[cfg(any(target_os = "linux", target_os = "macos"))]
fn set_nproc_limit(nproc: u64) -> Result<(), String> {
    set_limit(libc::RLIMIT_NPROC as _, nproc, "nproc")
}

#[cfg(all(unix, not(any(target_os = "linux", target_os = "macos"))))]
fn set_nproc_limit(_nproc: u64) -> Result<(), String> {
    // 此 Unix 平台不支持 RLIMIT_NPROC，由容器或外部沙箱提供进程数限制。
    Ok(())
}

/// 非 Unix 平台必须由容器或 Job Object 提供硬限制。
#[cfg(not(unix))]
pub fn apply(_limits: &WorkerLimits) -> Result<(), String> {
    Err("OS resource limits require Unix setrlimit or an external process sandbox".to_string())
}

#[cfg(unix)]
fn set_limit(resource: libc::c_int, value: u64, name: &str) -> Result<(), String> {
    let value = libc::rlim_t::try_from(value)
        .map_err(|_| format!("{name} limit does not fit platform rlim_t"))?;
    let mut current = libc::rlimit {
        rlim_cur: 0,
        rlim_max: 0,
    };
    // SAFETY: `current` 是有效可写的 `rlimit`，resource 来自 RLIMIT_*。
    if unsafe { libc::getrlimit(resource as _, &mut current) } != 0 {
        return Err(format!(
            "failed to read {name} limit: {}",
            std::io::Error::last_os_error()
        ));
    }
    // 只收紧 soft limit，不尝试改变宿主 hard limit；这在 macOS/Linux
    // 均可由普通进程执行，并避免容器已设置更低 hard limit 时出现 EINVAL。
    let limit = libc::rlimit {
        rlim_cur: value.min(current.rlim_max),
        rlim_max: current.rlim_max,
    };
    // SAFETY: `limit` 指向有效且已初始化的 `rlimit`，调用只影响当前 Worker。
    let result = unsafe { libc::setrlimit(resource as _, &limit) };
    if result == 0 {
        Ok(())
    } else {
        Err(format!(
            "failed to set {name} limit: {}",
            std::io::Error::last_os_error()
        ))
    }
}

#[cfg(all(test, unix))]
mod tests {
    use super::*;
    use std::time::Duration;

    /// 验证 `apply` 成功设置 RLIMIT_NPROC 并可读回。
    #[cfg(any(target_os = "linux", target_os = "macos"))]
    #[test]
    fn apply_sets_rlimit_nproc() {
        // 读取当前 hard limit 以选择一个不超出的 soft 值。
        let mut current = libc::rlimit {
            rlim_cur: 0,
            rlim_max: 0,
        };
        // SAFETY: `current` 是栈上有效可写的 rlimit。
        let rc = unsafe { libc::getrlimit(libc::RLIMIT_NPROC as _, &mut current) };
        assert_eq!(rc, 0, "getrlimit(RLIMIT_NPROC) failed");

        let target = current.rlim_max.min(256);
        let limits = WorkerLimits {
            wall_timeout: Duration::from_secs(1),
            memory_bytes: 256 * 1024 * 1024,
            cpu_seconds: 2,
            file_size_bytes: 2 * 1024 * 1024,
            open_files: 32,
            nproc: target,
        };
        apply(&limits).expect("apply should succeed");

        let mut after = libc::rlimit {
            rlim_cur: 0,
            rlim_max: 0,
        };
        // SAFETY: `after` 是栈上有效可写的 rlimit。
        let rc = unsafe { libc::getrlimit(libc::RLIMIT_NPROC as _, &mut after) };
        assert_eq!(rc, 0, "getrlimit(RLIMIT_NPROC) failed after apply");
        assert_eq!(
            after.rlim_cur, target,
            "RLIMIT_NPROC soft limit should equal requested value (clamped to hard limit)"
        );
    }

    /// 验证 `limits_from_env` 读取 QLEXPRESS_WORKER_NPROC 环境变量。
    #[cfg(unix)]
    #[test]
    fn limits_from_env_reads_nproc() {
        // 清除可能残留的环境变量。
        std::env::remove_var("QLEXPRESS_WORKER_NPROC");
        let defaults = limits_from_env();
        assert_eq!(defaults.nproc, 256, "default nproc should be 256");

        std::env::set_var("QLEXPRESS_WORKER_NPROC", "128");
        let custom = limits_from_env();
        assert_eq!(custom.nproc, 128, "nproc should be read from env");
        std::env::remove_var("QLEXPRESS_WORKER_NPROC");
    }
}
