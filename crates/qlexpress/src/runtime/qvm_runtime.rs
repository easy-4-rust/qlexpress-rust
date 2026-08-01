//! 根运行时与 VM 取指-执行循环,对应 Java `com.alibaba.qlexpress4.runtime.QvmRuntime`
//! (上下文)与 `QLambdaInner.callInner`(指令循环)。
//! (`QRuntime` trait 已拆至 [`crate::runtime::q_runtime`]。)
//!
//! Root runtime and the VM fetch-execute loop, mirroring Java
//! `com.alibaba.qlexpress4.runtime.QvmRuntime` (context) and
//! `QLambdaInner.callInner` (the instruction loop).

use std::cell::{Ref, RefMut};
use std::rc::Rc;
use std::time::Instant;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::exception::QLException;
use crate::ql_options::{Attachments, QLOptions, SharedAttachments};
use crate::runtime::delegate_qcontext::DelegateQContext;
use crate::runtime::execution_budget::ExecutionBudget;
use crate::runtime::instruction::Instruction;
use crate::runtime::member::NativeRegistry;
use crate::runtime::q_result::QResult;
use crate::runtime::q_runtime::QRuntime;
use crate::runtime::qcontext::QContext;
use crate::runtime::qlambda_definition::QLambdaDefinition;
use crate::runtime::qvm_global_scope::QvmGlobalScope;
use crate::runtime::scope::QScope;
use crate::runtime::trace::QTraces;
use crate::security::{CancellationToken, CapabilityPolicy, ResourceLimits};

/// 返回 Unix 纪元起经过的当前毫秒数。
/// 无显式参数；返回：`i64`。
/// 对应或承接 Java 源文件：`com/alibaba/qlexpress4/runtime/QvmRuntime.java`，方法 `currentTimeMillis`；Rust 侧按所有权与 `Result` 语义适配。
/// Current time in milliseconds since the Unix epoch (Java
/// `System.currentTimeMillis()`).
/// 对应 Java: com.alibaba.qlexpress4.runtime.QvmRuntime#currentTimeMillis。
pub fn current_time_millis() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

/// 保存一次 QVM 执行共享的注册表、追踪状态、安全预算和能力策略。
/// 对应或承接 Java 源文件：`com/alibaba/qlexpress4/runtime/QvmRuntime.java`；具体对象路径见 `docs/对象级对照表.md`。
/// Root runtime with external variable and function, mirroring Java
/// `QvmRuntime`. Immutable after construction (trace points mutate through
/// interior mutability inside [`QTraces`]); shared as `Rc<QvmRuntime>` like
/// Java shares the instance.
/// 对应 Java: com.alibaba.qlexpress4.runtime.QvmRuntime。
pub struct QvmRuntime {
    traces: QTraces,
    attachments: SharedAttachments,
    registry: Rc<NativeRegistry>,
    start_time: i64,
    execution_budget: Option<ExecutionBudget>,
    capability_policy: Option<CapabilityPolicy>,
}

impl QvmRuntime {
    /// 创建对象实例。
    /// 参数：`traces`、`attachments`、`registry`、`start_time`；返回：`Self`。
    /// 对应或承接 Java 源文件：`com/alibaba/qlexpress4/runtime/QvmRuntime.java`，构造器 `<init>`；Rust 侧按所有权与 `Result` 语义适配。
    /// Java `new QvmRuntime(traces, attachments, reflectLoader, startTime)`.
    /// 对应 Java: com.alibaba.qlexpress4.runtime.QvmRuntime#new。
    pub fn new(
        traces: QTraces,
        attachments: SharedAttachments,
        registry: Rc<NativeRegistry>,
        start_time: i64,
    ) -> Self {
        QvmRuntime {
            traces,
            attachments,
            registry,
            start_time,
            execution_budget: None,
            capability_policy: None,
        }
    }

    /// 创建带有限资源预算和取消令牌的安全运行时。
    /// 对应 Java：无（Rust 安全增强；兼容运行时仍由 [`Self::new`] 创建）。
    pub fn new_sandboxed(
        traces: QTraces,
        attachments: SharedAttachments,
        registry: Rc<NativeRegistry>,
        start_time: i64,
        limits: ResourceLimits,
        cancellation_token: CancellationToken,
        capability_policy: CapabilityPolicy,
    ) -> Self {
        QvmRuntime {
            traces,
            attachments,
            registry,
            start_time,
            execution_budget: Some(ExecutionBudget::new(limits, cancellation_token)),
            capability_policy: Some(capability_policy),
        }
    }

    /// 判断安全执行是否允许调用指定运行时方法；兼容路径不附加限制。
    /// 对应 Java：`QLSecurityStrategy#check(Member)`（Rust 扩展为统一 capability 策略）。
    pub fn is_method_capability_allowed(&self, type_name: &str, method_name: &str) -> bool {
        self.capability_policy
            .as_ref()
            .is_none_or(|policy| policy.is_method_allowed(type_name, method_name))
    }

    /// 返回安全运行时预算；普通 Java 兼容执行为 `None`。
    /// 对应 Java：无（Rust 安全增强的运行时预算）。
    pub fn execution_budget(&self) -> Option<&ExecutionBudget> {
        self.execution_budget.as_ref()
    }

    /// 返回宿主调用应遵守的截止时间。
    /// 对应 Java：无（Rust 安全增强的宿主调用截止时间）。
    pub fn deadline(&self) -> Option<Instant> {
        self.execution_budget
            .as_ref()
            .map(ExecutionBudget::deadline)
    }

    /// 返回宿主调用可检查的取消令牌。
    /// 对应 Java：无（Rust 安全增强的协作式取消能力）。
    pub fn cancellation_token(&self) -> Option<&CancellationToken> {
        self.execution_budget
            .as_ref()
            .map(ExecutionBudget::cancellation_token)
    }

    /// 返回原生成员加载注册表。
    ///
    /// 对应 Java：`QvmRuntime#getReflectLoader()`。Rust 的
    /// [`crate::runtime::reflect_loader::ReflectLoader`] 门面在 runner
    /// 构造阶段完成配置，QVM 只共享其底层 [`NativeRegistry`]。
    ///
    /// # 返回值
    ///
    /// 返回本运行时使用的共享注册表。
    pub fn get_reflect_loader(&self) -> &Rc<NativeRegistry> {
        &self.registry
    }

    /// 构造测试场景使用的实例。
    /// 参数：`registry`；返回：`Self`。
    /// 对应或承接 Java 源文件：`com/alibaba/qlexpress4/runtime/QvmRuntime.java`，方法 `forTest`；Rust 侧按所有权与 `Result` 语义适配。
    /// Convenience: a runtime with empty traces/attachments and the default
    /// registry, started now.
    /// 对应 Java: com.alibaba.qlexpress4.runtime.QvmRuntime#forTest。
    pub fn for_test(registry: Rc<NativeRegistry>) -> Self {
        Self::new(
            QTraces::empty(),
            Rc::new(std::cell::RefCell::new(Attachments::default())),
            registry,
            current_time_millis(),
        )
    }

    /// 执行编译后的 Lambda 并返回 QL 结果。
    /// 参数：`global_scope`、`root_definition`、`ql_options`；返回：`Result<QResult, QLException>`。
    /// 对应或承接 Java 源文件：`com/alibaba/qlexpress4/Express4Runner.java`，方法 `execute`；Rust 侧按所有权与 `Result` 语义适配。
    /// Top-level script execution, mirroring Java `Express4Runner`:
    /// `rootLambdaDefinition.toLambda(new DelegateQContext(qvmRuntime,
    /// globalScope), qlOptions, true).call()`.
    /// 对应 Java: com.alibaba.qlexpress4.runtime.QvmRuntime#execute。
    pub fn execute(
        self: &Rc<Self>,
        global_scope: QvmGlobalScope,
        root_definition: Rc<dyn QLambdaDefinition>,
        ql_options: &QLOptions,
    ) -> Result<QResult, QLException> {
        let mut root_context = DelegateQContext::new(Rc::clone(self), QScope::global(global_scope));
        let root_lambda = root_definition.to_lambda(&mut root_context, ql_options, true);
        root_lambda.call(&[])
    }

    /// 执行 instructions。
    /// 参数：`instructions`、`ql_options`；返回：`Result<QResult, QLException>`。
    /// 对应或承接 Java 源文件：`com/alibaba/qlexpress4/runtime/QvmRuntime.java`，方法 `executeInstructions`；Rust 侧按所有权与 `Result` 语义适配。
    /// Execute an instruction sequence directly with a fresh global scope
    /// (test/support entry point).
    /// 对应 Java: com.alibaba.qlexpress4.runtime.QvmRuntime#executeInstructions。
    pub fn execute_instructions(
        self: &Rc<Self>,
        instructions: &[Instruction],
        ql_options: &QLOptions,
    ) -> Result<QResult, QLException> {
        let global_scope = QScope::global(QvmGlobalScope::empty());
        // 此入口仅供底层测试/支持代码直接执行指令。Java 正常入口会由
        // QLambdaInner 按编译结果创建定长栈；这里以所有正栈输出之和作为
        // 保守上界，确保同样使用 FixedSizeStack 而不绕过容量模型。
        let max_stack_size = instructions
            .iter()
            .map(|instruction| instruction.stack_output().max(0) as usize)
            .sum();
        let instruction_scope =
            QScope::block_fresh_stack(&global_scope, Default::default(), max_stack_size);
        let mut context = DelegateQContext::new(Rc::clone(self), instruction_scope);
        run_instructions(&mut context, instructions, ql_options)
    }
}

impl QRuntime for QvmRuntime {
    fn script_start_time_stamp(&self) -> i64 {
        self.start_time
    }

    fn attachment(&self) -> Ref<'_, Attachments> {
        self.attachments.borrow()
    }

    fn attachment_mut(&self) -> RefMut<'_, Attachments> {
        self.attachments.borrow_mut()
    }

    fn registry(&self) -> &Rc<NativeRegistry> {
        &self.registry
    }

    fn traces(&self) -> &QTraces {
        &self.traces
    }
}

/// 执行 instructions。
/// 参数：`q_context`、`instructions`、`ql_options`；返回：`Result<QResult, QLException>`。
/// 对应或承接 Java 源文件：`com/alibaba/qlexpress4/runtime/QLambdaInner.java`，方法 `runInstructions`；Rust 侧按所有权与 `Result` 语义适配。
/// The QVM fetch-execute loop, mirroring Java `QLambdaInner.callInner`:
/// execute each instruction; `JUMP` adds the (relative) offset to the
/// program counter, `RETURN`/`BREAK`/`CONTINUE` exit the loop, anything
/// else advances to the next instruction.
/// 对应 Java: com.alibaba.qlexpress4.runtime.QvmRuntime#runInstructions。
pub fn run_instructions(
    q_context: &mut dyn QContext,
    instructions: &[Instruction],
    ql_options: &QLOptions,
) -> Result<QResult, QLException> {
    let mut i: i64 = 0;
    while i >= 0 && (i as usize) < instructions.len() {
        if let Some(budget) = q_context.q_runtime().execution_budget() {
            budget.consume_fuel(1)?;
        }
        let q_result = instructions[i as usize].execute(q_context, ql_options)?;
        if instructions[i as usize].stack_output() > 0 {
            if let Some(budget) = q_context.q_runtime().execution_budget() {
                budget.validate_value(&q_context.peek().get())?;
            }
        }
        match q_result {
            QResult::Jump(offset) => {
                // Java `callInner`: `case JUMP: i += position; continue;` —
                // the `for` loop's `i++` still runs on `continue`, so the
                // effective target is `i + position + 1`. The compiler's
                // back-patch arithmetic (`size - jumpStart`) assumes this.
                i += offset as i64 + 1;
                continue;
            }
            QResult::Return(_) | QResult::Break | QResult::Continue(_) => return Ok(q_result),
            QResult::NextInstruction => {}
        }
        i += 1;
    }
    Ok(QResult::NEXT_INSTRUCTION)
}
