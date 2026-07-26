//! 已加载的编译缓存,对应 Java `com.alibaba.qlexpress4.api.parsecache.LoadedParseCache`。
//! 职责:Importer 的产出——还原后的编译产物 + 源可序列化缓存 + runner 身份绑定。

use std::cell::RefCell;
use std::rc::Rc;

use crate::aparser::compile_cache::QCompileCache;
use crate::runtime::qlambda_definition::QLambdaDefinition;
use crate::runtime::trace::ExpressionTrace;

use super::serializable_parse_cache::SerializableParseCache;

/// Importer 还原后的编译缓存类型(Java `QCompileCache` 的具体化:
/// 主 Lambda 定义 + 表达式 trace 点)。
pub type LoadedCompileCache =
    QCompileCache<Rc<dyn QLambdaDefinition>, Rc<RefCell<ExpressionTrace>>>;

/// 已加载的编译缓存。对应 Java: com.alibaba.qlexpress4.api.parsecache.LoadedParseCache
///
/// runner 身份说明:Java 以 `Object runnerIdentity` 的引用相等(`==`)
/// 判断绑定;Rust 以 `usize` 身份令牌对应(由 Express4Runner 阶段为每个
/// runner 分配唯一值,指针/序号均可)。
pub struct LoadedParseCache {
    /// 还原后的编译产物。对应 Java 字段 `compileCache`。
    compile_cache: LoadedCompileCache,
    /// 源可序列化缓存。对应 Java 字段 `sourceCache`。
    source_cache: SerializableParseCache,
    /// runner 身份令牌。对应 Java 字段 `runnerIdentity`。
    runner_identity: usize,
}

impl LoadedParseCache {
    /// 构造。对应 Java 包私有构造器
    /// `LoadedParseCache(QCompileCache, SerializableParseCache, Object)`。
    pub fn new(
        compile_cache: LoadedCompileCache,
        source_cache: SerializableParseCache,
        runner_identity: usize,
    ) -> Self {
        LoadedParseCache {
            compile_cache,
            source_cache,
            runner_identity,
        }
    }

    /// 模型版本。对应 Java 方法 `getModelVersion()`。
    pub fn get_model_version(&self) -> i32 {
        self.source_cache.model_version
    }

    /// 产出方版本。对应 Java 方法 `getProducerVersion()`。
    pub fn get_producer_version(&self) -> Option<&str> {
        self.source_cache.producer_version.as_deref()
    }

    /// 脚本原文。对应 Java 方法 `getScript()`。
    pub fn get_script(&self) -> Option<&str> {
        self.source_cache.script.as_deref()
    }

    /// 脚本哈希。对应 Java 方法 `getScriptHash()`。
    pub fn get_script_hash(&self) -> Option<&str> {
        self.source_cache.script_hash.as_deref()
    }

    /// 是否含 trace 点。对应 Java 方法 `hasTracePoints()`。
    pub fn has_trace_points(&self) -> bool {
        self.source_cache.trace_points.is_some()
    }

    /// 还原后的编译产物。对应 Java 方法 `getCompileCache()`。
    pub fn get_compile_cache(&self) -> &LoadedCompileCache {
        &self.compile_cache
    }

    /// 源可序列化缓存。对应 Java 方法 `getSourceCache()`。
    pub fn get_source_cache(&self) -> &SerializableParseCache {
        &self.source_cache
    }

    /// 是否绑定到指定 runner。对应 Java 方法 `isBoundTo(Object)`
    /// (Java 为引用相等;Rust 为身份令牌相等)。
    pub fn is_bound_to(&self, runner_identity: usize) -> bool {
        self.runner_identity == runner_identity
    }
}
