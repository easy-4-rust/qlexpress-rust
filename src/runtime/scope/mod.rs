//! Scope chain, mirroring Java `com.alibaba.qlexpress4.runtime.scope`
//! (`QScope` + `QvmBlockScope`). The global scope lives in
//! `runtime/qvm_global_scope.rs`.
//!
//! Java models scopes as an interface hierarchy with the operand stack
//! (`FixedSizeStack`) shared between a scope and its `newScope()` children.
//! Rust models the chain as [`ScopeRef`] nodes (`Rc<RefCell<Scope>>`) whose
//! `kind` is either the global scope or a block scope; the operand stack is
//! an `Rc<RefCell<Vec<QValue>>>` shared exactly like Java's reused
//! `FixedSizeStack` (the `Vec` grows dynamically instead of being
//! fixed-size — see Stage-3a notes).

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use crate::runtime::data::convert::obj_type_convertor::TargetType;
use crate::runtime::data::AssignableDataValue;
use crate::runtime::function::CustomFunction;
use crate::runtime::left_value::LeftValue;
use crate::runtime::parameters::Parameters;
use crate::runtime::qvm_global_scope::QvmGlobalScope;
use crate::runtime::value::{DataValue, QValue};

/// Shared reference to a scope node.
pub type ScopeRef = Rc<RefCell<Scope>>;

/// Symbol table of a scope (Java `Map<String, Value> symbolTable`).
pub type SymbolTable = HashMap<String, Rc<RefCell<dyn LeftValue>>>;

/// One node of the scope chain.
pub struct Scope {
    parent: Option<ScopeRef>,
    /// Operand stack; shared with `new_scope` children (Java `reuseStack`).
    stack: Rc<RefCell<Vec<QValue>>>,
    kind: ScopeKind,
}

/// Scope node payload: the global scope or a block scope
/// (Java `QvmBlockScope`).
pub enum ScopeKind {
    Global(QvmGlobalScope),
    Block(QvmBlockScope),
}

/// Block scope data, mirroring Java `QvmBlockScope`.
pub struct QvmBlockScope {
    symbol_table: SymbolTable,
    function_table: HashMap<String, Rc<dyn CustomFunction>>,
}

impl QvmBlockScope {
    pub fn new(symbol_table: SymbolTable) -> Self {
        QvmBlockScope {
            symbol_table,
            function_table: HashMap::new(),
        }
    }

    pub fn symbol_table(&self) -> &SymbolTable {
        &self.symbol_table
    }

    pub fn function_table(&self) -> &HashMap<String, Rc<dyn CustomFunction>> {
        &self.function_table
    }
}

impl Scope {
    /// Create the root (global) scope node with a fresh operand stack.
    pub fn global(global: QvmGlobalScope) -> ScopeRef {
        Rc::new(RefCell::new(Scope {
            parent: None,
            stack: Rc::new(RefCell::new(Vec::new())),
            kind: ScopeKind::Global(global),
        }))
    }

    /// Java `new QvmBlockScope(parent, symbolTable, maxStackSize, ...)`:
    /// child scope with a **fresh** operand stack (used by lambda
    /// invocation and for/while scopes).
    pub fn block_fresh_stack(parent: &ScopeRef, symbol_table: SymbolTable) -> ScopeRef {
        Rc::new(RefCell::new(Scope {
            parent: Some(Rc::clone(parent)),
            stack: Rc::new(RefCell::new(Vec::new())),
            kind: ScopeKind::Block(QvmBlockScope::new(symbol_table)),
        }))
    }

    /// Java `QvmBlockScope.newScope()`: child scope **reusing** the parent
    /// operand stack.
    pub fn new_scope(this: &ScopeRef) -> ScopeRef {
        let stack = Rc::clone(&this.borrow().stack);
        Rc::new(RefCell::new(Scope {
            parent: Some(Rc::clone(this)),
            stack,
            kind: ScopeKind::Block(QvmBlockScope::new(HashMap::new())),
        }))
    }

    /// Java `getParent()`.
    pub fn parent(this: &ScopeRef) -> Option<ScopeRef> {
        this.borrow().parent.as_ref().map(Rc::clone)
    }

    /// Java `getSymbol`: local symbol table first, then the parent chain;
    /// the global scope creates the variable when absent.
    pub fn get_symbol(this: &ScopeRef, var_name: &str) -> Option<Rc<RefCell<dyn LeftValue>>> {
        let (local, parent) = {
            let mut borrowed = this.borrow_mut();
            let local = match &mut borrowed.kind {
                ScopeKind::Global(global) => Some(global.get_symbol(var_name)),
                ScopeKind::Block(block) => block.symbol_table.get(var_name).map(Rc::clone),
            };
            (local, borrowed.parent.as_ref().map(Rc::clone))
        };
        match (local, parent) {
            (Some(symbol), _) => Some(symbol),
            (None, Some(parent)) => Self::get_symbol(&parent, var_name),
            (None, None) => None,
        }
    }

    /// Java default `getSymbolValue`: inner data, `None` when absent
    /// (Java `null`).
    pub fn get_symbol_value(this: &ScopeRef, var_name: &str) -> Option<DataValue> {
        Self::get_symbol(this, var_name).map(|symbol| symbol.borrow().get())
    }

    /// Java `QvmBlockScope.defineLocalSymbol`.
    pub fn define_local_symbol(
        this: &ScopeRef,
        var_name: &str,
        var_clz: Option<TargetType>,
        value: DataValue,
    ) {
        let mut borrowed = this.borrow_mut();
        match &mut borrowed.kind {
            ScopeKind::Global(global) => global.define_local_symbol(var_name),
            ScopeKind::Block(block) => {
                let slot: Rc<RefCell<dyn LeftValue>> = match var_clz {
                    Some(clz) => Rc::new(RefCell::new(AssignableDataValue::with_type(
                        var_name, value, clz,
                    ))),
                    None => Rc::new(RefCell::new(AssignableDataValue::new(var_name, value))),
                };
                block.symbol_table.insert(var_name.to_string(), slot);
            }
        }
    }

    /// Java `QvmBlockScope.defineFunction`.
    pub fn define_function(
        this: &ScopeRef,
        function_name: &str,
        function: Rc<dyn CustomFunction>,
    ) {
        let mut borrowed = this.borrow_mut();
        match &mut borrowed.kind {
            ScopeKind::Global(global) => global.define_function(function_name),
            ScopeKind::Block(block) => {
                block.function_table.insert(function_name.to_string(), function);
            }
        }
    }

    /// Java `getFunction`: local table first, then the parent chain.
    pub fn get_function(this: &ScopeRef, function_name: &str) -> Option<Rc<dyn CustomFunction>> {
        let (local, parent) = {
            let borrowed = this.borrow();
            let local = match &borrowed.kind {
                ScopeKind::Global(global) => global.get_function(function_name),
                ScopeKind::Block(block) => block.function_table.get(function_name).cloned(),
            };
            (local, borrowed.parent.as_ref().map(Rc::clone))
        };
        match (local, parent) {
            (Some(function), _) => Some(function),
            (None, Some(parent)) => Self::get_function(&parent, function_name),
            (None, None) => None,
        }
    }

    /// Java `getFunctionTable`: the current scope's own table (not merged
    /// with parents).
    pub fn function_table(this: &ScopeRef) -> HashMap<String, Rc<dyn CustomFunction>> {
        let borrowed = this.borrow();
        match &borrowed.kind {
            ScopeKind::Global(global) => global.function_table().clone(),
            ScopeKind::Block(block) => block.function_table.clone(),
        }
    }

    /// The shared operand stack handle.
    pub fn stack(this: &ScopeRef) -> Rc<RefCell<Vec<QValue>>> {
        Rc::clone(&this.borrow().stack)
    }

    /// Java `push(Value)`.
    pub fn push(this: &ScopeRef, value: QValue) {
        Self::stack(this).borrow_mut().push(value);
    }

    /// Java `pop()`: top element. Panics on empty stack, like Java's
    /// `FixedSizeStack` array access.
    pub fn pop(this: &ScopeRef) -> QValue {
        Self::stack(this)
            .borrow_mut()
            .pop()
            .expect("operand stack underflow")
    }

    /// Java `pop(int number)`: the top `number` elements in stack order
    /// (deepest first).
    pub fn pop_n(this: &ScopeRef, number: usize) -> Parameters {
        let mut stack = Self::stack(this);
        let mut stack = stack.borrow_mut();
        let len = stack.len();
        assert!(number <= len, "operand stack underflow");
        Parameters::new(stack.split_off(len - number))
    }

    /// Java `peek()`: top element without popping.
    pub fn peek(this: &ScopeRef) -> QValue {
        Self::stack(this)
            .borrow()
            .last()
            .cloned()
            .expect("operand stack underflow")
    }

    /// Whether the operand stack is empty.
    pub fn stack_is_empty(this: &ScopeRef) -> bool {
        Self::stack(this).borrow().is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn root() -> ScopeRef {
        Scope::global(QvmGlobalScope::empty())
    }

    #[test]
    fn symbol_defined_in_child_is_invisible_in_parent() {
        let global = root();
        let child = Scope::new_scope(&global);
        Scope::define_local_symbol(&child, "x", None, DataValue::Int(1));
        assert_eq!(
            Scope::get_symbol_value(&child, "x"),
            Some(DataValue::Int(1))
        );
        // Parent (global) auto-creates an independent slot (Java behavior).
        assert_eq!(
            Scope::get_symbol_value(&global, "x"),
            Some(DataValue::Null)
        );
    }

    #[test]
    fn new_scope_shares_operand_stack_like_java() {
        let global = root();
        let child = Scope::new_scope(&global);
        Scope::push(&global, DataValue::Int(7).into());
        assert_eq!(Scope::peek(&child).get(), DataValue::Int(7));
        assert_eq!(Scope::pop(&child).get(), DataValue::Int(7));
        assert!(Scope::stack_is_empty(&global));
    }

    #[test]
    fn fresh_stack_child_does_not_share() {
        let global = root();
        let child = Scope::block_fresh_stack(&global, HashMap::new());
        Scope::push(&global, DataValue::Int(1).into());
        assert!(Scope::stack_is_empty(&child));
    }

    #[test]
    fn global_scope_autocreates_variables() {
        let global = root();
        let a = Scope::get_symbol(&global, "a").expect("created");
        a.borrow_mut().set_inner(DataValue::Long(5));
        let b = Scope::get_symbol(&global, "a").expect("same slot");
        assert_eq!(b.borrow().get(), DataValue::Long(5));
    }

    #[test]
    fn pop_n_preserves_stack_order() {
        let global = root();
        Scope::push(&global, DataValue::Int(1).into());
        Scope::push(&global, DataValue::Int(2).into());
        Scope::push(&global, DataValue::Int(3).into());
        let params = Scope::pop_n(&global, 2);
        assert_eq!(params.get_value(0), DataValue::Int(2));
        assert_eq!(params.get_value(1), DataValue::Int(3));
        assert_eq!(Scope::peek(&global).get(), DataValue::Int(1));
    }
}
