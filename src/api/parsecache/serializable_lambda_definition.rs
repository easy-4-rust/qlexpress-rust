//! 可序列化 Lambda 定义,对应 Java `com.alibaba.qlexpress4.api.parsecache.SerializableLambdaDefinition`。
//! 职责:以纯数据形式描述一个 Lambda 的编译期形态(指令序列 + 参数 + 最大栈深)。

use serde::{Deserialize, Serialize};

use super::serializable_instruction::SerializableInstruction;
use super::serializable_param::SerializableParam;

/// 可序列化 Lambda 定义。对应 Java: com.alibaba.qlexpress4.api.parsecache.SerializableLambdaDefinition
///
/// 字段对照:
/// - `name`(String):Lambda 名;
/// - `instructions`(List&lt;SerializableInstruction&gt;):指令序列;
/// - `params`(List&lt;SerializableParam&gt;):参数声明;
/// - `maxStackSize`(int):最大栈深。
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SerializableLambdaDefinition {
    /// Lambda 名。对应 Java 字段 `name`。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// 指令序列。对应 Java 字段 `instructions`。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub instructions: Option<Vec<SerializableInstruction>>,
    /// 参数声明列表。对应 Java 字段 `params`。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub params: Option<Vec<SerializableParam>>,
    /// 最大栈深。对应 Java 字段 `maxStackSize`(缺省 0)。
    #[serde(default)]
    pub max_stack_size: i32,
}
