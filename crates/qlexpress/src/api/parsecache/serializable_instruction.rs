//! 可序列化指令,对应 Java `com.alibaba.qlexpress4.api.parsecache.SerializableInstruction`。
//! 职责:以「opcode + 源码位置 + 操作数表」形式描述一条 QVM 指令。

use serde::{Deserialize, Serialize};

use super::serializable_source::SerializableSource;

/// 可序列化指令。对应 Java: com.alibaba.qlexpress4.api.parsecache.SerializableInstruction
///
/// 字段对照:
/// - `opcode`(String):指令操作码(`CONST / LOAD / POP / RETURN /
///   BREAK_CONTINUE / THROW / CHECK_TIMEOUT / JUMP / JUMP_IF / JUMP_IF_POP /
///   BINARY_OP / PREFIX_UNARY_OP / SUFFIX_UNARY_OP / CALL_FUNCTION / CALL /
///   LOAD_LAMBDA / DEFINE_FUNCTION / NEW_SCOPE / CLOSE_SCOPE / DEFINE_LOCAL /
///   NEW_INSTANCE / NEW_FILLED_INSTANCE / NEW_ARRAY / MULTI_NEW_ARRAY /
///   NEW_LIST / NEW_MAP / GET_FIELD / SPREAD_GET_FIELD / METHOD_INVOKE /
///   SPREAD_METHOD_INVOKE / GET_METHOD / INDEX / SLICE / CAST / WHILE / FOR /
///   FOR_EACH / TRY_CATCH / TRACE_PEEK / TRACE_EVALUATED / STRING_JOIN`);
/// - `source`([`SerializableSource`]):源码位置;
/// - `operands`(Map&lt;String, Object&gt;):操作数表,键与 Java Exporter
///   的 `operands.put(...)` 完全一致(如 `constant / name / position /
///   expect / operator / argNum / lambda / keys / traceKey` 等)。
///
/// 说明:Java 用 `LinkedHashMap` 保持操作数插入序;Rust 用
/// `serde_json::Map`(默认 BTree 序)。键值集合完全一致,顺序不影响语义。
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
/// 对应 Java: com.alibaba.qlexpress4.api.parsecache.SerializableInstruction。
pub struct SerializableInstruction {
    /// 指令操作码。对应 Java 字段 `opcode`。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub opcode: Option<String>,
    /// 源码位置。对应 Java 字段 `source`。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source: Option<SerializableSource>,
    /// 操作数表。对应 Java 字段 `operands`。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub operands: Option<serde_json::Map<String, serde_json::Value>>,
}
