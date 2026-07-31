//! 类型转换指令,对应 Java `com.alibaba.qlexpress4.runtime.instruction.CastInstruction`。
//! 职责:将栈顶值转换为目标类型。
//! 本文件由 `cast.rs` 拆分而来(SPEC §5.5 一类一文件),仅移动代码与补充中文注释,行为完全一致。

use crate::exception::QLException;
use crate::exception::error_codes;
use crate::exception::error_reporter::ErrorReporter;
use crate::ql_options::QLOptions;
use crate::runtime::data::convert::obj_type_convertor::ObjTypeConvertor;
use crate::runtime::instruction::QLInstruction;
use crate::runtime::member::{ClassRef, as_meta_class};
use crate::runtime::q_result::QResult;
use crate::runtime::qcontext::QContext;
use crate::runtime::value::{DataValue, QValue};
use crate::utils::println_utils::PrintlnUtils;
use std::rc::Rc;

/// 类型转换指令。对应 Java: com.alibaba.qlexpress4.runtime.instruction.CastInstruction(职责:将栈顶值转换为目标类型)
/// Operation: force cast value to specified type
/// Input: 2 targetCls and value
/// Output: 1 casted value
///
/// Mirrors Java `CastInstruction`.
pub struct CastInstruction {
    error_reporter: Rc<dyn ErrorReporter>,
}

impl CastInstruction {
    /// 构造指令,对应 Java 构造器 `CastInstruction`。
    pub fn new(error_reporter: Rc<dyn ErrorReporter>) -> Self {
        CastInstruction { error_reporter }
    }

    /// Java `popTargetClz`: `MetaClass` (or, in Java, a raw `Class`).
    fn pop_target_clz(&self, target: &DataValue) -> Result<ClassRef, QLException> {
        if let Some(meta_clz) = as_meta_class(target) {
            return Ok(meta_clz);
        }
        Err(self.error_reporter.report_format(
            error_codes::INVALID_CAST_TARGET,
            error_codes::error_msg(error_codes::INVALID_CAST_TARGET),
            &[if target.is_null() {
                "null".to_string()
            } else {
                target.data_type_name().to_string()
            }],
        ))
    }
}

impl QLInstruction for CastInstruction {
    /// 向下转型支持(供 api/parsecache Exporter 的 Java `instanceof` 分派)。
    fn as_any(&self) -> Option<&dyn std::any::Any> {
        Some(self)
    }

    fn execute(
        &self,
        q_context: &mut dyn QContext,
        _ql_options: &QLOptions,
    ) -> Result<QResult, QLException> {
        let value = q_context.pop();
        let target_clz = self.pop_target_clz(&q_context.pop().get())?;
        let value_data = value.get();
        if value_data.is_null() {
            q_context.push(QValue::Data(DataValue::NULL_VALUE));
            return Ok(QResult::NEXT_INSTRUCTION);
        }
        let result = ObjTypeConvertor::cast_class(
            &value_data,
            Some(&target_clz),
            Some(q_context.registry().as_ref()),
        );
        if !result.is_convertible() {
            return Err(self.error_reporter.report_format(
                error_codes::INCOMPATIBLE_TYPE_CAST,
                error_codes::error_msg(error_codes::INCOMPATIBLE_TYPE_CAST),
                &[
                    value_data.runtime_type_name(),
                    target_clz.java_name().to_string(),
                ],
            ));
        }
        let converted = result.into_converted();
        q_context.push(QValue::Data(converted));
        Ok(QResult::NEXT_INSTRUCTION)
    }

    fn stack_input(&self) -> i32 {
        2
    }

    fn stack_output(&self) -> i32 {
        1
    }

    fn println(&self, index: usize, depth: usize, debug: &mut dyn FnMut(String)) {
        PrintlnUtils::println_by_cur_depth(depth as i32, &format!("{index}: Cast"), debug);
    }

    fn error_reporter(&self) -> &Rc<dyn ErrorReporter> {
        &self.error_reporter
    }
}
