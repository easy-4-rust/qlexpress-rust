//! QVM 作用域节点的负载类别。

use crate::runtime::qvm_global_scope::QvmGlobalScope;
use crate::scope::qvm_block_scope::QvmBlockScope;

/// 作用域节点负载：全局作用域或块作用域。
///
/// 对应 Java: `QScope` 的实现类型 `QvmGlobalScope` 与
/// `QvmBlockScope`；Rust 用枚举保存同一封闭集合。
pub enum QScopeKind {
    /// 全局执行作用域。
    Global(QvmGlobalScope),
    /// 块级执行作用域。
    Block(QvmBlockScope),
}
