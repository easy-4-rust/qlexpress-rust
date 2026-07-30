//! Execution context passed to every instruction, mirroring Java
//! `com.alibaba.qlexpress4.runtime.QContext` (= `QScope` + `QRuntime`).

use std::cell::{Ref, RefCell};
use std::rc::Rc;
use std::time::Instant;

use crate::exception::QLException;
use crate::ql_options::Attachments;
use crate::runtime::class_ref::ClassRef;
use crate::runtime::function::CustomFunction;
use crate::runtime::left_value::LeftValue;
use crate::runtime::member::NativeRegistry;
use crate::runtime::parameters::Parameters;
use crate::runtime::qvm_runtime::QvmRuntime;
use crate::runtime::scope::{ScopeRef, SharedFunctionTable};
use crate::runtime::trace::QTraces;
use crate::runtime::value::{DataValue, QValue};
use crate::security::CancellationToken;

/// `QContext` 接口的 Rust 实现，保留对应对象的领域职责与公开契约。
/// 对应或承接 Java 源文件：`com/alibaba/qlexpress4/runtime/QContext.java`；具体对象路径见 `docs/对象级对照表.md`。
/// Full execution context, mirroring Java `QContext`.
///
/// Method set = Java `QRuntime` (time/attachments/registry/traces) + Java
/// `QScope` (symbols, functions, operand stack, scope chain) + Java
/// `QContext` (`getCurrentScope`/`closeScope`).
/// 对应 Java: com.alibaba.qlexpress4.runtime.QContext。
pub trait QContext {
    // ---- Java QRuntime ----

    /// 处理 script start time stamp 对应的接口职责。
    /// 无显式参数；返回：`i64`。
    /// 对应或承接 Java 源文件：`com/alibaba/qlexpress4/runtime/QContext.java`，方法 `scriptStartTimeStamp`。
    /// Java `scriptStartTimeStamp()`.
    fn script_start_time_stamp(&self) -> i64;

    /// 处理 attachment 对应的接口职责。
    /// 无显式参数；返回附件 Map 的共享借用。
    /// 对应或承接 Java 源文件：`com/alibaba/qlexpress4/runtime/QContext.java`，方法 `attachment`。
    /// Java `attachment()`.
    fn attachment(&self) -> Ref<'_, Attachments>;

    /// 处理 registry 对应的接口职责。
    /// 无显式参数；返回：`&Rc<NativeRegistry>`。
    /// 对应或承接 Java 源文件：`com/alibaba/qlexpress4/runtime/QContext.java`，方法 `registry`。
    /// Java `getReflectLoader()` (SPEC §4: explicit native registry).
    fn registry(&self) -> &Rc<NativeRegistry>;

    /// 处理 traces 对应的接口职责。
    /// 无显式参数；返回：`&QTraces`。
    /// 对应或承接 Java 源文件：`com/alibaba/qlexpress4/runtime/QContext.java`，方法 `traces`。
    /// Java `getTraces()`.
    fn traces(&self) -> &QTraces;

    /// 处理 q runtime 对应的接口职责。
    /// 无显式参数；返回：`&Rc<QvmRuntime>`。
    /// 对应或承接 Java 源文件：`com/alibaba/qlexpress4/runtime/QContext.java`，方法 `qRuntime`。
    /// The shared root runtime (used to build lambda-captured contexts,
    /// Java `new DelegateQContext(qContext, ...)`).
    fn q_runtime(&self) -> &Rc<QvmRuntime>;

    /// 返回安全执行的绝对截止时间；普通兼容执行返回 `None`。
    ///
    /// 宿主函数必须把该期限传给网络、数据库等下游调用。
    fn deadline(&self) -> Option<Instant> {
        self.q_runtime().deadline()
    }

    /// 返回外部协作式取消令牌；普通兼容执行返回 `None`。
    fn cancellation_token(&self) -> Option<&CancellationToken> {
        self.q_runtime().cancellation_token()
    }

    // ---- Java QScope ----

    /// Java `getSymbol`: assignable symbol by name; the global scope
    /// auto-creates it when absent.
    ///
    /// Stage 5a 接线改动:返回 `Result`——外部变量查询走 `ExpressContext`,
    /// 动态上下文求值失败与 Java 运行期异常上抛一致。
    fn get_symbol(
        &mut self,
        var_name: &str,
    ) -> Result<Option<Rc<RefCell<dyn LeftValue>>>, QLException>;

    /// 查询 symbol value。
    /// 参数：`var_name`；返回：`Result<Option<DataValue>, QLException>`。
    /// 对应或承接 Java 源文件：`com/alibaba/qlexpress4/runtime/QContext.java`，方法 `getSymbolValue`。
    /// Java default `getSymbolValue`.
    fn get_symbol_value(&mut self, var_name: &str) -> Result<Option<DataValue>, QLException> {
        Ok(self
            .get_symbol(var_name)?
            .map(|symbol| symbol.borrow().get()))
    }

    /// 添加或注册 local symbol。
    /// 参数：`var_name`、`var_clz`、`value`；返回：无。
    /// 对应或承接 Java 源文件：`com/alibaba/qlexpress4/runtime/QContext.java`，方法 `defineLocalSymbol`。
    /// Java `defineLocalSymbol`.
    fn define_local_symbol(
        &mut self,
        var_name: &str,
        var_clz: Option<ClassRef>,
        value: DataValue,
    );

    /// 添加或注册 function。
    /// 参数：`function_name`、`function`；返回：无。
    /// 对应或承接 Java 源文件：`com/alibaba/qlexpress4/runtime/QContext.java`，方法 `defineFunction`。
    /// Java `defineFunction`.
    fn define_function(&mut self, function_name: &str, function: Rc<dyn CustomFunction>);

    /// 查询 function。
    /// 参数：`function_name`；返回：`Option<Rc<dyn CustomFunction>>`。
    /// 对应或承接 Java 源文件：`com/alibaba/qlexpress4/runtime/QContext.java`，方法 `getFunction`。
    /// Java `getFunction`.
    fn get_function(&self, function_name: &str) -> Option<Rc<dyn CustomFunction>>;

    /// 处理 function table 对应的接口职责。
    /// 无显式参数；返回当前作用域函数表的共享句柄。
    /// 对应或承接 Java 源文件：`com/alibaba/qlexpress4/runtime/QContext.java`，方法 `functionTable`。
    /// Java `getFunctionTable` (current scope's own mutable table).
    fn function_table(&self) -> SharedFunctionTable;

    /// 处理 push 对应的接口职责。
    /// 参数：`value`；返回：无。
    /// 对应或承接 Java 源文件：`com/alibaba/qlexpress4/runtime/QContext.java`，方法 `push`。
    /// Java `push(Value)`.
    fn push(&mut self, value: QValue);

    /// 移除或关闭 n。
    /// 参数：`number`；返回：`Parameters`。
    /// 对应或承接 Java 源文件：`com/alibaba/qlexpress4/runtime/QContext.java`，方法 `popN`。
    /// Java `pop(int number)`.
    fn pop_n(&mut self, number: usize) -> Parameters;

    /// 处理 pop 对应的接口职责。
    /// 无显式参数；返回：`QValue`。
    /// 对应或承接 Java 源文件：`com/alibaba/qlexpress4/runtime/QContext.java`，方法 `pop`。
    /// Java `pop()`.
    fn pop(&mut self) -> QValue;

    /// 处理 peek 对应的接口职责。
    /// 无显式参数；返回：`QValue`。
    /// 对应或承接 Java 源文件：`com/alibaba/qlexpress4/runtime/QContext.java`，方法 `peek`。
    /// Java `peek()`.
    fn peek(&self) -> QValue;

    /// 处理 parent scope 对应的接口职责。
    /// 无显式参数；返回：`Option<ScopeRef>`。
    /// 对应或承接 Java 源文件：`com/alibaba/qlexpress4/runtime/QContext.java`，方法 `parentScope`。
    /// Java `getParent()`.
    fn parent_scope(&self) -> Option<ScopeRef>;

    /// 处理 new scope 对应的接口职责。
    /// 无显式参数；返回：`ScopeRef`。
    /// 对应或承接 Java 源文件：`com/alibaba/qlexpress4/runtime/QContext.java`，方法 `newScope`。
    /// Java `QScope.newScope()` + `DelegateQContext.newScope()`:
    /// opens a child scope (sharing the operand stack) and makes it current.
    fn new_scope(&mut self) -> ScopeRef;

    // ---- Java QContext ----

    /// 处理 current scope 对应的接口职责。
    /// 无显式参数；返回：`ScopeRef`。
    /// 对应或承接 Java 源文件：`com/alibaba/qlexpress4/runtime/QContext.java`，方法 `currentScope`。
    /// Java `getCurrentScope()`.
    fn current_scope(&self) -> ScopeRef;

    /// 移除或关闭 scope。
    /// 无显式参数；返回：无。
    /// 对应或承接 Java 源文件：`com/alibaba/qlexpress4/runtime/QContext.java`，方法 `closeScope`。
    /// Java `closeScope()`: the parent scope becomes current.
    fn close_scope(&mut self);

    /// 更新 current scope。
    /// 参数：`scope`；返回：无。
    /// 对应或承接 Java 源文件：`com/alibaba/qlexpress4/runtime/QContext.java`，方法 `setCurrentScope`。
    /// Replace the current scope (used when entering lambda/for scopes,
    /// mirroring Java's `new DelegateQContext(qContext, newScope)`).
    fn set_current_scope(&mut self, scope: ScopeRef);
}
