//! 带作用域 Visitor 的可替换栈状态。

use super::exist_stack::ExistStack;

/// Java `ScopeStackVisitor.existStack` 字段的 Rust 状态容器。
/// 对应 Java：`com.alibaba.qlexpress4.aparser.ScopeStackVisitor`。
#[derive(Clone, Debug)]
pub struct ScopeStack<S: ExistStack> {
    pub(crate) stack: S,
}
