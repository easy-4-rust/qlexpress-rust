//! 方法调用指令,对应 Java `com.alibaba.qlexpress4.runtime.instruction.MethodInvokeInstruction`。
//! 职责:调用对象方法。
//! 本文件由 `call.rs` 拆分而来(SPEC §5.5 一类一文件),仅移动代码与补充中文注释,行为完全一致。

use crate::exception::error_codes;
use crate::exception::error_reporter::ErrorReporter;
use crate::exception::QLException;
use crate::ql_options::QLOptions;
use crate::runtime::instruction::QLInstruction;
use crate::runtime::member::find_method_and_invoke;
use crate::runtime::q_result::QResult;
use crate::runtime::qcontext::QContext;
use crate::runtime::value::{DataValue, QValue};
use crate::utils::println_utils::PrintlnUtils;
use std::rc::Rc;

/// 方法调用指令。对应 Java: com.alibaba.qlexpress4.runtime.instruction.MethodInvokeInstruction(职责:调用对象方法)
/// Operation: invoke specified method of object on the top of stack
/// Input: ${argNum} + 1
/// Output: 1, method return value, null for void method
///
/// equivalent to GetMethodInstruction + CallInstruction
///
/// Mirrors Java `MethodInvokeInstruction`.
pub struct MethodInvokeInstruction {
    error_reporter: Rc<dyn ErrorReporter>,
    method_name: String,
    arg_num: usize,
    optional: bool,
}

impl MethodInvokeInstruction {
    /// 构造指令,对应 Java 构造器 `MethodInvokeInstruction`。
    pub fn new(
        error_reporter: Rc<dyn ErrorReporter>,
        method_name: impl Into<String>,
        arg_num: usize,
        optional: bool,
    ) -> Self {
        MethodInvokeInstruction {
            error_reporter,
            method_name: method_name.into(),
            arg_num,
            optional,
        }
    }

    /// 对应 Java 方法 `methodName`。
    pub fn method_name(&self) -> &str {
        &self.method_name
    }

    /// 对应 Java 方法 `argNum`。
    pub fn arg_num(&self) -> usize {
        self.arg_num
    }

    /// 对应 Java 方法 `isOptional`。
    pub fn is_optional(&self) -> bool {
        self.optional
    }
}

impl QLInstruction for MethodInvokeInstruction {
    /// 向下转型支持(供 api/parsecache Exporter 的 Java `instanceof` 分派)。
    fn as_any(&self) -> Option<&dyn std::any::Any> {
        Some(self)
    }

    fn execute(
        &self,
        q_context: &mut dyn QContext,
        ql_options: &QLOptions,
    ) -> Result<QResult, QLException> {
        let parameters = q_context.pop_n(self.arg_num + 1);
        let bean = parameters.get(0).expect("bean slot popped").get();
        let params: Vec<DataValue> = (0..self.arg_num)
            .map(|i| parameters.get_value(i + 1))
            .collect();
        if bean.is_null() {
            if ql_options.is_avoid_null_pointer() || self.optional {
                q_context.push(QValue::Data(DataValue::NULL_VALUE));
                return Ok(QResult::NEXT_INSTRUCTION);
            }
            return Err(self.error_reporter.report(
                error_codes::NULL_METHOD_ACCESS,
                error_codes::error_msg(error_codes::NULL_METHOD_ACCESS),
            ));
        }
        let invoke_res = find_method_and_invoke(
            &bean,
            &self.method_name,
            &params,
            q_context.registry(),
            &*self.error_reporter,
        )?;
        q_context.push(invoke_res);
        Ok(QResult::NEXT_INSTRUCTION)
    }

    fn stack_input(&self) -> i32 {
        self.arg_num as i32 + 1
    }

    fn stack_output(&self) -> i32 {
        1
    }

    fn println(&self, index: usize, depth: usize, debug: &mut dyn FnMut(String)) {
        PrintlnUtils::println_by_cur_depth(
            depth as i32,
            &format!(
                "{}: MethodInvoke {} with argNum {}",
                index, self.method_name, self.arg_num
            ),
            debug,
        );
    }

    fn error_reporter(&self) -> &Rc<dyn ErrorReporter> {
        &self.error_reporter
    }
}
