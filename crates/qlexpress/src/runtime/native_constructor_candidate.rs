//! 带显式 Java 形参签名的原生构造器候选。

use crate::runtime::class_ref::ClassRef;
use crate::runtime::native_type::NativeConstructor;

/// 供 `MemberResolver` 按 Java 重载规则选择的构造器候选。
#[derive(Clone)]
pub struct NativeConstructorCandidate {
    /// Java `Constructor#getParameterTypes()`。
    pub parameter_types: Vec<ClassRef>,
    /// Java `Constructor#isVarArgs()`。
    pub var_args: bool,
    /// 实际构造器。
    pub constructor: NativeConstructor,
}
