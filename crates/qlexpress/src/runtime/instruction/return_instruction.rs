//! 返回指令,对应 Java `com.alibaba.qlexpress4.runtime.instruction.ReturnInstruction`。
//! 职责:以 return/break/continue 结束当前执行。
//! 本文件由 `flow.rs` 拆分而来(SPEC §5.5 一类一文件),仅移动代码与补充中文注释,行为完全一致。

use std::rc::Rc;
use crate::exception::error_reporter::ErrorReporter;
use crate::exception::QLException;
use crate::ql_options::QLOptions;
use crate::runtime::q_result::QResult;
use crate::runtime::instruction::{with_trace, QLInstruction};
use crate::runtime::qcontext::QContext;
use crate::runtime::value::DataValue;
use crate::utils::println_utils::PrintlnUtils;

/// 返回结果类型。对应 Java: `QResult.ResultType` 中可供 ReturnInstruction 使用的取值(Java 侧为 QResult 内部枚举,此处独立定义)。
/// Java `QResult.ResultType` values usable by `ReturnInstruction`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ReturnResultType {
    /// `QResult.ResultType.RETURN`
    Return,
    /// `QResult.ResultType.BREAK`
    Break,
    /// `QResult.ResultType.CONTINUE`
    Continue,
}

impl ReturnResultType {
    fn build(self, value: DataValue) -> QResult {
        match self {
            ReturnResultType::Return => QResult::Return(value),
            ReturnResultType::Break => QResult::Break,
            ReturnResultType::Continue => QResult::Continue(value),
        }
    }
}

/// 返回指令。对应 Java: com.alibaba.qlexpress4.runtime.instruction.ReturnInstruction(职责:以 return/break/continue 结束当前执行)
/// Operation: return top element and exit lambda
/// Input: 1
/// Output: 0
///
/// Mirrors Java `ReturnInstruction`.
pub struct ReturnInstruction {
    error_reporter: Rc<dyn ErrorReporter>,
    result_type: ReturnResultType,
    trace_key: Option<i32>,
}

impl ReturnInstruction {
    /// 构造指令,对应 Java 构造器 `ReturnInstruction`。
    pub fn new(
        error_reporter: Rc<dyn ErrorReporter>,
        result_type: ReturnResultType,
        trace_key: Option<i32>,
    ) -> Self {
        ReturnInstruction {
            error_reporter,
            result_type,
            trace_key,
        }
    }

    /// 对应 Java 方法 `resultType`。
    pub fn result_type(&self) -> ReturnResultType {
        self.result_type
    }

    /// 对应 Java 方法 `traceKey`。
    pub fn trace_key(&self) -> Option<i32> {
        self.trace_key
    }
}

impl QLInstruction for ReturnInstruction {
    /// 向下转型支持(供 api/parsecache Exporter 的 Java `instanceof` 分派)。
    fn as_any(&self) -> Option<&dyn std::any::Any> {
        Some(self)
    }

    fn execute(
        &self,
        q_context: &mut dyn QContext,
        _ql_options: &QLOptions,
    ) -> Result<QResult, QLException> {
        let return_value = q_context.pop();
        // Java traces only when traceKey != null (with_trace handles it).
        with_trace(q_context, self.trace_key, |trace| {
            trace.value_evaluated(return_value.get());
        });
        Ok(self.result_type.build(return_value.get()))
    }

    fn stack_input(&self) -> i32 {
        1
    }

    fn stack_output(&self) -> i32 {
        0
    }

    fn println(&self, index: usize, depth: usize, debug: &mut dyn FnMut(String)) {
        PrintlnUtils::println_by_cur_depth(depth as i32, &format!("{index}: Return"), debug);
    }

    fn is_terminal(&self) -> bool {
        true
    }

    fn error_reporter(&self) -> &Rc<dyn ErrorReporter> {
        &self.error_reporter
    }
}

