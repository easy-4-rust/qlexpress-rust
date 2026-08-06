//! 已存在变量作用域栈,对应 Java `com.alibaba.qlexpress4.aparser.ExistStack`。
//! 职责:编译期判断变量名在当前作用域链中是否可见。
//! `ExistVarStack` 为 Java 各 Visitor 中重复的 `ExistVarStack`/`ExistFunctionStack`
//! 私有实现类的 Rust 统一实现(Java 无独立顶层类,此处聚合并注释说明)。
//! 本文件由 `scope_stack_visitor.rs` 拆分而来(SPEC §5.5 一类一文件),仅移动代码与补充中文注释,行为完全一致。

use std::collections::HashSet;
use std::rc::Rc;

/// 已存在变量作用域栈契约。对应 Java: com.alibaba.qlexpress4.aparser.ExistStack
/// (含其实现类暴露的 `add` 操作)
pub trait ExistStack: Sized {
    /// 压入一个子作用域。对应 Java 方法 `push`。
    /// Java `push`: a child scope.
    fn push(&self) -> Self;
    /// 弹出到父作用域；根作用域返回 `None`，对应 Java 实现返回 `null`。
    fn pop(&self) -> Option<Self>;
    /// `var_name` 在此作用域链中是否可见？Java `HashSet` 允许保存并查询 `null`。
    /// Java `exist`: is `var_name` visible in this scope chain?
    fn exist(&self, var_name: Option<&str>) -> bool;
    /// 在当前(栈顶)作用域声明 `var_name`。对应 Java 方法 `add`。
    /// Declare `var_name` in the current (top) scope.
    fn add(&mut self, var_name: Option<String>);
}

pub use super::exist_var_stack::ExistVarStack;

impl ExistVarStack {
    /// 构造根作用域。对应 Java `new ExistVarStack(null)`。
    /// Java `new ExistVarStack(null)` — a root scope.
    pub fn root() -> Self {
        ExistVarStack::default()
    }
}

impl ExistStack for ExistVarStack {
    fn push(&self) -> Self {
        ExistVarStack {
            parent: Some(Rc::new(self.clone())),
            exist_vars: HashSet::new(),
        }
    }

    fn pop(&self) -> Option<Self> {
        self.parent.as_ref().map(|parent| (**parent).clone())
    }

    fn exist(&self, var_name: Option<&str>) -> bool {
        if self
            .exist_vars
            .iter()
            .any(|candidate| candidate.as_deref() == var_name)
        {
            return true;
        }
        self.parent
            .as_ref()
            .map(|parent| parent.exist(var_name))
            .unwrap_or(false)
    }

    fn add(&mut self, var_name: Option<String>) {
        self.exist_vars.insert(var_name);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn preserves_java_scope_chain_duplicates_and_null_names() {
        let mut root = ExistVarStack::root();
        assert!(!root.exist(Some("root")));
        assert!(!root.exist(None));
        assert!(root.pop().is_none());

        root.add(Some("root".to_string()));
        root.add(Some("root".to_string()));
        root.add(None);
        assert!(root.exist(Some("root")));
        assert!(root.exist(None));

        let mut child = root.push();
        assert!(child.exist(Some("root")));
        child.add(Some("child".to_string()));
        assert!(child.exist(Some("child")));
        let parent = child.pop().expect("child must retain its parent");
        assert!(parent.exist(Some("root")));
        assert!(!parent.exist(Some("child")));
        assert!(parent.exist(None));
    }
}
