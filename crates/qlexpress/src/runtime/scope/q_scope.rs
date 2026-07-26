//! 作用域链节点,对应 Java `com.alibaba.qlexpress4.runtime.scope.QScope`。
//! 职责:符号表/函数表查找、操作数栈存取、`newScope` 子作用域。
//! 本文件由 `scope/mod.rs` 拆分而来(SPEC §5.5 一类一文件),仅移动代码、
//! 对齐命名(`Scope` -> `QScope`)与补充中文注释,行为完全一致。

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use crate::exception::QLException;
use crate::runtime::data::convert::obj_type_convertor::TargetType;
use crate::runtime::data::AssignableDataValue;
use crate::runtime::function::CustomFunction;
use crate::runtime::left_value::LeftValue;
use crate::runtime::parameters::Parameters;
use crate::runtime::qvm_global_scope::QvmGlobalScope;
use crate::runtime::scope::qvm_block_scope::QvmBlockScope;
use crate::runtime::value::{DataValue, QValue};

/// 作用域节点的共享引用(Java 侧直接持有 `QScope` 引用;Rust 用 `Rc<RefCell>` 实现共享可变)。
pub type ScopeRef = Rc<RefCell<QScope>>;

/// 作用域符号表。对应 Java: `QScope` 的 `Map<String, Value> symbolTable`。
/// Symbol table of a scope (Java `Map<String, Value> symbolTable`).
pub type SymbolTable = HashMap<String, Rc<RefCell<dyn LeftValue>>>;

/// 作用域链上的一个节点。对应 Java: com.alibaba.qlexpress4.runtime.scope.QScope
/// (Java 为接口体系,操作数栈 `FixedSizeStack` 在作用域与其 `newScope()` 子作用域间共享;
/// Rust 以 `Rc<RefCell<Vec<QValue>>>` 复现该共享语义)
/// One node of the scope chain.
pub struct QScope {
    parent: Option<ScopeRef>,
    /// Operand stack; shared with `new_scope` children (Java `reuseStack`).
    stack: Rc<RefCell<Vec<QValue>>>,
    kind: QScopeKind,
}

/// 作用域节点负载:全局作用域或块作用域(Java `QvmBlockScope`)。
/// Java 无同名类(Rust 适配枚举)。
pub enum QScopeKind {
    Global(QvmGlobalScope),
    Block(QvmBlockScope),
}

impl QScope {
    /// 创建带全新操作数栈的根(全局)作用域节点。对应 Java `QvmGlobalScope` 作为根作用域。
    /// Create the root (global) scope node with a fresh operand stack.
    pub fn global(global: QvmGlobalScope) -> ScopeRef {
        Rc::new(RefCell::new(QScope {
            parent: None,
            stack: Rc::new(RefCell::new(Vec::new())),
            kind: QScopeKind::Global(global),
        }))
    }

    /// Java `new QvmBlockScope(parent, symbolTable, maxStackSize, ...)`:
    /// child scope with a **fresh** operand stack (used by lambda
    /// invocation and for/while scopes).
    pub fn block_fresh_stack(parent: &ScopeRef, symbol_table: SymbolTable) -> ScopeRef {
        Rc::new(RefCell::new(QScope {
            parent: Some(Rc::clone(parent)),
            stack: Rc::new(RefCell::new(Vec::new())),
            kind: QScopeKind::Block(QvmBlockScope::new(symbol_table)),
        }))
    }

    /// Java `QvmBlockScope.newScope()`: child scope **reusing** the parent
    /// operand stack.
    pub fn new_scope(this: &ScopeRef) -> ScopeRef {
        let stack = Rc::clone(&this.borrow().stack);
        Rc::new(RefCell::new(QScope {
            parent: Some(Rc::clone(this)),
            stack,
            kind: QScopeKind::Block(QvmBlockScope::new(HashMap::new())),
        }))
    }

    /// Java `getParent()`.
    pub fn parent(this: &ScopeRef) -> Option<ScopeRef> {
        this.borrow().parent.as_ref().map(Rc::clone)
    }

    /// Java `getSymbol`: local symbol table first, then the parent chain;
    /// the global scope creates the variable when absent.
    ///
    /// 返回 `Result`(Stage 5a 接线改动):全局作用域的外部变量查询走
    /// `ExpressContext`,其动态求值(如 `DynamicVariableContext`)可能失败,
    /// 与 Java 中 `ExpressContext.get` 抛运行期异常上抛一致。
    pub fn get_symbol(
        this: &ScopeRef,
        var_name: &str,
    ) -> Result<Option<Rc<RefCell<dyn LeftValue>>>, QLException> {
        let (local, parent) = {
            let mut borrowed = this.borrow_mut();
            let local = match &mut borrowed.kind {
                QScopeKind::Global(global) => Some(global.get_symbol(var_name)?),
                QScopeKind::Block(block) => block.symbol_table().get(var_name).map(Rc::clone),
            };
            (local, borrowed.parent.as_ref().map(Rc::clone))
        };
        Ok(match (local, parent) {
            (Some(symbol), _) => Some(symbol),
            (None, Some(parent)) => Self::get_symbol(&parent, var_name)?,
            (None, None) => None,
        })
    }

    /// Java default `getSymbolValue`: inner data, `None` when absent
    /// (Java `null`).
    pub fn get_symbol_value(
        this: &ScopeRef,
        var_name: &str,
    ) -> Result<Option<DataValue>, QLException> {
        Ok(Self::get_symbol(this, var_name)?.map(|symbol| symbol.borrow().get()))
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
            QScopeKind::Global(global) => global.define_local_symbol(var_name),
            QScopeKind::Block(block) => {
                let slot: Rc<RefCell<dyn LeftValue>> = match var_clz {
                    Some(clz) => Rc::new(RefCell::new(AssignableDataValue::with_type(
                        var_name, value, clz,
                    ))),
                    None => Rc::new(RefCell::new(AssignableDataValue::new(var_name, value))),
                };
                block.symbol_table_mut().insert(var_name.to_string(), slot);
            }
        }
    }

    /// Java `QvmBlockScope.defineFunction`.
    pub fn define_function(this: &ScopeRef, function_name: &str, function: Rc<dyn CustomFunction>) {
        let mut borrowed = this.borrow_mut();
        match &mut borrowed.kind {
            QScopeKind::Global(global) => global.define_function(function_name),
            QScopeKind::Block(block) => {
                block
                    .function_table_mut()
                    .insert(function_name.to_string(), function);
            }
        }
    }

    /// Java `getFunction`: local table first, then the parent chain.
    pub fn get_function(this: &ScopeRef, function_name: &str) -> Option<Rc<dyn CustomFunction>> {
        let (local, parent) = {
            let borrowed = this.borrow();
            let local = match &borrowed.kind {
                QScopeKind::Global(global) => global.get_function(function_name),
                QScopeKind::Block(block) => block.function_table().get(function_name).cloned(),
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
            QScopeKind::Global(global) => global.function_table().clone(),
            QScopeKind::Block(block) => block.function_table().clone(),
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
        let stack = Self::stack(this);
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
        QScope::global(QvmGlobalScope::empty())
    }

    #[test]
    fn symbol_defined_in_child_is_invisible_in_parent() {
        let global = root();
        let child = QScope::new_scope(&global);
        QScope::define_local_symbol(&child, "x", None, DataValue::Int(1));
        assert_eq!(
            QScope::get_symbol_value(&child, "x").unwrap(),
            Some(DataValue::Int(1))
        );
        // Parent (global) auto-creates an independent slot (Java behavior).
        assert_eq!(
            QScope::get_symbol_value(&global, "x").unwrap(),
            Some(DataValue::Null)
        );
    }

    #[test]
    fn new_scope_shares_operand_stack_like_java() {
        let global = root();
        let child = QScope::new_scope(&global);
        QScope::push(&global, DataValue::Int(7).into());
        assert_eq!(QScope::peek(&child).get(), DataValue::Int(7));
        assert_eq!(QScope::pop(&child).get(), DataValue::Int(7));
        assert!(QScope::stack_is_empty(&global));
    }

    #[test]
    fn fresh_stack_child_does_not_share() {
        let global = root();
        let child = QScope::block_fresh_stack(&global, HashMap::new());
        QScope::push(&global, DataValue::Int(1).into());
        assert!(QScope::stack_is_empty(&child));
    }

    #[test]
    fn global_scope_autocreates_variables() {
        let global = root();
        let a = QScope::get_symbol(&global, "a").unwrap().expect("created");
        a.borrow_mut().set_inner(DataValue::Long(5));
        let b = QScope::get_symbol(&global, "a")
            .unwrap()
            .expect("same slot");
        assert_eq!(b.borrow().get(), DataValue::Long(5));
    }

    #[test]
    fn pop_n_preserves_stack_order() {
        let global = root();
        QScope::push(&global, DataValue::Int(1).into());
        QScope::push(&global, DataValue::Int(2).into());
        QScope::push(&global, DataValue::Int(3).into());
        let params = QScope::pop_n(&global, 2);
        assert_eq!(params.get_value(0), DataValue::Int(2));
        assert_eq!(params.get_value(1), DataValue::Int(3));
        assert_eq!(QScope::peek(&global).get(), DataValue::Int(1));
    }
}
