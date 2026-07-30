//! Serializable parse cache, mirroring Java
//! `com.alibaba.qlexpress4.api.parsecache`(12 个类一一对应)。
//!
//! 编译缓存的导出/导入链路:编译产物(指令序列 + Lambda 定义 + 常量 +
//! trace 点)→ [`SerializableParseCacheExporter`] → `Serializable*` 纯数据
//! 结构(serde JSON)→ [`SerializableParseCacheImporter`] 还原。
//! 序列化按 SPEC §5.5.5 使用 serde/serde_json(Jackson 的对应物),默认启用。

pub mod concurrent_parse_cache;
pub mod loaded_parse_cache;
pub mod serializable_catch_entry;
pub mod serializable_constant;
pub mod serializable_instruction;
pub mod serializable_lambda_definition;
pub mod serializable_param;
pub mod serializable_parse_cache;
pub mod serializable_parse_cache_exception;
pub mod serializable_parse_cache_exporter;
pub mod serializable_parse_cache_importer;
pub mod serializable_source;
pub mod serializable_trace_point;

pub use concurrent_parse_cache::ConcurrentParseCache;
pub use loaded_parse_cache::{LoadedCompileCache, LoadedParseCache};
pub use serializable_catch_entry::SerializableCatchEntry;
pub use serializable_constant::SerializableConstant;
pub use serializable_instruction::SerializableInstruction;
pub use serializable_lambda_definition::SerializableLambdaDefinition;
pub use serializable_param::SerializableParam;
pub use serializable_parse_cache::SerializableParseCache;
pub use serializable_parse_cache_exception::SerializableParseCacheException;
pub use serializable_parse_cache_exporter::{SerializableParseCacheExporter, MODEL_VERSION};
pub use serializable_parse_cache_importer::SerializableParseCacheImporter;
pub use serializable_source::SerializableSource;
pub use serializable_trace_point::SerializableTracePoint;
