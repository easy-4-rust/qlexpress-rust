//! 根运行时与 VM 取指-执行循环,对应 Java `com.alibaba.qlexpress4.runtime.QvmRuntime`
//! (上下文)与 `QLambdaInner.callInner`(指令循环)。
//! (`QRuntime` trait 已拆至 [`crate::runtime::q_runtime`]。)
//!
//! Root runtime and the VM fetch-execute loop, mirroring Java
//! `com.alibaba.qlexpress4.runtime.QvmRuntime` (context) and
//! `QLambdaInner.callInner` (the instruction loop).

use std::rc::Rc;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::exception::QLException;
use crate::ql_options::{Attachments, QLOptions};
use crate::runtime::q_result::QResult;
use crate::runtime::delegate_qcontext::DelegateQContext;
use crate::runtime::instruction::Instruction;
use crate::runtime::member::NativeRegistry;
use crate::runtime::q_runtime::QRuntime;
use crate::runtime::qcontext::QContext;
use crate::runtime::qlambda_definition::QLambdaDefinition;
use crate::runtime::qvm_global_scope::QvmGlobalScope;
use crate::runtime::scope::QScope;
use crate::runtime::trace::QTraces;

/// Current time in milliseconds since the Unix epoch (Java
/// `System.currentTimeMillis()`).
pub fn current_time_millis() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

/// Root runtime with external variable and function, mirroring Java
/// `QvmRuntime`. Immutable after construction (trace points mutate through
/// interior mutability inside [`QTraces`]); shared as `Rc<QvmRuntime>` like
/// Java shares the instance.
pub struct QvmRuntime {
    traces: QTraces,
    attachments: Attachments,
    registry: Rc<NativeRegistry>,
    start_time: i64,
}

impl QvmRuntime {
    /// Java `new QvmRuntime(traces, attachments, reflectLoader, startTime)`.
    pub fn new(
        traces: QTraces,
        attachments: Attachments,
        registry: Rc<NativeRegistry>,
        start_time: i64,
    ) -> Self {
        QvmRuntime {
            traces,
            attachments,
            registry,
            start_time,
        }
    }

    /// Convenience: a runtime with empty traces/attachments and the default
    /// registry, started now.
    pub fn for_test(registry: Rc<NativeRegistry>) -> Self {
        Self::new(
            QTraces::empty(),
            Attachments::default(),
            registry,
            current_time_millis(),
        )
    }

    /// Top-level script execution, mirroring Java `Express4Runner`:
    /// `rootLambdaDefinition.toLambda(new DelegateQContext(qvmRuntime,
    /// globalScope), qlOptions, true).call()`.
    pub fn execute(
        self: &Rc<Self>,
        global_scope: QvmGlobalScope,
        root_definition: Rc<dyn QLambdaDefinition>,
        ql_options: &QLOptions,
    ) -> Result<QResult, QLException> {
        let mut root_context =
            DelegateQContext::new(Rc::clone(self), QScope::global(global_scope));
        let root_lambda = root_definition.to_lambda(&mut root_context, ql_options, true);
        root_lambda.call(&[])
    }

    /// Execute an instruction sequence directly with a fresh global scope
    /// (test/support entry point).
    pub fn execute_instructions(
        self: &Rc<Self>,
        instructions: &[Instruction],
        ql_options: &QLOptions,
    ) -> Result<QResult, QLException> {
        let mut context = DelegateQContext::new(Rc::clone(self), QScope::global(QvmGlobalScope::empty()));
        run_instructions(&mut context, instructions, ql_options)
    }
}

impl QRuntime for QvmRuntime {
    fn script_start_time_stamp(&self) -> i64 {
        self.start_time
    }

    fn attachment(&self) -> &Attachments {
        &self.attachments
    }

    fn registry(&self) -> &Rc<NativeRegistry> {
        &self.registry
    }

    fn traces(&self) -> &QTraces {
        &self.traces
    }
}

/// The QVM fetch-execute loop, mirroring Java `QLambdaInner.callInner`:
/// execute each instruction; `JUMP` adds the (relative) offset to the
/// program counter, `RETURN`/`BREAK`/`CONTINUE` exit the loop, anything
/// else advances to the next instruction.
pub fn run_instructions(
    q_context: &mut dyn QContext,
    instructions: &[Instruction],
    ql_options: &QLOptions,
) -> Result<QResult, QLException> {
    let mut i: i64 = 0;
    while i >= 0 && (i as usize) < instructions.len() {
        let q_result = instructions[i as usize].execute(q_context, ql_options)?;
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
