//! 作用域链节点,对应 Java `com.alibaba.qlexpress4.runtime.scope.QScope`。
//! 职责:符号表/函数表查找、操作数栈存取、`newScope` 子作用域。
//! 本文件由 `scope/mod.rs` 拆分而来(SPEC §5.5 一类一文件),仅移动代码、
//! 对齐命名(`Scope` -> `QScope`)与补充中文注释,行为完全一致。

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

pub use super::q_scope_kind::QScopeKind;
use crate::exception::QLException;
use crate::runtime::class_ref::ClassRef;
use crate::runtime::data::AssignableDataValue;
use crate::runtime::fixed_size_stack::FixedSizeStack;
use crate::runtime::function::CustomFunction;
use crate::runtime::left_value::LeftValue;
use crate::runtime::native_registry::NativeRegistry;
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

/// 当前作用域自身的函数表。
/// 对应 Java: `QScope` 的 `Map<String, CustomFunction> functionTable`。
pub type FunctionTable = HashMap<String, Rc<dyn CustomFunction>>;

/// Java `Map<String, CustomFunction>` 的共享可变引用。
///
/// `QScope#getFunctionTable()` 返回实际函数表而非副本；Rust 使用该句柄保留
/// 宿主函数通过运行时上下文动态登记函数的写穿语义。
/// 对应 Java: `com.alibaba.qlexpress4.runtime.scope.QScope#getFunctionTable()`。
pub type SharedFunctionTable = Rc<RefCell<FunctionTable>>;

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

    /// 判断名字是否已经在当前作用域链中声明，且不会触发全局外部变量的懒创建。
    ///
    /// 对应 Java：函数调用区分“未知函数名”与“已声明但值为 null 的 Lambda
    /// 变量”时的符号表查询；后者必须抛 `NULL_CALL`，不能误报
    /// `FUNCTION_NOT_FOUND`。
    pub fn contains_symbol(this: &ScopeRef, var_name: &str) -> bool {
        let (contains, parent) = {
            let borrowed = this.borrow();
            let contains = match &borrowed.kind {
                QScopeKind::Global(global) => global.has_declared_symbol(var_name),
                QScopeKind::Block(block) => block.symbol_table().contains_key(var_name),
            };
            (contains, borrowed.parent.as_ref().map(Rc::clone))
        };
        contains || parent.is_some_and(|parent| Self::contains_symbol(&parent, var_name))
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
        var_clz: Option<ClassRef>,
        value: DataValue,
        type_registry: Rc<NativeRegistry>,
    ) {
        let mut borrowed = this.borrow_mut();
        match &mut borrowed.kind {
            QScopeKind::Global(global) => global.define_local_symbol(var_name),
            QScopeKind::Block(block) => {
                let slot: Rc<RefCell<dyn LeftValue>> = match var_clz {
                    Some(clz) => Rc::new(RefCell::new(AssignableDataValue::with_class(
                        var_name,
                        value,
                        clz,
                        type_registry,
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
                    .function_table()
                    .borrow_mut()
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
                QScopeKind::Block(block) => {
                    let function_table = block.function_table().borrow();
                    function_table.get(function_name).cloned()
                }
            };
            (local, borrowed.parent.as_ref().map(Rc::clone))
        };
        match (local, parent) {
            (Some(function), _) => Some(function),
            (None, Some(parent)) => Self::get_function(&parent, function_name),
            (None, None) => None,
        }
    }

    /// 返回当前作用域自身的函数表共享句柄。
    /// 参数：`this`；返回：[`SharedFunctionTable`]。
    /// 对应或承接 Java 源文件：`com/alibaba/qlexpress4/annotation/QLFunction.java`，方法 `functionTable`；Rust 侧按所有权与 `Result` 语义适配。
    /// Java `getFunctionTable`: the current scope's own table (not merged
    /// with parents), returned by reference rather than copied.
    /// 对应 Java: com.alibaba.qlexpress4.runtime.scope.QScope#functionTable。
    pub fn function_table(this: &ScopeRef) -> SharedFunctionTable {
        let borrowed = this.borrow();
        match &borrowed.kind {
            QScopeKind::Global(global) => global.function_table(),
            QScopeKind::Block(block) => Rc::clone(block.function_table()),
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
    use std::panic::{catch_unwind, AssertUnwindSafe};

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
        QScope::define_local_symbol(
            &child,
            "x",
            None,
            DataValue::Int(1),
            Rc::new(NativeRegistry::new()),
        );
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
        assert_eq!(
            QScope::get_symbol_value(&global, "a").unwrap(),
            Some(DataValue::Long(5))
        );
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

    /// `SOURCE_PARITY`：Java `QvmBlockScope#defineFunction/getFunction/
    /// getParent/newScope` 的本地优先、父级回退和父引用语义。
    #[test]
    fn block_scope_functions_and_parent_chain_match_java() {
        let parent = stack_scope();
        let child = QScope::new_scope(&parent);
        assert!(Rc::ptr_eq(
            &QScope::parent(&child).expect("block parent"),
            &parent
        ));

        let parent_function: Rc<dyn CustomFunction> = Rc::new(
            |_context: &mut dyn crate::runtime::qcontext::QContext, _parameters: &Parameters| {
                Ok(DataValue::Int(1))
            },
        );
        QScope::define_function(&parent, "f", Rc::clone(&parent_function));
        assert!(Rc::ptr_eq(
            &QScope::get_function(&child, "f").expect("parent function"),
            &parent_function
        ));

        let child_function: Rc<dyn CustomFunction> = Rc::new(
            |_context: &mut dyn crate::runtime::qcontext::QContext, _parameters: &Parameters| {
                Ok(DataValue::Int(2))
            },
        );
        QScope::define_function(&child, "f", Rc::clone(&child_function));
        assert!(Rc::ptr_eq(
            &QScope::get_function(&child, "f").expect("child function"),
            &child_function
        ));
        assert!(Rc::ptr_eq(
            &QScope::get_function(&parent, "f").expect("parent unchanged"),
            &parent_function
        ));

        // Java `getFunctionTable()` 返回当前块的实际 Map，不是副本。
        let child_table = QScope::function_table(&child);
        let table_function: Rc<dyn CustomFunction> = Rc::new(
            |_context: &mut dyn crate::runtime::qcontext::QContext, _parameters: &Parameters| {
                Ok(DataValue::Int(3))
            },
        );
        child_table
            .borrow_mut()
            .insert("tableOnly".to_string(), Rc::clone(&table_function));
        assert!(Rc::ptr_eq(
            &QScope::get_function(&child, "tableOnly").expect("live function table"),
            &table_function
        ));
        assert!(QScope::get_function(&parent, "tableOnly").is_none());
    }

    /// `SOURCE_PARITY`：Java `QvmGlobalScope` 不支持操作数栈与
    /// `newScope`；Rust 用 panic 表达 UnsupportedOperationException，
    /// `parent` 用 None 表达不存在父作用域。
    #[test]
    fn global_scope_rejects_block_only_operations() {
        let global = root();
        assert!(QScope::parent(&global).is_none());
        assert!(catch_unwind(AssertUnwindSafe(|| {
            QScope::push(&global, DataValue::Int(1).into());
        }))
        .is_err());
        assert!(catch_unwind(AssertUnwindSafe(|| QScope::pop_n(&global, 1))).is_err());
        assert!(catch_unwind(AssertUnwindSafe(|| QScope::pop(&global))).is_err());
        assert!(catch_unwind(AssertUnwindSafe(|| QScope::peek(&global))).is_err());
        assert!(catch_unwind(AssertUnwindSafe(|| QScope::new_scope(&global))).is_err());
        assert!(catch_unwind(AssertUnwindSafe(|| {
            QScope::define_local_symbol(
                &global,
                "x",
                None,
                DataValue::Null,
                Rc::new(NativeRegistry::new()),
            );
        }))
        .is_err());
        let unsupported_function: Rc<dyn CustomFunction> = Rc::new(
            |_context: &mut dyn crate::runtime::qcontext::QContext, _parameters: &Parameters| {
                Ok(DataValue::Null)
            },
        );
        assert!(catch_unwind(AssertUnwindSafe(|| {
            QScope::define_function(&global, "f", unsupported_function);
        }))
        .is_err());
    }

    /// `SOURCE_PARITY`：Java `QvmGlobalScope#getFunctionTable` 返回构造时传入
    /// 的外部函数 Map，同一 Map 的后续修改必须立即影响 `getFunction`。
    #[test]
    fn global_scope_preserves_live_external_function_table() {
        let global = root();
        let table = QScope::function_table(&global);
        let function: Rc<dyn CustomFunction> = Rc::new(
            |_context: &mut dyn crate::runtime::qcontext::QContext, _parameters: &Parameters| {
                Ok(DataValue::Int(7))
            },
        );
        table
            .borrow_mut()
            .insert("late".to_string(), Rc::clone(&function));
        assert!(Rc::ptr_eq(
            &QScope::get_function(&global, "late").expect("shared global function"),
            &function
        ));
    }
}
