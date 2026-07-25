//! new 实例指令,对应 Java `com.alibaba.qlexpress4.runtime.instruction.NewInstanceInstruction`。
//! 职责:创建对象实例。
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
use crate::runtime::value::QValue;
use crate::utils::println_utils::PrintlnUtils;

/// new 实例指令。对应 Java: com.alibaba.qlexpress4.runtime.instruction.NewInstanceInstruction(职责:创建对象实例)
/// Operation: new a instance with top ${argNum} stack element
/// Input: ${argNum}
/// Output: 1
///
/// Mirrors Java `NewInstanceInstruction`.
pub struct NewInstanceInstruction {
    error_reporter: Rc<dyn ErrorReporter>,
    new_cls: ClassRef,
    arg_num: usize,
}

impl NewInstanceInstruction {
    /// 构造指令,对应 Java 构造器 `NewInstanceInstruction`。
    pub fn new(
        error_reporter: Rc<dyn ErrorReporter>,
        new_cls: ClassRef,
        arg_num: usize,
    ) -> Self {
        NewInstanceInstruction {
            error_reporter,
            new_cls,
            arg_num,
        }
    }

    /// 对应 Java 方法 `newCls`。
    pub fn new_cls(&self) -> &ClassRef {
        &self.new_cls
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
        let params = q_context.pop_n(self.arg_num);
        let param_values = params.values();
        let Some(constructor) = q_context.registry().load_constructor(&self.new_cls) else {
            return Err(self.error_reporter.report(
                error_codes::INVOKE_CONSTRUCTOR_UNKNOWN_ERROR,
                error_codes::error_msg(error_codes::INVOKE_CONSTRUCTOR_UNKNOWN_ERROR),
            ));
        };
        let new_object = constructor(&param_values).map_err(|err| {
            self.error_reporter.report_with_catch(
                err.catch_obj().cloned(),
                error_codes::INVOKE_CONSTRUCTOR_INNER_ERROR,
                error_codes::error_msg(error_codes::INVOKE_CONSTRUCTOR_INNER_ERROR),
            )
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
                "{}: New instace of cls {} with {} params",
                index,
                self.new_cls.simple_name(),
                self.arg_num
            ),
            debug,
        );
    }

    fn error_reporter(&self) -> &Rc<dyn ErrorReporter> {
        &self.error_reporter
    }
}

