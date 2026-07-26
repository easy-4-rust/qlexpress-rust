//! 对应 Java 类：com.alibaba.qlexpress4.runtime.ReflectLoader
//!
//! Java `ReflectLoader` 是基于 `java.lang.reflect` 的成员加载器：
//! 持有一个 `QLSecurityStrategy` 与 `allowPrivateAccess` 标志，
//! 对外暴露 `loadConstructor`/`loadMethod`/`loadField`/`loadJavaField`/
//! `loadFieldReflectCache`/`securityFilter` 等方法，
//! 在 QVM 执行 `MethodInvokeInstruction`/`GetFieldInstruction` 等指令时被调用。
//!
//! **本文件不迁移**（SPEC §4 🚫）。Rust 无 JVM 反射，QLExpress 的反射成员分派
//! 全部由显式注册的 [`crate::runtime::native_registry::NativeRegistry`] 承载：
//!
//! | Java `ReflectLoader` 方法 | Rust 化替代 |
//! |---------------------------|------------|
//! | `loadConstructor(Class, args)` | `NativeRegistry::find_constructor(type, args)` |
//! | `loadMethod(Class, name, args)` | `NativeRegistry::find_method(type, name, args)` |
//! | `loadField(Class, name)` | `NativeRegistry::find_field(type, name)` |
//! | `loadJavaField` / `loadFieldReflectCache` | `NativeRegistry::find_field`（直接走注册表，无反射缓存） |
//! | `securityFilter(IMethod)` / `securityFilter(Field)` | `NativeRegistry` 在分派期由注入的 `QLSecurityStrategy` 过滤 |
//! | `setMethodAccessible` / `setFieldAccessible` | Rust 侧无访问修饰符运行时反射；`AccessMode` 在 `NativeRegistry` 注册期决定 |
//!
//! 保留本占位文件仅用于"对象级对照表"的结构对齐（SPEC §2），
//! 文件头注释即唯一的对外说明；**不要在此文件内定义任何业务逻辑**。

#![allow(dead_code)]

/// 仅作为类型身份占位，对应 Java `com.alibaba.qlexpress4.runtime.ReflectLoader`。
///
/// 该 enum 不参与任何运行时逻辑；所有反射成员分派请使用
/// [`crate::runtime::native_registry::NativeRegistry`]。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ReflectLoader {}

impl ReflectLoader {
    /// 返回 Java 类全限定名，供 doc 引用统一。
    pub const JAVA_FQN: &'static str = "com.alibaba.qlexpress4.runtime.ReflectLoader";

    /// 返回 Rust 化替代的全限定路径。
    pub const RUST_REPLACEMENT: &'static str = "crate::runtime::native_registry::NativeRegistry";
}
