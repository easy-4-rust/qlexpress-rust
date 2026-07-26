//! Method-parameter conversion (incl. varargs), mirroring Java
//! `ParametersTypeConvertor`.

use crate::runtime::value::DataValue;

use super::obj_type_convertor::{ObjTypeConvertor, TargetType};

pub struct ParametersTypeConvertor;

impl ParametersTypeConvertor {
    /// Java `ParametersTypeConvertor.cast(Object[], Class<?>[], boolean)`.
    ///
    /// Like the Java version, convertibility is *not* checked: an
    /// unconvertible argument becomes `DataValue::Null` (Java's `null`).
    ///
    /// For varargs, the last entry of `param_types` is the item type (Java
    /// reads the component type of the trailing array parameter); trailing
    /// arguments are collected into a [`DataValue::Array`].
    pub fn cast(
        arguments: &[DataValue],
        param_types: &[TargetType],
        is_var_arg: bool,
    ) -> Vec<DataValue> {
        if !is_var_arg {
            return arguments
                .iter()
                .zip(param_types.iter())
                .map(|(argument, param_type)| {
                    ObjTypeConvertor::cast(argument, *param_type).into_converted()
                })
                .collect();
        }

        debug_assert!(!param_types.is_empty());
        let item_type = *param_types.last().expect("vararg param types must be non-empty");
        let var_arg_start = param_types.len() - 1;

        let var_args: Vec<DataValue> = arguments[var_arg_start.min(arguments.len())..]
            .iter()
            .map(|argument| ObjTypeConvertor::cast(argument, item_type).into_converted())
            .collect();

        let mut result: Vec<DataValue> = param_types[..var_arg_start]
            .iter()
            .zip(arguments.iter())
            .map(|(param_type, argument)| {
                ObjTypeConvertor::cast(argument, *param_type).into_converted()
            })
            .collect();
        result.push(DataValue::array(var_args));
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn non_vararg_casts_elementwise() {
        let args = vec![DataValue::Int(1), DataValue::Double(2.5)];
        let result = ParametersTypeConvertor::cast(
            &args,
            &[TargetType::Long, TargetType::Int],
            false,
        );
        assert_eq!(result, vec![DataValue::Long(1), DataValue::Int(2)]);
    }

    #[test]
    fn unconvertible_becomes_null_like_java() {
        let args = vec![DataValue::Str("x".into())];
        let result = ParametersTypeConvertor::cast(&args, &[TargetType::Int], false);
        assert_eq!(result, vec![DataValue::Null]);
    }

    #[test]
    fn vararg_tail_is_collected_into_array() {
        let args = vec![
            DataValue::Int(1),
            DataValue::Int(2),
            DataValue::Int(3),
        ];
        let result = ParametersTypeConvertor::cast(&args, &[TargetType::Long, TargetType::Int], true);
        assert_eq!(
            result,
            vec![
                DataValue::Long(1),
                DataValue::array(vec![DataValue::Int(2), DataValue::Int(3)])
            ]
        );
    }

    #[test]
    fn vararg_with_no_tail_arguments() {
        let args = vec![DataValue::Int(1)];
        let result = ParametersTypeConvertor::cast(&args, &[TargetType::Long, TargetType::Int], true);
        assert_eq!(result, vec![DataValue::Long(1), DataValue::array(vec![])]);
    }
}
