//! Execution context passed to every instruction, mirroring Java
//! `com.alibaba.qlexpress4.runtime.QContext` (= `QScope` + `QRuntime`).

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use crate::ql_options::Attachments;
use crate::runtime::data::convert::obj_type_convertor::TargetType;
use crate::runtime::function::CustomFunction;
use crate::runtime::left_value::LeftValue;
use crate::runtime::member::NativeRegistry;
use crate::runtime::parameters::Parameters;
use crate::runtime::qvm_runtime::QvmRuntime;
use crate::runtime::scope::ScopeRef;
use crate::runtime::trace::QTraces;
use crate::runtime::value::{DataValue, QValue};

/// Full execution context, mirroring Java `QContext`.
///
/// Method set = Java `QRuntime` (time/attachments/registry/traces) + Java
/// `QScope` (symbols, functions, operand stack, scope chain) + Java
/// `QContext` (`getCurrentScope`/`closeScope`).
pub trait QContext {
    // ---- Java QRuntime ----

    /// Java `scriptStartTimeStamp()`.
    fn script_start_time_stamp(&self) -> i64;

    /// Java `attachment()`.
    fn attachment(&self) -> &Attachments;

    /// Java `getReflectLoader()` (SPEC §4: explicit native registry).
    fn registry(&self) -> &Rc<NativeRegistry>;

    /// Java `getTraces()`.
    fn traces(&self) -> &QTraces;

    /// The shared root runtime (used to build lambda-captured contexts,
    /// Java `new DelegateQContext(qContext, ...)`).
    fn q_runtime(&self) -> &Rc<QvmRuntime>;

    // ---- Java QScope ----

    /// Java `getSymbol`: assignable symbol by name; the global scope
    /// auto-creates it when absent.
    fn get_symbol(&mut self, var_name: &str) -> Option<Rc<RefCell<dyn LeftValue>>>;

    /// Java default `getSymbolValue`.
    fn get_symbol_value(&mut self, var_name: &str) -> Option<DataValue> {
        self.get_symbol(var_name).map(|symbol| symbol.borrow().get())
    }

    /// Java `defineLocalSymbol`.
    fn define_local_symbol(
        &mut self,
        var_name: &str,
        var_clz: Option<TargetType>,
        value: DataValue,
    );

    /// Java `defineFunction`.
    fn define_function(&mut self, function_name: &str, function: Rc<dyn CustomFunction>);

    /// Java `getFunction`.
    fn get_function(&self, function_name: &str) -> Option<Rc<dyn CustomFunction>>;

    /// Java `getFunctionTable` (current scope's own table).
    fn function_table(&self) -> HashMap<String, Rc<dyn CustomFunction>>;

    /// Java `push(Value)`.
    fn push(&mut self, value: QValue);

    /// Java `pop(int number)`.
    fn pop_n(&mut self, number: usize) -> Parameters;

    /// Java `pop()`.
    fn pop(&mut self) -> QValue;

    /// Java `peek()`.
    fn peek(&self) -> QValue;

    /// Java `getParent()`.
    fn parent_scope(&self) -> Option<ScopeRef>;

    /// Java `QScope.newScope()` + `DelegateQContext.newScope()`:
    /// opens a child scope (sharing the operand stack) and makes it current.
    fn new_scope(&mut self) -> ScopeRef;

    // ---- Java QContext ----

    /// Java `getCurrentScope()`.
    fn current_scope(&self) -> ScopeRef;

    /// Java `closeScope()`: the parent scope becomes current.
    fn close_scope(&mut self);

    /// Replace the current scope (used when entering lambda/for scopes,
    /// mirroring Java's `new DelegateQContext(qContext, newScope)`).
    fn set_current_scope(&mut self, scope: ScopeRef);
}
