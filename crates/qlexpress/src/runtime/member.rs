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

pub use crate::runtime::class_ref::ClassRef;
pub use crate::runtime::meta_class::{MetaClass, as_meta_class};
pub use crate::runtime::native_registry::NativeRegistry;
pub use crate::runtime::native_type::{
    NativeConstructor, NativeConstructorCandidate, NativeFieldGetter, NativeFieldSetter,
    NativeMethod, NativeMethodCandidate, NativeStaticField, NativeType,
};
pub use crate::runtime::ql_express_native_type::QLExpressNativeType;
pub use crate::runtime::ql_express_registry_ext::QLExpressRegistryExt;
pub use crate::runtime::util::method_invoke_utils::{find_method_and_invoke, invoke_native_method};

impl QLExpressRegistryExt for NativeRegistry {
    fn register_qlexpress_type<T: QLExpressNativeType>(&mut self) {
        self.register_type(T::build_native_type());
    }
}
