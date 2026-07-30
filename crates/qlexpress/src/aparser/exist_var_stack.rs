//! 外部变量与函数收集 Visitor 共用的持久化作用域栈。

use std::collections::HashSet;
use std::rc::Rc;

/// Java 多个 Visitor 内部 `ExistVarStack`/`ExistFunctionStack` 的统一实现。
#[derive(Clone, Debug, Default)]
pub struct ExistVarStack {
    pub(crate) parent: Option<Rc<ExistVarStack>>,
    pub(crate) exist_vars: HashSet<String>,
}
