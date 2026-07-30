//! 扩展函数的可共享适配器。

use super::extension_function::ExtensionFunction;

/// 等价于 Java 单例扩展函数的 Rust 共享句柄。
pub struct SharedExtension<F: ExtensionFunction> {
    pub(crate) extension: F,
}
