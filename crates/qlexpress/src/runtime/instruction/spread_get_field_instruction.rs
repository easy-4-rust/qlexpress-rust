//! 展开字段读取指令,对应 Java `com.alibaba.qlexpress4.runtime.instruction.SpreadGetFieldInstruction`。
//! 职责:对集合元素逐个读取字段并收集结果。
//! 本文件由 `field_method.rs` 拆分而来(SPEC §5.5 一类一文件),仅移动代码与补充中文注释,行为完全一致。

use crate::exception::error_codes;
use crate::exception::error_reporter::ErrorReporter;
use crate::exception::QLException;
use crate::ql_options::QLOptions;
use crate::runtime::instruction::QLInstruction;
use crate::runtime::q_result::QResult;
use crate::runtime::qcontext::QContext;
use crate::runtime::value::{DataValue, QValue};
use crate::utils::println_utils::PrintlnUtils;
use std::rc::Rc;

/// 展开字段读取指令。对应 Java: com.alibaba.qlexpress4.runtime.instruction.SpreadGetFieldInstruction(职责:对集合元素逐个读取字段并收集结果)
/// Operation: get field of each object in the list
/// Input: 1
/// Output: 1, a list composed of field values
///
/// Mirrors Java `SpreadGetFieldInstruction`.
pub struct SpreadGetFieldInstruction {
    error_reporter: Rc<dyn ErrorReporter>,
    field_name: String,
}

/// Java `SpreadGetFieldInstruction.KEY`.
const KEY: &str = "key";
/// Java `SpreadGetFieldInstruction.VALUE`.
const VALUE: &str = "value";

impl SpreadGetFieldInstruction {
    pub fn new(error_reporter: Rc<dyn ErrorReporter>, field_name: impl Into<String>) -> Self {
        SpreadGetFieldInstruction {
            error_reporter,
            field_name: field_name.into(),
        }
    }

    pub fn field_name(&self) -> &str {
        &self.field_name
    }

    /// Java `isTraversable` (Iterable or array → List/Array here).
    fn is_traversable(obj: &DataValue) -> bool {
        matches!(obj, DataValue::List(_) | DataValue::Array(_))
    }

    /// Java `spreadGetFieldRecursive`.
    fn spread_recursive(
        &self,
        traversable: &DataValue,
        q_context: &dyn QContext,
        ql_options: &QLOptions,
        result: &mut Vec<DataValue>,
    ) -> Result<(), QLException> {
        let items = match traversable {
            DataValue::List(l) => l.borrow().clone(),
            DataValue::Array(a) => a.borrow().clone(),
            _ => vec![],
        };
        for item in items {
            self.process_item(&item, q_context, ql_options, result)?;
        }
        Ok(())
    }

    /// Java `processItem`.
    fn process_item(
        &self,
        item: &DataValue,
        q_context: &dyn QContext,
        ql_options: &QLOptions,
        result: &mut Vec<DataValue>,
    ) -> Result<(), QLException> {
        if item.is_null() {
            if ql_options.is_avoid_null_pointer() {
                result.push(DataValue::Null);
                return Ok(());
            }
            return Err(self.error_reporter.report(
                error_codes::NULL_FIELD_ACCESS,
                error_codes::error_msg(error_codes::NULL_FIELD_ACCESS),
            ));
        }

        // Check if the field exists at current level
        match q_context.registry().load_field(item, &self.field_name) {
            Some(field_value) => {
                result.push(field_value.get());
                Ok(())
            }
            None => {
                // Field not found, check if item is nested list/array
                if Self::is_traversable(item) {
                    self.spread_recursive(item, q_context, ql_options, result)
                } else {
                    Err(self.error_reporter.report_format(
                        error_codes::FIELD_NOT_FOUND,
                        error_codes::error_msg(error_codes::FIELD_NOT_FOUND),
                        &[self.field_name.clone()],
                    ))
                }
            }
        }
    }
}

impl QLInstruction for SpreadGetFieldInstruction {
    /// 向下转型支持(供 api/parsecache Exporter 的 Java `instanceof` 分派)。
    fn as_any(&self) -> Option<&dyn std::any::Any> {
        Some(self)
    }

    fn execute(
        &self,
        q_context: &mut dyn QContext,
        ql_options: &QLOptions,
    ) -> Result<QResult, QLException> {
        let traversable = q_context.pop().get();
        if traversable.is_null() {
            if ql_options.is_avoid_null_pointer() {
                q_context.push(QValue::Data(DataValue::NULL_VALUE));
                return Ok(QResult::NEXT_INSTRUCTION);
            }
            return Err(self.error_reporter.report_format(
                error_codes::NONTRAVERSABLE_OBJECT,
                error_codes::error_msg(error_codes::NONTRAVERSABLE_OBJECT),
                &["null".to_string()],
            ));
        }

        // Map special handling for key/value access
        if let DataValue::Map(map) = &traversable {
            let mut result = Vec::new();
            for (k, v) in map.borrow().entries().to_vec() {
                if self.field_name == KEY {
                    result.push(k);
                } else if self.field_name == VALUE {
                    result.push(v);
                } else {
                    return Err(self.error_reporter.report_format(
                        error_codes::FIELD_NOT_FOUND,
                        error_codes::error_msg(error_codes::FIELD_NOT_FOUND),
                        &[self.field_name.clone()],
                    ));
                }
            }
            q_context.push(QValue::Data(DataValue::list(result)));
        } else if Self::is_traversable(&traversable) {
            let mut result = Vec::new();
            self.spread_recursive(&traversable, q_context, ql_options, &mut result)?;
            q_context.push(QValue::Data(DataValue::list(result)));
        } else {
            return Err(self.error_reporter.report_format(
                error_codes::NONTRAVERSABLE_OBJECT,
                error_codes::error_msg(error_codes::NONTRAVERSABLE_OBJECT),
                &[traversable.data_type_name().to_string()],
            ));
        }
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
            &format!("{}: SpreadGetField {}", index, self.field_name),
            debug,
        );
    }

    fn error_reporter(&self) -> &Rc<dyn ErrorReporter> {
        &self.error_reporter
    }
}
