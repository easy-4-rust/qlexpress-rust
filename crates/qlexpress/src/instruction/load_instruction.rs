//! 变量加载指令,对应 Java `com.alibaba.qlexpress4.runtime.instruction.LoadInstruction`。
//! 职责:按名字加载变量值并压栈。
//! 本文件由 `scope.rs` 拆分而来(SPEC §5.5 一类一文件),仅移动代码与补充中文注释,行为完全一致。

use crate::exception::QLException;
use crate::exception::error_reporter::ErrorReporter;
use crate::ql_options::QLOptions;
use crate::runtime::instruction::{QLInstruction, with_trace};
use crate::runtime::q_result::QResult;
use crate::runtime::qcontext::QContext;
use crate::runtime::value::QValue;
use crate::utils::println_utils::PrintlnUtils;
use std::rc::Rc;

/// 变量加载指令。对应 Java: com.alibaba.qlexpress4.runtime.instruction.LoadInstruction(职责:按名字加载变量值并压栈)
/// Operation: load variable from local to global scope, create when not exist
/// Input: 0
/// Output: 1 left value of local variable
///
/// Mirrors Java `LoadInstruction`.
pub struct LoadInstruction {
    error_reporter: Rc<dyn ErrorReporter>,
    name: String,
    trace_key: Option<i32>,
}

impl LoadInstruction {
    /// 构造指令,对应 Java 构造器 `LoadInstruction`。
    pub fn new(
        error_reporter: Rc<dyn ErrorReporter>,
        name: impl Into<String>,
        trace_key: Option<i32>,
    ) -> Self {
        LoadInstruction {
            error_reporter,
            name: name.into(),
            trace_key,
        }
    }

    /// 对应 Java 方法 `name`。
    pub fn name(&self) -> &str {
        &self.name
    }

    /// 对应 Java 方法 `traceKey`。
    pub fn trace_key(&self) -> Option<i32> {
        self.trace_key
    }
}

impl QLInstruction for LoadInstruction {
    /// 向下转型支持(供 api/parsecache Exporter 的 Java `instanceof` 分派)。
    fn as_any(&self) -> Option<&dyn std::any::Any> {
        Some(self)
    }

    fn execute(
        &self,
        q_context: &mut dyn QContext,
        _ql_options: &QLOptions,
    ) -> Result<QResult, QLException> {
        let symbol_value = q_context
            .get_symbol(&self.name)?
            .expect("global scope always creates symbols");
        let evaluated = symbol_value.borrow().get();
        q_context.push(QValue::Left(symbol_value));

        // trace
        with_trace(q_context, self.trace_key, |trace| {
            trace.value_evaluated(evaluated);
        });

        Ok(QResult::NEXT_INSTRUCTION)
    }

    fn stack_input(&self) -> i32 {
        0
    }

    fn stack_output(&self) -> i32 {
        1
    }

    fn println(&self, index: usize, depth: usize, debug: &mut dyn FnMut(String)) {
        PrintlnUtils::println_by_cur_depth(
            depth as i32,
            &format!("{}: Load {}", index, self.name),
            debug,
        );
    }

    fn error_reporter(&self) -> &Rc<dyn ErrorReporter> {
        &self.error_reporter
    }
}
