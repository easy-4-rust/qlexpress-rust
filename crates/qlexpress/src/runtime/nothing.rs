//! 对应 Java 类：com.alibaba.qlexpress4.runtime.Nothing
//!
//! Java 中 `Nothing` 是一个 `private` 构造器的占位类，注释为
//! "subclass of any object"——它在 QLExpress4 中代表 **null 字面量的运行时类型**：
//! 当脚本里出现 `null` 时，对应的 `Class<?>` 不是 Java 的 `null`，而是 `Nothing.class`。
//!
//! Rust 侧没有运行时 `Class<?>` 概念，`DataValue::Null` 已经表达 null 值，
//! 而"null 字面量的类型"以字符串 `"com.alibaba.qlexpress4.runtime.Nothing"`
//! 形式在 [`crate::runtime::class_ref::ClassRef`] 与
//! [`crate::runtime::member_resolver`] 中流通（与 Java 字节码里的
//! `Nothing` 类型标识一一对应）。
//!
//! 该文件保留独立模块以维持"一文件一对象"的结构对齐（SPEC §2），
//! 并提供：
//! - [`NOTHING_TYPE_NAME`]：所有引用点的单一事实来源；
//! - [`Nothing`] 单元结构体：仅作为类型存在性的占位，对应 Java 类身份。

/// Java `com.alibaba.qlexpress4.runtime.Nothing` 的全限定类型名，
/// 与 Java 字节码中 `Nothing` 类型标识一一对应。
///
/// 该字符串在以下位置被引用：
/// - `runtime/value.rs`：`DataValue::Null` 的 `type_name()` / `class_name()`
/// - `runtime/member_resolver.rs`：null 实参的方法分派匹配
/// - `runtime/function/qmethod_function.rs`：方法实参类型回填
/// - `runtime/class_ref.rs`：通过 `ClassRef::Named(NOTHING_TYPE_NAME)` 表达
pub const NOTHING_TYPE_NAME: &str = "com.alibaba.qlexpress4.runtime.Nothing";

/// 占位类型，对应 Java `com.alibaba.qlexpress4.runtime.Nothing`。
///
/// 与 Java 一致，构造器为 `private`：私有零尺寸字段阻止 crate 外直接
/// 构造，同时不提供 `Default` 等公开替代构造入口。该类型仅用于“类身份
/// 存在”的对照。
///
/// 注：null 字面量在 Rust 运行时由 `DataValue::Null` 表达，
/// 类型信息以字符串 [`NOTHING_TYPE_NAME`] 流通，无需构造本结构体实例。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Nothing(());

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn type_name_matches_java_fqn() {
        assert_eq!(NOTHING_TYPE_NAME, "com.alibaba.qlexpress4.runtime.Nothing");
    }

    #[test]
    fn nothing_is_zero_sized() {
        assert_eq!(std::mem::size_of::<Nothing>(), 0);
    }
}
