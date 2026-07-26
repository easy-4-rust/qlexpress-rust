//! new 实例指令,对应 Java `com.alibaba.qlexpress4.runtime.instruction.NewInstanceInstruction`。
//! 职责:调用构造器创建实例。
//! 本文件由 `new_instance.rs` 拆分而来(SPEC §5.5 一类一文件),仅移动代码与补充中文注释,行为完全一致。

use std::rc::Rc;
use crate::exception::error_codes;
use crate::exception::error_reporter::ErrorReporter;
use crate::exception::QLException;
use crate::ql_options::QLOptions;
use crate::runtime::q_result::QResult;
use crate::runtime::instruction::QLInstruction;
use crate::runtime::member::ClassRef;
use crate::runtime::qcontext::QContext;
use crate::runtime::value::{DataValue, QValue};
use crate::utils::println_utils::PrintlnUtils;

/// new 实例指令。对应 Java: com.alibaba.qlexpress4.runtime.instruction.NewInstanceInstruction(职责:调用构造器创建实例)
/// Operation: new an object of specified class
/// Input: ${argNum} + 1
/// Output: 1
///
/// Mirrors Java `NewInstanceInstruction`.
pub struct NewInstanceInstruction {
    error_reporter: Rc<dyn ErrorReporter>,
    new_clz: ClassRef,
    arg_num: usize,
}

impl NewInstanceInstruction {
    /// 构造指令,对应 Java 构造器 `NewInstanceInstruction`。
    pub fn new(error_reporter: Rc<dyn ErrorReporter>, new_clz: ClassRef, arg_num: usize) -> Self {
        NewInstanceInstruction {
            error_reporter,
            new_clz,
            arg_num,
        }
    }

    /// 对应 Java 方法 `newClz`。
    pub fn new_clz(&self) -> &ClassRef {
        &self.new_clz
    }

    /// 对应 Java 方法 `argNum`。
    pub fn arg_num(&self) -> usize {
        self.arg_num
    }
}

impl QLInstruction for NewInstanceInstruction {
    /// 向下转型支持(供 api/parsecache Exporter 的 Java `instanceof` 分派)。
    fn as_any(&self) -> Option<&dyn std::any::Any> {
        Some(self)
    }

    fn execute(
        &self,
        q_context: &mut dyn QContext,
        _ql_options: &QLOptions,
    ) -> Result<QResult, QLException> {
        let objs: Vec<DataValue> = if self.arg_num == 0 {
            Vec::new()
        } else {
            q_context.pop_n(self.arg_num).values()
        };
        let Some(constructor) = q_context.registry().load_constructor(&self.new_clz) else {
            let param_types = objs
                .iter()
                .map(|o| o.data_type_name().to_string())
                .collect::<Vec<_>>()
                .join(", ");
            return Err(self.error_reporter.report_format(
                error_codes::NO_SUITABLE_CONSTRUCTOR,
                error_codes::error_msg(error_codes::NO_SUITABLE_CONSTRUCTOR),
                &[format!("[{param_types}]")],
            ));
        };
        // Java: InvocationTargetException → INVOKE_CONSTRUCTOR_INNER_ERROR,
        // other reflection failures → INVOKE_CONSTRUCTOR_UNKNOWN_ERROR. The
        // registry constructor reports `QLException` directly; an
        // uncoded inner failure is normalised to INVOKE_CONSTRUCTOR_INNER_ERROR.
        let new_object = constructor(&objs).map_err(|err| {
            if err.error_code() == error_codes::INVOKE_CONSTRUCTOR_INNER_ERROR
                || err.error_code() == error_codes::INVOKE_CONSTRUCTOR_UNKNOWN_ERROR
            {
                err
            } else {
                self.error_reporter.report_with_catch(
                    err.catch_obj().cloned(),
                    error_codes::INVOKE_CONSTRUCTOR_INNER_ERROR,
                    error_codes::error_msg(error_codes::INVOKE_CONSTRUCTOR_INNER_ERROR),
                )
            }
        })?;
        q_context.push(QValue::Data(new_object));
        Ok(QResult::NEXT_INSTRUCTION)
    }

    fn stack_input(&self) -> i32 {
        self.arg_num as i32
    }

    fn stack_output(&self) -> i32 {
        1
    }

    fn println(&self, index: usize, depth: usize, debug: &mut dyn FnMut(String)) {
        PrintlnUtils::println_by_cur_depth(
            depth as i32,
            &format!(
                "{}: New instance of cls {} with argNum {}",
                index,
                self.new_clz.simple_name(),
                self.arg_num
            ),
            debug,
        );
    }

    fn error_reporter(&self) -> &Rc<dyn ErrorReporter> {
        &self.error_reporter
    }
}

