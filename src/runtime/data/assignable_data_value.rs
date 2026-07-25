//! A named, assignable variable slot, mirroring Java `AssignableDataValue`.

use crate::runtime::data::convert::obj_type_convertor::TargetType;
use crate::runtime::left_value::LeftValue;
use crate::runtime::value::{DataValue, Value};

/// Mirrors Java `AssignableDataValue`: a symbol table entry with an optional
/// declared type.
pub struct AssignableDataValue {
    symbol_name: Option<String>,
    value: DataValue,
    define_type: Option<TargetType>,
}

impl AssignableDataValue {
    /// Java `AssignableDataValue(String symbolName, Object value)`.
    pub fn new(symbol_name: impl Into<String>, value: DataValue) -> Self {
        AssignableDataValue {
            symbol_name: Some(symbol_name.into()),
            value,
            define_type: None,
        }
    }

    /// Java `AssignableDataValue(String symbolName, Object value,
    /// Class<?> defineType)`.
    pub fn with_type(
        symbol_name: impl Into<String>,
        value: DataValue,
        define_type: TargetType,
    ) -> Self {
        AssignableDataValue {
            symbol_name: Some(symbol_name.into()),
            value,
            define_type: Some(define_type),
        }
    }
}

impl Value for AssignableDataValue {
    fn get(&self) -> DataValue {
        self.value.clone()
    }

    fn type_name(&self) -> &'static str {
        self.value.data_type_name()
    }
}

impl LeftValue for AssignableDataValue {
    fn defined_type(&self) -> Option<TargetType> {
        self.define_type
    }

    fn set_inner(&mut self, new_value: DataValue) {
        self.value = new_value;
    }

    fn symbol_name(&self) -> Option<&str> {
        self.symbol_name.as_deref()
    }
}

impl std::fmt::Debug for AssignableDataValue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AssignableDataValue")
            .field("symbol_name", &self.symbol_name)
            .field("value", &self.value)
            .field("define_type", &self.define_type)
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::exception::error_codes;
    use crate::exception::pure_err_reporter::PureErrReporter;

    #[test]
    fn untyped_set_accepts_anything() {
        let mut var = AssignableDataValue::new("a", DataValue::Int(1));
        var.set(DataValue::Str("s".into()), &PureErrReporter::INSTANCE).unwrap();
        assert_eq!(var.get(), DataValue::Str("s".into()));
        assert_eq!(var.symbol_name(), Some("a"));
        assert_eq!(var.defined_type(), None);
    }

    #[test]
    fn typed_set_converts_compatible_value() {
        let mut var = AssignableDataValue::with_type("n", DataValue::Int(0), TargetType::Long);
        var.set(DataValue::Int(7), &PureErrReporter::INSTANCE).unwrap();
        assert_eq!(var.get(), DataValue::Long(7));
    }

    #[test]
    fn typed_set_rejects_incompatible_value_with_java_message() {
        let mut var = AssignableDataValue::with_type("n", DataValue::Int(0), TargetType::Int);
        let err = var
            .set(DataValue::Str("x".into()), &PureErrReporter::INSTANCE)
            .unwrap_err();
        assert_eq!(err.error_code(), error_codes::INCOMPATIBLE_ASSIGNMENT_TYPE);
        // Faithful to Java's (valueType, defineType) argument order.
        assert_eq!(
            err.reason(),
            "variable declared type java.lang.String, assigned with incompatible value type java.lang.Integer"
        );
    }
}
