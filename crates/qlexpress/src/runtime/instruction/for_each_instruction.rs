//! for-each 循环指令,对应 Java `com.alibaba.qlexpress4.runtime.instruction.ForEachInstruction`。
//! 职责:遍历集合/数组的循环执行体。
//! 本文件由 `flow.rs` 拆分而来(SPEC §5.5 一类一文件),仅移动代码与补充中文注释,行为完全一致。

use crate::exception::error_codes;
use crate::exception::error_reporter::ErrorReporter;
use crate::exception::QLException;
use crate::ql_options::QLOptions;
use crate::runtime::instruction::QLInstruction;
use crate::runtime::member::ClassRef;
use crate::runtime::q_result::QResult;
use crate::runtime::qcontext::QContext;
use crate::runtime::qlambda_definition::QLambdaDefinition;
use crate::runtime::value::DataValue;
use crate::utils::println_utils::PrintlnUtils;
use std::rc::Rc;

/// for-each 循环指令。对应 Java: com.alibaba.qlexpress4.runtime.instruction.ForEachInstruction(职责:遍历集合/数组的循环执行体)
/// Operation: process each element in iterable object on top of stack,
/// Input: 1
/// Output: 0
///
/// Mirrors Java `ForEachInstruction`.
pub struct ForEachInstruction {
    error_reporter: Rc<dyn ErrorReporter>,
    body: Rc<dyn QLambdaDefinition>,
    target_error_reporter: Rc<dyn ErrorReporter>,
    it_cls: ClassRef,
}

impl ForEachInstruction {
    /// 构造指令,对应 Java 构造器 `ForEachInstruction`。
    pub fn new(
        error_reporter: Rc<dyn ErrorReporter>,
        body: Rc<dyn QLambdaDefinition>,
        it_cls: ClassRef,
        target_error_reporter: Rc<dyn ErrorReporter>,
    ) -> Self {
        ForEachInstruction {
            error_reporter,
            body,
            target_error_reporter,
            it_cls,
        }
    }

    /// 对应 Java 方法 `body`。
    pub fn body(&self) -> &Rc<dyn QLambdaDefinition> {
        &self.body
    }

    /// 对应 Java 方法 `targetErrorReporter`。
    pub fn target_error_reporter(&self) -> &Rc<dyn ErrorReporter> {
        &self.target_error_reporter
    }

    /// 对应 Java 方法 `itCls`。
    pub fn it_cls(&self) -> &ClassRef {
        &self.it_cls
    }
}

impl QLInstruction for ForEachInstruction {
    /// 向下转型支持(供 api/parsecache Exporter 的 Java `instanceof` 分派)。
    fn as_any(&self) -> Option<&dyn std::any::Any> {
        Some(self)
    }

    fn execute(
        &self,
        q_context: &mut dyn QContext,
        ql_options: &QLOptions,
    ) -> Result<QResult, QLException> {
        let may_be_iterable = q_context.pop().get();
        // Java: array → ReflectArrayIterable; Iterable → as-is; else error.
        let items: Vec<DataValue> = match &may_be_iterable {
            DataValue::Array(arr) => arr.borrow().clone(),
            DataValue::List(list) => list.borrow().clone(),
            _ => {
                return Err(self.target_error_reporter.report(
                    error_codes::FOR_EACH_ITERABLE_REQUIRED,
                    error_codes::error_msg(error_codes::FOR_EACH_ITERABLE_REQUIRED),
                ))
            }
        };
        let body_lambda = Rc::clone(&self.body).to_lambda(q_context, ql_options, true);
        // forEachBody:
        for item in items {
            match body_lambda.call(std::slice::from_ref(&item)) {
                Ok(body_result) => match body_result {
                    QResult::Return(_) => return Ok(body_result),
                    QResult::Break => break,
                    _ => {}
                },
                Err(err) => {
                    // Java: UserDefineException (lambda argument conversion)
                    // → FOR_EACH_TYPE_MISMATCH; QLRuntimeException → rethrow;
                    // else FOR_EACH_UNKNOWN_ERROR.
                    if err.error_code() == error_codes::INVALID_ARGUMENT {
                        return Err(self.error_reporter.report_format(
                            error_codes::FOR_EACH_TYPE_MISMATCH,
                            error_codes::error_msg(error_codes::FOR_EACH_TYPE_MISMATCH),
                            &[
                                self.it_cls.java_name().to_string(),
                                if item.is_null() {
                                    "null".to_string()
                                } else {
                                    item.data_type_name().to_string()
                                },
                            ],
                        ));
                    }
                    return Err(err);
                }
            }
        }
        Ok(QResult::NEXT_INSTRUCTION)
    }

    fn stack_input(&self) -> i32 {
        1
    }

    fn stack_output(&self) -> i32 {
        0
    }

    fn println(&self, index: usize, depth: usize, debug: &mut dyn FnMut(String)) {
        PrintlnUtils::println_by_cur_depth(depth as i32, &format!("{index}: ForEach"), debug);
        self.body.println(depth + 1, debug);
    }

    fn error_reporter(&self) -> &Rc<dyn ErrorReporter> {
        &self.error_reporter
    }
}
