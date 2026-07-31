//! 局部变量定义指令,对应 Java `com.alibaba.qlexpress4.runtime.instruction.DefineLocalInstruction`。
//! 职责:在当前作用域定义局部变量。
//! 本文件由 `scope.rs` 拆分而来(SPEC §5.5 一类一文件),仅移动代码与补充中文注释,行为完全一致。

use crate::exception::QLException;
use crate::exception::error_codes;
use crate::exception::error_reporter::ErrorReporter;
use crate::ql_options::QLOptions;
use crate::runtime::class_ref::ClassRef;
use crate::runtime::data::convert::obj_type_convertor::ObjTypeConvertor;
use crate::runtime::instruction::QLInstruction;
use crate::runtime::q_result::QResult;
use crate::runtime::qcontext::QContext;
use crate::utils::println_utils::PrintlnUtils;
use std::rc::Rc;

/// 局部变量定义指令。对应 Java: com.alibaba.qlexpress4.runtime.instruction.DefineLocalInstruction(职责:在当前作用域定义局部变量)
/// Operation: define a symbol in local scope
/// Input: 1 symbol init value
/// Output: 0
///
/// Mirrors Java `DefineLocalInstruction`.
pub struct DefineLocalInstruction {
    error_reporter: Rc<dyn ErrorReporter>,
    variable_name: String,
    define_clz: Option<ClassRef>,
}

impl DefineLocalInstruction {
    /// 构造指令,对应 Java 构造器 `DefineLocalInstruction`。
    pub fn new(
        error_reporter: Rc<dyn ErrorReporter>,
        variable_name: impl Into<String>,
        define_clz: Option<ClassRef>,
    ) -> Self {
        DefineLocalInstruction {
            error_reporter,
            variable_name: variable_name.into(),
            define_clz,
        }
    }

    /// 对应 Java 方法 `variableName`。
    pub fn variable_name(&self) -> &str {
        &self.variable_name
    }

    /// 对应 Java 方法 `defineClz`。
    pub fn define_clz(&self) -> Option<&ClassRef> {
        self.define_clz.as_ref()
    }
}

impl QLInstruction for DefineLocalInstruction {
    /// 向下转型支持(供 api/parsecache Exporter 的 Java `instanceof` 分派)。
    fn as_any(&self) -> Option<&dyn std::any::Any> {
        Some(self)
    }

    fn execute(
        &self,
        q_context: &mut dyn QContext,
        _ql_options: &QLOptions,
    ) -> Result<QResult, QLException> {
        let init_value = q_context.pop().get();
        let ql_convert_result = ObjTypeConvertor::cast_class(
            &init_value,
            self.define_clz.as_ref(),
            Some(q_context.registry().as_ref()),
        );
        if !ql_convert_result.is_convertible() {
            // Java reportFormat(INCOMPATIBLE_ASSIGNMENT_TYPE, msg,
            //   defineClz.getName(), initValue class name)
            return Err(self.error_reporter.report_format(
                error_codes::INCOMPATIBLE_ASSIGNMENT_TYPE,
                error_codes::error_msg(error_codes::INCOMPATIBLE_ASSIGNMENT_TYPE),
                &[
                    self.define_clz
                        .as_ref()
                        .map(ClassRef::java_name)
                        .unwrap_or("java.lang.Object")
                        .to_string(),
                    if init_value.is_null() {
                        "null".to_string()
                    } else {
                        init_value.runtime_type_name()
                    },
                ],
            ));
        }
        q_context.define_local_symbol(
            &self.variable_name,
            self.define_clz.clone(),
            ql_convert_result.into_converted(),
        );
        Ok(QResult::NEXT_INSTRUCTION)
    }

    fn stack_input(&self) -> i32 {
        1
    }

    fn stack_output(&self) -> i32 {
        0
    }

    fn println(&self, index: usize, depth: usize, debug: &mut dyn FnMut(String)) {
        PrintlnUtils::println_by_cur_depth(
            depth as i32,
            &format!("{}: DefineLocal {}", index, self.variable_name),
            debug,
        );
    }

    fn error_reporter(&self) -> &Rc<dyn ErrorReporter> {
        &self.error_reporter
    }
}
