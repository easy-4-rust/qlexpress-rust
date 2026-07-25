//! QVM instructions, mirroring Java
//! `com.alibaba.qlexpress4.runtime.instruction` (all 42 classes).
//!
//! Every instruction keeps the Java javadoc *Specification* (Operation /
//! Input / Output) as its doc comment, reports through its own
//! [`ErrorReporter`], and reproduces the Java debug `println` output.
//!
//! Grouping (SPEC §2): one category per file.

use std::rc::Rc;

use crate::exception::error_reporter::ErrorReporter;
use crate::exception::QLException;
use crate::ql_options::QLOptions;
use crate::ql_result::QResult;
use crate::runtime::qcontext::QContext;

pub mod call;
pub mod cast;
pub mod const_inst;
pub mod field_method;
pub mod flow;
pub mod index;
pub mod new_instance;
pub mod scope;
pub mod string_join;
pub mod trace;
pub mod unary_binary;

pub use call::{CallFunctionInstruction, CallInstruction, MethodInvokeInstruction, SpreadMethodInvokeInstruction};
pub use cast::CastInstruction;
pub use const_inst::{CallConstInstruction, ConstInstruction};
pub use field_method::{GetFieldInstruction, GetMethodInstruction, SpreadGetFieldInstruction};
pub use flow::{
    BreakContinueInstruction, CheckTimeOutInstruction, ForEachInstruction, ForInstruction,
    JumpIfInstruction, JumpIfPopInstruction, JumpInstruction, PopInstruction, ReturnInstruction,
    ReturnResultType, ThrowInstruction, TryCatchInstruction, WhileInstruction,
};
pub use index::{IndexInstruction, SliceInstruction, SliceMode};
pub use new_instance::{
    MultiNewArrayInstruction, NewArrayInstruction, NewFilledInstanceInstruction,
    NewInstanceInstruction, NewListInstruction, NewMapInstruction,
};
pub use scope::{
    CloseScopeInstruction, DefineFunctionInstruction, DefineLocalInstruction, LoadInstruction,
    LoadLambdaInstruction, NewScopeInstruction,
};
pub use string_join::StringJoinInstruction;
pub use trace::{TraceEvaluatedInstruction, TracePeekInstruction};
pub use unary_binary::{OperatorInstruction, UnaryInstruction};

/// Base instruction contract, mirroring Java `QLInstruction`.
///
/// Instruction Specification (Java convention, kept per instruction):
/// * Operation: What does it do?
/// * Input: How many stack element it consumes? and their means
/// * Output: How many stack element it push back? and their means
pub trait QLInstruction {
    /// Java `execute(QContext, QLOptions)`; errors are reported through the
    /// instruction's [`ErrorReporter`] (Java: thrown `QLRuntimeException`).
    fn execute(
        &self,
        q_context: &mut dyn QContext,
        ql_options: &QLOptions,
    ) -> Result<QResult, QLException>;

    /// Java `stackInput()`: input size.
    fn stack_input(&self) -> i32;

    /// Java `stackOutput()`: output size.
    fn stack_output(&self) -> i32;

    /// Java `println(int index, int depth, Consumer<String> debug)`.
    fn println(&self, index: usize, depth: usize, debug: &mut dyn FnMut(String));

    /// Java `getErrorReporter()`.
    fn error_reporter(&self) -> &Rc<dyn ErrorReporter>;
}

/// An owned instruction (Java `QLInstruction[]` element).
pub type Instruction = Box<dyn QLInstruction>;

/// Java shares instruction objects (e.g. a compiled macro body is inlined
/// at every macro call site). The Rust port shares them through `Rc`; this
/// blanket impl lets a shared instruction be boxed back into an
/// [`Instruction`] without changing the `execute(&self)` semantics.
impl<T: QLInstruction + ?Sized> QLInstruction for Rc<T> {
    fn execute(
        &self,
        q_context: &mut dyn QContext,
        ql_options: &QLOptions,
    ) -> Result<QResult, QLException> {
        (**self).execute(q_context, ql_options)
    }

    fn stack_input(&self) -> i32 {
        (**self).stack_input()
    }

    fn stack_output(&self) -> i32 {
        (**self).stack_output()
    }

    fn println(&self, index: usize, depth: usize, debug: &mut dyn FnMut(String)) {
        (**self).println(index, depth, debug)
    }

    fn error_reporter(&self) -> &Rc<dyn ErrorReporter> {
        (**self).error_reporter()
    }
}

/// Shared trace-marking helper: Java
/// `ExpressionTrace t = traces.getExpressionTraceByKey(key); if (t != null) { ... }`.
pub(crate) fn with_trace(
    q_context: &dyn QContext,
    trace_key: Option<i32>,
    f: impl FnOnce(&mut crate::runtime::trace::ExpressionTrace),
) {
    if let Some(trace) = q_context.traces().get_expression_trace_by_key(trace_key) {
        f(&mut trace.borrow_mut());
    }
}
