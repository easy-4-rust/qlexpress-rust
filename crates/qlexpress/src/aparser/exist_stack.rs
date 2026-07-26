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
    /// 弹出到父作用域;在根作用域上调用会 panic(Java 会 NPE)。对应 Java 方法 `pop`。
    /// Java `pop`: the parent scope; panics on the root (Java would NPE).
    fn pop(&self) -> Self;
    /// `var_name` 在此作用域链中是否可见?对应 Java 方法 `exist`。
    /// Java `exist`: is `var_name` visible in this scope chain?
    fn exist(&self, var_name: &str) -> bool;
    /// 在当前(栈顶)作用域声明 `var_name`。对应 Java 方法 `add`。
    /// Declare `var_name` in the current (top) scope.
    fn add(&mut self, var_name: String);
}

/// 持久化作用域栈,out-var/out-function 各 Visitor 共用。
/// 对应 Java: 各 Visitor 中重复的 `ExistVarStack`/`ExistFunctionStack` 私有类
/// (Java 无独立顶层类,此处聚合为统一实现)。
///
/// Persistent scope stack shared by the out-var/out-function visitors,
/// mirroring the duplicated `ExistVarStack`/`ExistFunctionStack` private
/// classes in the Java visitors.
#[derive(Clone, Debug, Default)]
pub struct ExistVarStack {
    parent: Option<Rc<ExistVarStack>>,
    exist_vars: HashSet<String>,
}

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

    fn pop(&self) -> Self {
        match &self.parent {
            Some(parent) => (**parent).clone(),
            // Java 在根上 pop 会 NPE;Rust 显式 panic,语义等价(编译期不会发生)
            None => panic!("ExistStack.pop on root scope"),
        }
    }

    fn exist(&self, var_name: &str) -> bool {
        if self.exist_vars.contains(var_name) {
            return true;
        }
        self.parent
            .as_ref()
            .map(|parent| parent.exist(var_name))
            .unwrap_or(false)
    }

    fn add(&mut self, var_name: String) {
        self.exist_vars.insert(var_name);
    }
}
