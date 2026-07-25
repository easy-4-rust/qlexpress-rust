//! trace 窥看指令,对应 Java `com.alibaba.qlexpress4.runtime.instruction.TracePeekInstruction`。
//! 职责:记录当前栈顶值到 trace(不弹栈)。
//! 本文件由 `trace.rs` 拆分而来(SPEC §5.5 一类一文件),仅移动代码与补充中文注释,行为完全一致。

use std::rc::Rc;
use crate::exception::error_reporter::ErrorReporter;
use crate::exception::QLException;
use crate::ql_options::QLOptions;
use crate::runtime::q_result::QResult;
use crate::runtime::instruction::{with_trace, QLInstruction};
use crate::runtime::qcontext::QContext;
use crate::utils::println_utils::PrintlnUtils;

/// trace 窥看指令。对应 Java: com.alibaba.qlexpress4.runtime.instruction.TracePeekInstruction(职责:记录当前栈顶值到 trace(不弹栈))
/// Operation: no op, only for tracing peek value of stack
/// Input: 0
/// Output: 0
///
/// Mirrors Java `TracePeekInstruction`.
pub struct TracePeekInstruction {
    error_reporter: Rc<dyn ErrorReporter>,
    trace_key: Option<i32>,
}

impl TracePeekInstruction {
    /// 构造指令,对应 Java 构造器 `TracePeekInstruction`。
    pub fn new(error_reporter: Rc<dyn ErrorReporter>, trace_key: Option<i32>) -> Self {
        TracePeekInstruction {
            error_reporter,
            trace_key,
        }
    }

    /// 对应 Java 方法 `traceKey`。
    pub fn trace_key(&self) -> Option<i32> {
        self.trace_key
    }
}

impl QLInstruction for TracePeekInstruction {
    /// 向下转型支持(供 api/parsecache Exporter 的 Java `instanceof` 分派)。
    fn as_any(&self) -> Option<&dyn std::any::Any> {
        Some(self)
    }

    fn execute(
        &self,
        q_context: &mut dyn QContext,
        _ql_options: &QLOptions,
    ) -> Result<QResult, QLException> {
        // Java peeks only when the trace point exists.
        if q_context
            .traces()
            .get_expression_trace_by_key(self.trace_key)
            .is_some()
        {
            let peeked = q_context.peek().get();
            with_trace(q_context, self.trace_key, |trace| {
                trace.value_evaluated(peeked);
            });
        }
        Ok(QResult::NEXT_INSTRUCTION)
    }

    fn stack_input(&self) -> i32 {
        0
    }

    fn stack_output(&self) -> i32 {
        0
    }

    fn println(&self, index: usize, depth: usize, debug: &mut dyn FnMut(String)) {
        PrintlnUtils::println_by_cur_depth(
            depth as i32,
            &format!(
                "{}: TracePeek {}",
                index,
                self.trace_key
                    .map(|k| k.to_string())
                    .unwrap_or_else(|| "null".to_string())
            ),
            debug,
        );
    }

    fn error_reporter(&self) -> &Rc<dyn ErrorReporter> {
        &self.error_reporter
    }
}

