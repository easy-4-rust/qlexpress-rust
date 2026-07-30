//! Lambda 定义中的参数声明。

use crate::runtime::class_ref::ClassRef;

/// Lambda 参数名和可选声明类型。
///
/// 对应 Java:
/// `com.alibaba.qlexpress4.runtime.QLambdaDefinitionInner.Param`；
/// `clazz == None` 对应 Java 的 `null Class<?>`。
#[derive(Clone, Debug)]
pub struct Param {
    pub(crate) name: String,
    pub(crate) clazz: Option<ClassRef>,
}
