//! 安全执行取消令牌。

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

/// 可由宿主跨线程触发的协作式取消令牌。
///
/// QlExpress Rust 原 Java 模型没有同名对象；该类型是 Rust 安全执行入口
/// 的扩展。宿主函数可从 [`crate::runtime::qcontext::QContext`] 取得令牌，
/// 长耗时操作应在阻塞点之间主动检查。
/// 对应 Java: 无（Rust 安全增强）。
#[derive(Clone, Debug, Default)]
pub struct CancellationToken {
    cancelled: Arc<AtomicBool>,
}

impl CancellationToken {
    /// 创建未取消的令牌。
    /// 对应 Java：无（Rust 安全增强的协作式取消令牌）。
    pub fn new() -> Self {
        Self::default()
    }

    /// 请求取消所有共享该令牌的执行与宿主调用。
    /// 对应 Java：无（Rust 安全增强的协作式取消操作）。
    pub fn cancel(&self) {
        self.cancelled.store(true, Ordering::Release);
    }

    /// 返回是否已收到取消请求。
    /// 对应 Java：无（Rust 安全增强的协作式取消状态）。
    pub fn is_cancelled(&self) -> bool {
        self.cancelled.load(Ordering::Acquire)
    }
}
