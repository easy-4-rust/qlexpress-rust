//! QVM 指令基 trait,对应 Java `com.alibaba.qlexpress4.runtime.instruction.QLInstruction`。
//! 职责:定义所有指令的统一契约(执行、栈输入/输出大小、调试打印、错误报告器)。
//! 本文件同时承载 Rust 侧的两个辅助定义(Java 无对应类):
//! - `Instruction` 类型别名:拥有所有权的指令对象(Java `QLInstruction[]` 元素);
//! - `with_trace` 辅助函数:Java 各指令中重复的 trace 记录片段的公共抽取。

use std::rc::Rc;

use crate::exception::error_reporter::ErrorReporter;
use crate::exception::QLException;
use crate::ql_options::QLOptions;
use crate::runtime::q_result::QResult;
use crate::runtime::qcontext::QContext;

/// 指令基契约。对应 Java: com.alibaba.qlexpress4.runtime.instruction.QLInstruction(指令统一接口)
///
/// Instruction Specification (Java convention, kept per instruction):
/// * Operation: What does it do?
/// * Input: How many stack element it consumes? and their means
/// * Output: How many stack element it push back? and their means
pub trait QLInstruction {
    /// 执行指令。对应 Java 方法 `execute(QContext, QLOptions)`;
    /// 错误通过指令自带的 [`ErrorReporter`] 报告(Java: 抛出 `QLRuntimeException`)。
    fn execute(
        &self,
        q_context: &mut dyn QContext,
        ql_options: &QLOptions,
    ) -> Result<QResult, QLException>;

    /// 栈输入大小。对应 Java 方法 `stackInput()`。
    fn stack_input(&self) -> i32;

    /// 栈输出大小。对应 Java 方法 `stackOutput()`。
    fn stack_output(&self) -> i32;

    /// 调试打印。对应 Java 方法 `println(int index, int depth, Consumer<String> debug)`。
    fn println(&self, index: usize, depth: usize, debug: &mut dyn FnMut(String));

    /// 获取错误报告器。对应 Java 方法 `getErrorReporter()`。
    fn error_reporter(&self) -> &Rc<dyn ErrorReporter>;

    /// 无条件相对跳转偏移(Java `JumpInstruction`),用于静态控制流/栈分析。
    /// 默认 `None`(非跳转指令)。
    fn static_jump(&self) -> Option<i32> {
        None
    }

    /// 条件相对跳转偏移(Java `JumpIfInstruction` / `JumpIfPopInstruction`);
    /// 执行时也可能顺序下落。默认 `None`。
    fn conditional_jump(&self) -> Option<i32> {
        None
    }

    /// 执行后是否绝不顺序下落(return/throw/break/continue)。默认 `false`。
    fn is_terminal(&self) -> bool {
        false
    }

    /// 向下转型支持(Java `instanceof` 链的 Rust 等价物),供
    /// `api/parsecache` 的 Exporter 按具体指令类型分派导出。
    /// 各具体指令实现返回 `Some(self)`;默认 `None`(表示不支持导出分派)。
    fn as_any(&self) -> Option<&dyn std::any::Any> {
        None
    }
}

/// 拥有所有权的指令对象(Java `QLInstruction[]` 元素)。
pub type Instruction = Box<dyn QLInstruction>;

/// Java 侧会共享指令对象(如编译后的宏体在每个宏调用点内联)。
/// Rust 侧通过 `Rc` 共享;此 blanket impl 让共享指令可以重新装箱为
/// [`Instruction`],且不改变 `execute(&self)` 语义。
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

    fn static_jump(&self) -> Option<i32> {
        (**self).static_jump()
    }

    fn conditional_jump(&self) -> Option<i32> {
        (**self).conditional_jump()
    }

    fn is_terminal(&self) -> bool {
        (**self).is_terminal()
    }

    fn as_any(&self) -> Option<&dyn std::any::Any> {
        (**self).as_any()
    }
}

/// 共享的 trace 记录辅助:Java
/// `ExpressionTrace t = traces.getExpressionTraceByKey(key); if (t != null) { ... }`。
/// 对应 Java: com.alibaba.qlexpress4.runtime.instruction.QLInstruction#withTrace。
pub(crate) fn with_trace(
    q_context: &dyn QContext,
    trace_key: Option<i32>,
    f: impl FnOnce(&mut crate::runtime::trace::ExpressionTrace),
) {
    q_context
        .traces()
        .with_expression_trace_by_key(trace_key, f);
}
