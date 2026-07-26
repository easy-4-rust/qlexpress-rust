//! Assignable values, mirroring Java `LeftValue`.

use crate::exception::error_codes;
use crate::exception::error_reporter::ErrorReporter;
use crate::exception::QLException;
use crate::runtime::data::convert::obj_type_convertor::{ObjTypeConvertor, TargetType};
use crate::runtime::value::{DataValue, Value};

/// An assignable `Value`, mirroring Java `LeftValue`.
pub trait LeftValue: Value {
    /// Java `getDefinedType`; `None` mirrors `null` (no declared type).
    fn defined_type(&self) -> Option<TargetType>;

    /// Java `setInner`: assign without conversion.
    fn set_inner(&mut self, new_value: DataValue);

    /// Java `getSymbolName`; `None` mirrors `null`.
    fn symbol_name(&self) -> Option<&str>;

    /// Java default `set(Object, ErrorReporter)`: convert to the declared
    /// type, reporting `INCOMPATIBLE_ASSIGNMENT_TYPE` when unconvertible.
    ///
    /// Note: the argument order (value type name, declared type name)
    /// faithfully reproduces the Java call, including its original order.
    fn set(
        &mut self,
        new_value: DataValue,
        error_reporter: &dyn ErrorReporter,
    ) -> Result<(), QLException> {
        let define_type = self.defined_type();
        let result = ObjTypeConvertor::cast_opt(&new_value, define_type);
        if !result.is_convertible() {
            let value_type = if new_value.is_null() {
                "null".to_string()
            } else {
                new_value.data_type_name().to_string()
            };
            let define_type_name = define_type
                .map(TargetType::java_name)
                .unwrap_or("java.lang.Object")
                .to_string();
            return Err(error_reporter.report_format(
                error_codes::INCOMPATIBLE_ASSIGNMENT_TYPE,
                error_codes::error_msg(error_codes::INCOMPATIBLE_ASSIGNMENT_TYPE),
                &[value_type, define_type_name],
            ));
        }
        self.set_inner(result.into_converted());
        Ok(())
    }
}

/// Convenience for `LeftValue` trait objects (Java uses `LeftValue` as a
/// normal interface).
impl dyn LeftValue {
    /// Helper to format this value for error messages.
    pub fn debug_value(&self) -> String {
        format!("{:?}", self.get())
    }
}
