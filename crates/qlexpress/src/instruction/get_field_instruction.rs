//! 字段读取指令,对应 Java `com.alibaba.qlexpress4.runtime.instruction.GetFieldInstruction`。
//! 职责:读取对象字段并压栈。
//! 本文件由 `field_method.rs` 拆分而来(SPEC §5.5 一类一文件),仅移动代码与补充中文注释,行为完全一致。

use crate::exception::QLException;
use crate::exception::error_codes;
use crate::exception::error_reporter::ErrorReporter;
use crate::ql_options::QLOptions;
use crate::runtime::instruction::QLInstruction;
use crate::runtime::opaque_native_object::OpaqueNativeObject;
use crate::runtime::q_result::QResult;
use crate::runtime::qcontext::QContext;
use crate::runtime::value::{DataValue, QValue};
use crate::utils::println_utils::PrintlnUtils;
use std::rc::Rc;

/// 字段读取指令。对应 Java: com.alibaba.qlexpress4.runtime.instruction.GetFieldInstruction(职责:读取对象字段并压栈)
/// Operation: get specified field of object on the top of stack
/// Input: 1
/// Output: 1
///
/// Mirrors Java `GetFieldInstruction`.
pub struct GetFieldInstruction {
    error_reporter: Rc<dyn ErrorReporter>,
    field_name: String,
    optional: bool,
}

impl GetFieldInstruction {
    /// 构造指令,对应 Java 构造器 `GetFieldInstruction`。
    pub fn new(
        error_reporter: Rc<dyn ErrorReporter>,
        field_name: impl Into<String>,
        optional: bool,
    ) -> Self {
        GetFieldInstruction {
            error_reporter,
            field_name: field_name.into(),
            optional,
        }
    }

    /// 对应 Java 方法 `fieldName`。
    pub fn field_name(&self) -> &str {
        &self.field_name
    }

    /// 对应 Java 方法 `isOptional`。
    pub fn is_optional(&self) -> bool {
        self.optional
    }
}

impl QLInstruction for GetFieldInstruction {
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
            if ql_options.is_avoid_null_pointer() || self.optional {
                q_context.push(QValue::Data(DataValue::NULL_VALUE));
                return Ok(QResult::NEXT_INSTRUCTION);
            }
            return Err(self.error_reporter.report_with_catch(
                Some(OpaqueNativeObject::new("java.lang.NullPointerException").into_data_value()),
                error_codes::NULL_FIELD_ACCESS,
                error_codes::error_msg(error_codes::NULL_FIELD_ACCESS),
            ));
        }
        let Some(field_value) = q_context.registry().load_field(&bean, &self.field_name) else {
            return Err(self.error_reporter.report_format(
                error_codes::FIELD_NOT_FOUND,
                error_codes::error_msg(error_codes::FIELD_NOT_FOUND),
                std::slice::from_ref(&self.field_name),
            ));
        };
        q_context.push(field_value);
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
            &format!("{}: GetField {}", index, self.field_name),
            debug,
        );
    }

    fn error_reporter(&self) -> &Rc<dyn ErrorReporter> {
        &self.error_reporter
    }
}
