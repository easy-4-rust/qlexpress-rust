//! Context delegating runtime services to the root runtime and scope
//! operations to a current scope, mirroring Java
//! `com.alibaba.qlexpress4.runtime.DelegateQContext`.

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use crate::exception::QLException;
use crate::ql_options::Attachments;
use crate::runtime::data::convert::obj_type_convertor::TargetType;
use crate::runtime::function::CustomFunction;
use crate::runtime::left_value::LeftValue;
use crate::runtime::member::NativeRegistry;
use crate::runtime::parameters::Parameters;
use crate::runtime::q_runtime::QRuntime;
use crate::runtime::qcontext::QContext;
use crate::runtime::qvm_runtime::QvmRuntime;
use crate::runtime::scope::{QScope, ScopeRef};
use crate::runtime::trace::QTraces;
use crate::runtime::value::{DataValue, QValue};

/// `DelegateQContext` 结构体的 Rust 实现，保留对应对象的领域职责与公开契约。
/// 对应或承接 Java 源文件：`com/alibaba/qlexpress4/runtime/DelegateQContext.java`；具体对象路径见 `docs/对象级对照表.md`。
/// Mirroring Java `DelegateQContext`: pairs the shared [`QvmRuntime`] with
/// a mutable "current scope" pointer.
pub struct DelegateQContext {
    q_runtime: Rc<QvmRuntime>,
    q_scope: ScopeRef,
}

impl DelegateQContext {
    /// 创建对象实例。
    /// 参数：`q_runtime`、`q_scope`；返回：`Self`。
    /// 对应或承接 Java 源文件：`com/alibaba/qlexpress4/runtime/DelegateQContext.java`，构造器 `<init>`；Rust 侧按所有权与 `Result` 语义适配。
    /// Java `new DelegateQContext(qRuntime, qScope)`.
    pub fn new(q_runtime: Rc<QvmRuntime>, q_scope: ScopeRef) -> Self {
        DelegateQContext { q_runtime, q_scope }
    }
}

impl QContext for DelegateQContext {
    fn script_start_time_stamp(&self) -> i64 {
        self.q_runtime.script_start_time_stamp()
    }

    fn attachment(&self) -> &Attachments {
        self.q_runtime.attachment()
    }

    fn registry(&self) -> &Rc<NativeRegistry> {
        self.q_runtime.registry()
    }

    fn traces(&self) -> &QTraces {
        self.q_runtime.traces()
    }

    fn q_runtime(&self) -> &Rc<QvmRuntime> {
        &self.q_runtime
    }

    fn get_symbol(
        &mut self,
        var_name: &str,
    ) -> Result<Option<Rc<RefCell<dyn LeftValue>>>, QLException> {
        QScope::get_symbol(&self.q_scope, var_name)
    }

    fn get_symbol_value(&mut self, var_name: &str) -> Result<Option<DataValue>, QLException> {
        QScope::get_symbol_value(&self.q_scope, var_name)
    }

    fn define_local_symbol(
        &mut self,
        var_name: &str,
        var_clz: Option<TargetType>,
        value: DataValue,
    ) {
        QScope::define_local_symbol(&self.q_scope, var_name, var_clz, value)
    }

    fn define_function(&mut self, function_name: &str, function: Rc<dyn CustomFunction>) {
        QScope::define_function(&self.q_scope, function_name, function)
    }

    fn get_function(&self, function_name: &str) -> Option<Rc<dyn CustomFunction>> {
        QScope::get_function(&self.q_scope, function_name)
    }

    fn function_table(&self) -> HashMap<String, Rc<dyn CustomFunction>> {
        QScope::function_table(&self.q_scope)
    }

    fn push(&mut self, value: QValue) {
        QScope::push(&self.q_scope, value)
    }

    fn pop_n(&mut self, number: usize) -> Parameters {
        QScope::pop_n(&self.q_scope, number)
    }

    fn pop(&mut self) -> QValue {
        QScope::pop(&self.q_scope)
    }

    fn peek(&self) -> QValue {
        QScope::peek(&self.q_scope)
    }

    fn parent_scope(&self) -> Option<ScopeRef> {
        QScope::parent(&self.q_scope)
    }

    /// Java `qScope = qScope.newScope()`.
    fn new_scope(&mut self) -> ScopeRef {
        self.q_scope = QScope::new_scope(&self.q_scope);
        Rc::clone(&self.q_scope)
    }

    fn current_scope(&self) -> ScopeRef {
        Rc::clone(&self.q_scope)
    }

    /// Java `qScope = qScope.getParent()`.
    fn close_scope(&mut self) {
        if let Some(parent) = QScope::parent(&self.q_scope) {
            self.q_scope = parent;
        }
    }

    fn set_current_scope(&mut self, scope: ScopeRef) {
        self.q_scope = scope;
    }
}
