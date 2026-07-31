//! 方法引用读取指令,对应 Java `com.alibaba.qlexpress4.runtime.instruction.GetMethodInstruction`。
//! 职责:读取对象方法引用并压栈。
//! 本文件由 `field_method.rs` 拆分而来(SPEC §5.5 一类一文件),仅移动代码与补充中文注释,行为完全一致。

use crate::exception::error_codes;
use crate::exception::error_reporter::ErrorReporter;
use crate::exception::QLException;
use crate::ql_options::QLOptions;
use crate::runtime::data::lambda::QLambdaMethod;
use crate::runtime::instruction::QLInstruction;
use crate::runtime::q_result::QResult;
use crate::runtime::qcontext::QContext;
use crate::runtime::qlambda::QLambda;
use crate::runtime::value::{DataValue, QValue};
use crate::utils::println_utils::PrintlnUtils;
use std::rc::Rc;

/// 方法引用读取指令。对应 Java: com.alibaba.qlexpress4.runtime.instruction.GetMethodInstruction(职责:读取对象方法引用并压栈)
/// Operation: get specified method of object on the top of stack
/// Input: 1
/// Output: 1
///
/// Mirrors Java `GetMethodInstruction`.
pub struct GetMethodInstruction {
    error_reporter: Rc<dyn ErrorReporter>,
    method_name: String,
}

impl GetMethodInstruction {
    /// 构造指令,对应 Java 构造器 `GetMethodInstruction`。
    pub fn new(error_reporter: Rc<dyn ErrorReporter>, method_name: impl Into<String>) -> Self {
        GetMethodInstruction {
            error_reporter,
            method_name: method_name.into(),
        }
    }

    /// 对应 Java 方法 `methodName`。
    pub fn method_name(&self) -> &str {
        &self.method_name
    }
}

impl QLInstruction for GetMethodInstruction {
    /// 向下转型支持(供 api/parsecache Exporter 的 Java `instanceof` 分派)。
    fn as_any(&self) -> Option<&dyn std::any::Any> {
        Some(self)
    }

    fn execute(
        &self,
        q_context: &mut dyn QContext,
        ql_options: &QLOptions,
    ) -> Result<QResult, QLException> {
        let bean = q_context.pop().get();
        if bean.is_null() {
            if ql_options.is_avoid_null_pointer() {
                q_context.push(QValue::Data(DataValue::NULL_VALUE));
                return Ok(QResult::NEXT_INSTRUCTION);
            }
            return Err(self.error_reporter.report(
                error_codes::NULL_METHOD_ACCESS,
                error_codes::error_msg(error_codes::NULL_METHOD_ACCESS),
            ));
        }
        let bean_type_name = bean.runtime_type_name();
        if !q_context
            .q_runtime()
            .is_method_capability_allowed(&bean_type_name, &self.method_name)
        {
            return Err(crate::runtime::execution_budget::budget_error(
                crate::exception::QLExceptionKind::Runtime,
                "SANDBOX_CAPABILITY_DENIED",
                format!(
                    "method capability is not allowed: {}.{}",
                    bean_type_name, self.method_name
                ),
            ));
        }
        let registry = Rc::clone(q_context.registry());
        q_context.push(QValue::Data(DataValue::Lambda(Rc::new(QLambda::Method(
            QLambdaMethod::new(self.method_name.clone(), registry, bean),
        )))));
        Ok(QResult::NEXT_INSTRUCTION)
    }

    fn stack_input(&self) -> i32 {
        1
    }

    fn stack_output(&self) -> i32 {
        1
    }

    fn println(&self, index: usize, depth: usize, debug: &mut dyn FnMut(String)) {
        PrintlnUtils::println_by_cur_depth(
            depth as i32,
            &format!("{}: GetMethod {}", index, self.method_name),
            debug,
        );
    }

    fn error_reporter(&self) -> &Rc<dyn ErrorReporter> {
        &self.error_reporter
    }
}
