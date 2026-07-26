//! Cast instruction, mirroring Java `CastInstruction`.

use std::rc::Rc;

use crate::exception::error_codes;
use crate::exception::error_reporter::ErrorReporter;
use crate::exception::QLException;
use crate::ql_options::QLOptions;
use crate::ql_result::QResult;
use crate::runtime::data::convert::obj_type_convertor::ObjTypeConvertor;
use crate::runtime::instruction::QLInstruction;
use crate::runtime::member::{as_meta_class, ClassRef};
use crate::runtime::qcontext::QContext;
use crate::runtime::value::{DataValue, QValue};
use crate::utils::println_utils::PrintlnUtils;

/// Operation: force cast value to specified type
/// Input: 2 targetCls and value
/// Output: 1 casted value
///
/// Mirrors Java `CastInstruction`.
pub struct CastInstruction {
    error_reporter: Rc<dyn ErrorReporter>,
}

impl CastInstruction {
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
        let converted = match &target_clz {
            ClassRef::Primitive(target) => {
                let result = ObjTypeConvertor::cast(&value_data, *target);
                if !result.is_convertible() {
                    return Err(self.error_reporter.report_format(
                        error_codes::INCOMPATIBLE_TYPE_CAST,
                        error_codes::error_msg(error_codes::INCOMPATIBLE_TYPE_CAST),
                        &[
                            value.type_name().to_string(),
                            target_clz.java_name().to_string(),
                        ],
                    ));
                }
                result.into_converted()
            }
            ClassRef::Named(name) => {
                // Java `ObjTypeConvertor.cast` for reference types succeeds
                // when the value is assignable to the target class.
                let value_type = value_data.data_type_name();
                let assignable = name == "java.lang.Object"
                    || name == value_type
                    || matches!(&value_data, DataValue::Object(obj) if {
                        let n = obj.borrow().native_type_name().to_string();
                        n == *name
                    });
                if !assignable {
                    return Err(self.error_reporter.report_format(
                        error_codes::INCOMPATIBLE_TYPE_CAST,
                        error_codes::error_msg(error_codes::INCOMPATIBLE_TYPE_CAST),
                        &[
                            value.type_name().to_string(),
                            target_clz.java_name().to_string(),
                        ],
                    ));
                }
                value_data
            }
        };
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
