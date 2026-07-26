//! 成员访问(字段/方法/构造器)兼容门面。
//!
//! SPEC §5.5.6 拆分说明:原 `member.rs` 聚合实现已按 Java 类边界迁出——
//! - `runtime/class_ref.rs`(`ClassRef`,Rust 新增,对应 Java `Class<?>` 角色)
//! - `runtime/meta_class.rs`(对应 `runtime/MetaClass.java`)
//! - `runtime/native_type.rs`(`NativeType`/`NativeMethod` 等,SPEC §4)
//! - `runtime/native_registry.rs`(`NativeRegistry`,对应 `ReflectLoader`
//!   + `ClassSupplier` 职责,含内建方法子集)
//! - `runtime/native_object.rs`(`NativeObject`,SPEC §4/§6)
//! - `runtime/member_resolver.rs` + `runtime/i_method.rs` +
//!   `runtime/jvm_i_method.rs`(对应 `MemberResolver`/`IMethod`/`JvmIMethod`)
//! - `runtime/util/method_invoke_utils.rs`(对应 `runtime/util/MethodInvokeUtils.java`)
//! - `member/`(对应 Java `member/` 包的 `FieldHandler`/`MethodHandler`)
//!
//! 本文件仅 re-export 以保持 Stage 3a/4 既有 `crate::runtime::member::*`
//! 路径兼容,不含任何实现。

use std::any::TypeId;
use std::rc::Rc;

use crate::runtime::native_object::NativeObject;

pub use crate::runtime::class_ref::ClassRef;
pub use crate::runtime::meta_class::{as_meta_class, MetaClass};
pub use crate::runtime::native_registry::NativeRegistry;
pub use crate::runtime::native_type::{
    NativeConstructor, NativeFieldGetter, NativeMethod, NativeType,
};
pub use crate::runtime::util::method_invoke_utils::{find_method_and_invoke, invoke_native_method};

/// `QLExpressNativeType` 接口的 Rust 实现，保留对应对象的领域职责与公开契约。
/// 对应或承接 Java 源文件：`com/alibaba/qlexpress4/runtime/QRuntime.java`；具体对象路径见 `docs/对象级对照表.md`。
/// Trait auto-derivable via `#[derive(QLExpressType)]`
/// (proc-macro crate `qlexpress-derive`).
///
/// Each implementor declares how to build the [`NativeType`] description
/// for the script engine. The trait is also a [`NativeObject`] so that
/// the engine can dispatch field/method access through the registry
/// *or* through `DataValue::Object(Rc<RefCell<T>>)` directly.
pub trait QLExpressNativeType: NativeObject + 'static {
    /// Canonical Java-style name (e.g. `"com.example.Calc"`).
    /// Defaults to the Rust type identifier when unspecified.
    const QL_TYPE_NAME: &'static str;

    /// 处理 ql type id 对应的接口职责。
    /// 无显式参数；返回：`TypeId`。
    /// Rust 原生接口适配；承接当前 trait 既有 rustdoc 标注的 Java 职责。
    /// `TypeId` of the implementor. Used by the runtime to downcast
    /// `DataValue::Object` payloads back to `&mut T`.
    fn ql_type_id(&self) -> TypeId {
        TypeId::of::<Self>()
    }

    /// 构建 native type。
    /// 无显式参数；返回：`NativeType`。
    /// Rust 原生接口适配；承接当前 trait 既有 rustdoc 标注的 Java 职责。
    /// Build the `NativeType` description for registration into a
    /// [`NativeRegistry`].
    fn build_native_type() -> NativeType;

    /// 处理 into data value 对应的接口职责。
    /// 无显式参数；返回：`crate::runtime::value::DataValue`。
    /// Rust 原生接口适配；承接当前 trait 既有 rustdoc 标注的 Java 职责。
    /// Wrap `self` into a `DataValue::Object` cell so the engine can hold
    /// a strong reference. The default impl wraps via `Rc<RefCell<T>>`;
    /// override only when you need a custom cell (e.g. for tracing).
    ///
    /// Note: takes `Self` by value. The trait is consumed by the caller
    /// to build the cell, so users that need shared ownership must clone
    /// first.
    fn into_data_value(self) -> crate::runtime::value::DataValue
    where
        Self: Sized,
    {
        use std::cell::RefCell;
        let cell: Rc<RefCell<dyn NativeObject>> = Rc::new(RefCell::new(self));
        crate::runtime::value::DataValue::Object(cell)
    }
}

/// `QLExpressRegistryExt` 接口的 Rust 实现，保留对应对象的领域职责与公开契约。
/// 对应或承接 Java 源文件：`com/alibaba/qlexpress4/runtime/QRuntime.java`；具体对象路径见 `docs/对象级对照表.md`。
/// Convenience: register `T` into a registry by-name.
pub trait QLExpressRegistryExt {
    /// 将派生宏生成的宿主类型元数据注册进 QLExpress 注册表。
    /// 替代 Java `ReflectLoader` 的运行时反射发现。
    fn register_qlexpress_type<T: QLExpressNativeType>(&mut self);
}

impl QLExpressRegistryExt for NativeRegistry {
    fn register_qlexpress_type<T: QLExpressNativeType>(&mut self) {
        self.register_type(T::build_native_type());
    }
}
