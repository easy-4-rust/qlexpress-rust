//! Runner 使用的脚本文本到编译产物缓存。

use std::collections::HashMap;

pub use super::q_compile_cache::{QCompileCache, ScriptCompileCache};

/// Java `Express4Runner.parseCache` 的 Rust 泛型缓存容器。
/// 对应 Java：`Express4Runner#compileCache`。
#[derive(Clone, Debug, Default)]
pub struct CompileCache<L, T> {
    pub(crate) map: HashMap<String, QCompileCache<L, T>>,
}
