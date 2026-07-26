//! Field/method access instructions, mirroring Java `GetFieldInstruction`,
//! `GetMethodInstruction`, `SpreadGetFieldInstruction`.

use std::rc::Rc;

use crate::exception::error_codes;
use crate::exception::error_reporter::ErrorReporter;
use crate::exception::QLException;
use crate::ql_options::QLOptions;
use crate::ql_result::QResult;
use crate::runtime::data::lambda::QLambdaMethod;
use crate::runtime::instruction::QLInstruction;
use crate::runtime::qcontext::QContext;
use crate::runtime::qlambda::QLambda;
use crate::runtime::value::{DataValue, QValue};
use crate::utils::println_utils::PrintlnUtils;

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

    pub fn field_name(&self) -> &str {
        &self.field_name
    }

    pub fn is_optional(&self) -> bool {
        self.optional
    }
}

impl QLInstruction for GetFieldInstruction {
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
            return Err(self.error_reporter.report(
                error_codes::NULL_FIELD_ACCESS,
                error_codes::error_msg(error_codes::NULL_FIELD_ACCESS),
            ));
        }
        let Some(field_value) = q_context.registry().load_field(&bean, &self.field_name) else {
            return Err(self.error_reporter.report_format(
                error_codes::FIELD_NOT_FOUND,
                error_codes::error_msg(error_codes::FIELD_NOT_FOUND),
                &[self.field_name.clone()],
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
    pub fn new(error_reporter: Rc<dyn ErrorReporter>, method_name: impl Into<String>) -> Self {
        GetMethodInstruction {
            error_reporter,
            method_name: method_name.into(),
        }
    }

    pub fn method_name(&self) -> &str {
        &self.method_name
    }
}

impl QLInstruction for GetMethodInstruction {
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
