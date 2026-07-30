//! 安全策略使用的原生成员标识。

/// 供原生成员安全策略匹配的“声明类型 + 成员名”标识。
///
/// 对应 Java: 传入 `QLSecurityStrategy#check` 的
/// `java.lang.reflect.Member` 身份信息。
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct NativeMember {
    /// NativeRegistry 中注册的原生类型名。
    pub type_name: String,
    /// 方法或字段名称。
    pub member_name: String,
}
