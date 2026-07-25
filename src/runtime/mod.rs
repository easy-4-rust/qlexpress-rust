//! Runtime model mirroring Java `com.alibaba.qlexpress4.runtime`.
//!
//! Stage 3a adds the instruction set, the QVM (`qvm_runtime`), the scope
//! chain (`scope`, `qvm_global_scope`), contexts (`qcontext`,
//! `delegate_qcontext`), lambdas (`qlambda`, `data/lambda`), member access
//! (`member`, replacing Java reflection per SPEC §4) and VM utilities
//! (`util`).

pub mod data;
pub mod delegate_qcontext;
pub mod function;
pub mod instruction;
pub mod left_value;
pub mod member;
pub mod operator;
pub mod parameters;
pub mod qcontext;
pub mod qlambda;
pub mod qvm_global_scope;
pub mod qvm_runtime;
pub mod scope;
pub mod trace;
pub mod util;
pub mod value;

pub use delegate_qcontext::DelegateQContext;
pub use function::{CustomFunction, LazyArgCustomFunction};
pub use left_value::LeftValue;
pub use parameters::Parameters;
pub use qcontext::QContext;
pub use qlambda::{QLambda, QLambdaDefinition, QLambdaDefinitionInner, QLambdaTrace};
pub use qvm_global_scope::QvmGlobalScope;
pub use qvm_runtime::{QvmRuntime, QRuntime};
pub use value::{DataValue, NativeObject, QValue, Value};
