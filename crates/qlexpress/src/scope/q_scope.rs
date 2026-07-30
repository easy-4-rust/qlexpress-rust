//! 作用域链节点,对应 Java `com.alibaba.qlexpress4.runtime.scope.QScope`。
//! 职责:符号表/函数表查找、操作数栈存取、`newScope` 子作用域。
//! 本文件由 `scope/mod.rs` 拆分而来(SPEC §5.5 一类一文件),仅移动代码、
//! 对齐命名(`Scope` -> `QScope`)与补充中文注释,行为完全一致。

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

pub use super::q_scope_kind::QScopeKind;
use crate::exception::QLException;
use crate::runtime::data::convert::obj_type_convertor::TargetType;
use crate::runtime::data::AssignableDataValue;
use crate::runtime::fixed_size_stack::FixedSizeStack;
use crate::runtime::function::CustomFunction;
use crate::runtime::left_value::LeftValue;
use crate::runtime::parameters::Parameters;
use crate::runtime::qvm_global_scope::QvmGlobalScope;
use crate::runtime::scope::qvm_block_scope::QvmBlockScope;
use crate::runtime::value::{DataValue, QValue};

/// 作用域节点的共享引用(Java 侧直接持有 `QScope` 引用;Rust 用 `Rc<RefCell>` 实现共享可变)。
/// 对应 Java: `QScope` 对象引用的 Rust 共享所有权适配。
pub type ScopeRef = Rc<RefCell<QScope>>;

/// 作用域符号表。对应 Java: `QScope` 的 `Map<String, Value> symbolTable`。
/// Symbol table of a scope (Java `Map<String, Value> symbolTable`).
pub type SymbolTable = HashMap<String, Rc<RefCell<dyn LeftValue>>>;

/// 作用域链上的一个节点。对应 Java: com.alibaba.qlexpress4.runtime.scope.QScope
/// (Java 为接口体系,操作数栈 `FixedSizeStack` 在作用域与其 `newScope()` 子作用域间共享;
/// Rust 以 `Rc<RefCell<FixedSizeStack>>` 复现该共享语义)
/// One node of the scope chain.
pub struct QScope {
    parent: Option<ScopeRef>,
    /// Operand stack; shared with `new_scope` children (Java `reuseStack`).
    stack: Option<Rc<RefCell<FixedSizeStack>>>,
    kind: QScopeKind,
}

impl QScope {
    /// 创建根(全局)作用域节点。对应 Java `QvmGlobalScope`；Java 全局作用域
    /// 不拥有操作数栈，实际执行由其上的 `QvmBlockScope` 承担。
    pub fn global(global: QvmGlobalScope) -> ScopeRef {
        Rc::new(RefCell::new(QScope {
            parent: None,
            stack: None,
            kind: QScopeKind::Global(global),
        }))
    }

    /// 创建拥有独立操作数栈的子块作用域。
    /// 参数：`parent`、`symbol_table`、`max_stack_size`；返回：`ScopeRef`。
    /// 对应或承接 Java 源文件：`com/alibaba/qlexpress4/runtime/scope/QScope.java`，方法 `blockFreshStack`；Rust 侧按所有权与 `Result` 语义适配。
    /// Java `new QvmBlockScope(parent, symbolTable, maxStackSize, ...)`:
    /// child scope with a **fresh** operand stack (used by lambda
    /// invocation and for/while scopes).
    /// 对应 Java: com.alibaba.qlexpress4.runtime.scope.QScope#blockFreshStack。
    pub fn block_fresh_stack(
        parent: &ScopeRef,
        symbol_table: SymbolTable,
        max_stack_size: usize,
    ) -> ScopeRef {
        Rc::new(RefCell::new(QScope {
            parent: Some(Rc::clone(parent)),
            stack: Some(Rc::new(RefCell::new(FixedSizeStack::new(max_stack_size)))),
            kind: QScopeKind::Block(QvmBlockScope::new(symbol_table)),
        }))
    }

    /// 创建 new scope 实例。
    /// 参数：`this`；返回：`ScopeRef`。
    /// 对应或承接 Java 源文件：`com/alibaba/qlexpress4/runtime/scope/QScope.java`，方法 `newScope`；Rust 侧按所有权与 `Result` 语义适配。
    /// Java `QvmBlockScope.newScope()`: child scope **reusing** the parent
    /// operand stack.
    /// 对应 Java: com.alibaba.qlexpress4.runtime.scope.QScope#newScope。
    pub fn new_scope(this: &ScopeRef) -> ScopeRef {
        let stack = this
            .borrow()
            .stack
            .as_ref()
            .map(Rc::clone)
            .expect("QvmGlobalScope.newScope is unsupported");
        Rc::new(RefCell::new(QScope {
            parent: Some(Rc::clone(this)),
            stack: Some(stack),
            kind: QScopeKind::Block(QvmBlockScope::new(HashMap::new())),
        }))
    }

    /// 返回当前作用域的可选父作用域。
    /// 参数：`this`；返回：`Option<ScopeRef>`。
    /// 对应或承接 Java 源文件：`com/alibaba/qlexpress4/runtime/scope/QScope.java`，方法 `parent`；Rust 侧按所有权与 `Result` 语义适配。
    /// Java `getParent()`.
    /// 对应 Java: com.alibaba.qlexpress4.runtime.scope.QScope#parent。
    pub fn parent(this: &ScopeRef) -> Option<ScopeRef> {
        this.borrow().parent.as_ref().map(Rc::clone)
    }

    /// Java `getSymbol`: local symbol table first, then the parent chain;
    /// the global scope creates the variable when absent.
    ///
    /// 返回 `Result`(Stage 5a 接线改动):全局作用域的外部变量查询走
    /// `ExpressContext`,其动态求值(如 `DynamicVariableContext`)可能失败,
    /// 与 Java 中 `ExpressContext.get` 抛运行期异常上抛一致。
    /// 对应 Java: com.alibaba.qlexpress4.runtime.scope.QScope#getSymbol。
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

    /// 查询 symbol value。
    /// 参数：`this`、`var_name`；返回：`Result<Option<DataValue>, QLException>`。
    /// 对应或承接 Java 源文件：`com/alibaba/qlexpress4/runtime/Value.java`，方法 `getSymbolValue`；Rust 侧按所有权与 `Result` 语义适配。
    /// Java default `getSymbolValue`: inner data, `None` when absent
    /// (Java `null`).
    /// 对应 Java: com.alibaba.qlexpress4.runtime.scope.QScope#getSymbolValue。
    pub fn get_symbol_value(
        this: &ScopeRef,
        var_name: &str,
    ) -> Result<Option<DataValue>, QLException> {
        Ok(Self::get_symbol(this, var_name)?.map(|symbol| symbol.borrow().get()))
    }

    /// 添加或注册 local symbol。
    /// 参数：`this`、`var_name`、`var_clz`、`value`；返回：无。
    /// 对应或承接 Java 源文件：`com/alibaba/qlexpress4/runtime/scope/QScope.java`，方法 `defineLocalSymbol`；Rust 侧按所有权与 `Result` 语义适配。
    /// Java `QvmBlockScope.defineLocalSymbol`.
    /// 对应 Java: com.alibaba.qlexpress4.runtime.scope.QScope#defineLocalSymbol。
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

    /// 添加或注册 function。
    /// 参数：`this`、`function_name`、`function`；返回：无。
    /// 对应或承接 Java 源文件：`com/alibaba/qlexpress4/runtime/scope/QScope.java`，方法 `defineFunction`；Rust 侧按所有权与 `Result` 语义适配。
    /// Java `QvmBlockScope.defineFunction`.
    /// 对应 Java: com.alibaba.qlexpress4.runtime.scope.QScope#defineFunction。
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

    /// 查询 function。
    /// 参数：`this`、`function_name`；返回：`Option<Rc<dyn CustomFunction>>`。
    /// 对应或承接 Java 源文件：`com/alibaba/qlexpress4/annotation/QLFunction.java`，方法 `getFunction`；Rust 侧按所有权与 `Result` 语义适配。
    /// Java `getFunction`: local table first, then the parent chain.
    /// 对应 Java: com.alibaba.qlexpress4.runtime.scope.QScope#getFunction。
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

    /// 汇总当前作用域链可见的函数定义。
    /// 参数：`this`；返回：`HashMap<String, Rc<dyn CustomFunction>>`。
    /// 对应或承接 Java 源文件：`com/alibaba/qlexpress4/annotation/QLFunction.java`，方法 `functionTable`；Rust 侧按所有权与 `Result` 语义适配。
    /// Java `getFunctionTable`: the current scope's own table (not merged
    /// with parents).
    /// 对应 Java: com.alibaba.qlexpress4.runtime.scope.QScope#functionTable。
    pub fn function_table(this: &ScopeRef) -> HashMap<String, Rc<dyn CustomFunction>> {
        let borrowed = this.borrow();
        match &borrowed.kind {
            QScopeKind::Global(global) => global.function_table().clone(),
            QScopeKind::Block(block) => block.function_table().clone(),
        }
    }

    /// 返回当前内部栈的只读视图。
    /// 参数：`this`；返回：`Rc<RefCell<FixedSizeStack>>`。
    /// 对应或承接 Java 源文件：`com/alibaba/qlexpress4/runtime/scope/QScope.java`，方法 `stack`；Rust 侧按所有权与 `Result` 语义适配。
    /// The shared operand stack handle.
    /// 对应 Java: com.alibaba.qlexpress4.runtime.scope.QScope#stack。
    pub fn stack(this: &ScopeRef) -> Rc<RefCell<FixedSizeStack>> {
        this.borrow()
            .stack
            .as_ref()
            .map(Rc::clone)
            .expect("QvmGlobalScope operand stack operation is unsupported")
    }

    /// 将一个元素压入当前栈。
    /// 参数：`this`、`value`；返回：无。
    /// 对应或承接 Java 源文件：`com/alibaba/qlexpress4/runtime/scope/QScope.java`，方法 `push`；Rust 侧按所有权与 `Result` 语义适配。
    /// Java `push(Value)`.
    /// 对应 Java: com.alibaba.qlexpress4.runtime.scope.QScope#push。
    pub fn push(this: &ScopeRef, value: QValue) {
        Self::stack(this).borrow_mut().push(value);
    }

    /// 弹出并返回当前栈顶元素。
    /// 参数：`this`；返回：`QValue`。
    /// 对应或承接 Java 源文件：`com/alibaba/qlexpress4/runtime/scope/QScope.java`，方法 `pop`；Rust 侧按所有权与 `Result` 语义适配。
    /// Java `pop()`: top element. Panics on empty stack, like Java's
    /// `FixedSizeStack` array access.
    /// 对应 Java: com.alibaba.qlexpress4.runtime.scope.QScope#pop。
    pub fn pop(this: &ScopeRef) -> QValue {
        Self::stack(this).borrow_mut().pop()
    }

    /// 移除或清理 n。
    /// 参数：`this`、`number`；返回：`Parameters`。
    /// 对应或承接 Java 源文件：`com/alibaba/qlexpress4/runtime/scope/QScope.java`，方法 `popN`；Rust 侧按所有权与 `Result` 语义适配。
    /// Java `pop(int number)`: the top `number` elements in stack order
    /// (deepest first).
    /// 对应 Java: com.alibaba.qlexpress4.runtime.scope.QScope#popN。
    pub fn pop_n(this: &ScopeRef, number: usize) -> Parameters {
        Self::stack(this).borrow_mut().pop_n(number)
    }

    /// 读取但不移除当前栈顶元素。
    /// 参数：`this`；返回：`QValue`。
    /// 对应或承接 Java 源文件：`com/alibaba/qlexpress4/runtime/scope/QScope.java`，方法 `peek`；Rust 侧按所有权与 `Result` 语义适配。
    /// Java `peek()`: top element without popping.
    /// 对应 Java: com.alibaba.qlexpress4.runtime.scope.QScope#peek。
    pub fn peek(this: &ScopeRef) -> QValue {
        Self::stack(this).borrow().peak()
    }

    /// 判断当前操作数栈是否为空。
    /// 参数：`this`；返回：`bool`。
    /// 对应或承接 Java 源文件：`com/alibaba/qlexpress4/runtime/scope/QScope.java`，方法 `stackIsEmpty`；Rust 侧按所有权与 `Result` 语义适配。
    /// Whether the operand stack is empty.
    /// 对应 Java: com.alibaba.qlexpress4.runtime.scope.QScope#stackIsEmpty。
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

    fn stack_scope() -> ScopeRef {
        QScope::block_fresh_stack(&root(), HashMap::new(), 8)
    }

    #[test]
    fn symbol_defined_in_child_is_invisible_in_parent() {
        let parent = stack_scope();
        let child = QScope::new_scope(&parent);
        QScope::define_local_symbol(&child, "x", None, DataValue::Int(1));
        assert_eq!(
            QScope::get_symbol_value(&child, "x").unwrap(),
            Some(DataValue::Int(1))
        );
        // Parent chain reaches the global scope, which auto-creates an
        // independent slot (Java behavior).
        assert_eq!(
            QScope::get_symbol_value(&parent, "x").unwrap(),
            Some(DataValue::Null)
        );
    }

    #[test]
    fn new_scope_shares_operand_stack_like_java() {
        let parent = stack_scope();
        let child = QScope::new_scope(&parent);
        QScope::push(&parent, DataValue::Int(7).into());
        assert_eq!(QScope::peek(&child).get(), DataValue::Int(7));
        assert_eq!(QScope::pop(&child).get(), DataValue::Int(7));
        assert!(QScope::stack_is_empty(&parent));
    }

    #[test]
    fn fresh_stack_child_does_not_share() {
        let parent = stack_scope();
        let child = QScope::block_fresh_stack(&parent, HashMap::new(), 1);
        QScope::push(&parent, DataValue::Int(1).into());
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
        let scope = stack_scope();
        QScope::push(&scope, DataValue::Int(1).into());
        QScope::push(&scope, DataValue::Int(2).into());
        QScope::push(&scope, DataValue::Int(3).into());
        let params = QScope::pop_n(&scope, 2);
        assert_eq!(params.get_value(0), DataValue::Int(2));
        assert_eq!(params.get_value(1), DataValue::Int(3));
        assert_eq!(QScope::peek(&scope).get(), DataValue::Int(1));
    }
}
