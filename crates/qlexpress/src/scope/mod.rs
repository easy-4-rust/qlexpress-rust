//! Scope chain, mirroring Java `com.alibaba.qlexpress4.runtime.scope`
//! (`QScope` + `QvmBlockScope`). The global scope lives in
//! `runtime/qvm_global_scope.rs`.
//!
//! Java models scopes as an interface hierarchy with the operand stack
//! (`FixedSizeStack`) shared between a scope and its `newScope()` children.
//! Rust models the chain as [`ScopeRef`] nodes (`Rc<RefCell<QScope>>`) whose
//! `kind` is either the global scope or a block scope; the operand stack is
//! an `Rc<RefCell<Vec<QValue>>>` shared exactly like Java's reused
//! `FixedSizeStack` (the `Vec` grows dynamically instead of being
//! fixed-size — see Stage-3a notes).
//!
//! 一类一文件(SPEC §5.5):本模块只做 mod 声明 + re-export。

pub mod q_scope;
pub mod q_scope_kind;
pub mod qvm_block_scope;

pub use q_scope::{QScope, ScopeRef, SymbolTable};
pub use q_scope_kind::QScopeKind;
pub use qvm_block_scope::QvmBlockScope;
