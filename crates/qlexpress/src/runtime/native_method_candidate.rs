//! 带显式 Java 形参签名的原生方法候选。

use crate::runtime::class_ref::ClassRef;
use crate::runtime::native_type::NativeMethod;

/// 供 `MemberResolver` 按 Java 重载规则选择的方法候选。
/// 对应 Java：`java.lang.reflect.Method` 的参数类型、变参标记和调用体。
#[derive(Clone)]
pub struct NativeMethodCandidate {
    /// Java `Method#getParameterTypes()`。
    pub parameter_types: Vec<ClassRef>,
    /// Java `Method#isVarArgs()`。
    pub var_args: bool,
    /// 实际方法体。
    pub method: NativeMethod,
}
