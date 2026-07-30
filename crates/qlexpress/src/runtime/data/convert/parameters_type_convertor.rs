//! Method-parameter conversion (incl. varargs), mirroring Java
//! `ParametersTypeConvertor`.

use crate::runtime::value::DataValue;

use super::obj_type_convertor::{ObjTypeConvertor, TargetType};

/// 将脚本参数逐项转换为宿主方法声明类型，并负责收集变长参数。
/// 对应 Java: `com.alibaba.qlexpress4.runtime.data.convert.ParametersTypeConvertor`。
pub struct ParametersTypeConvertor;

impl ParametersTypeConvertor {
    /// 按 Java 类型转换规则转换输入值。
    /// 参数：`arguments`、`param_types`、`is_var_arg`；返回：`Vec<DataValue>`。
    /// 对应或承接 Java 源文件：`com/alibaba/qlexpress4/runtime/data/convert/ParametersTypeConvertor.java`，方法 `cast`；Rust 侧按所有权与 `Result` 语义适配。
    /// Java `ParametersTypeConvertor.cast(Object[], Class<?>[], boolean)`.
    ///
    /// Like the Java version, convertibility is *not* checked: an
    /// unconvertible argument becomes `DataValue::Null` (Java's `null`).
    ///
    /// For varargs, the last entry of `param_types` is the item type (Java
    /// reads the component type of the trailing array parameter); trailing
    /// arguments are collected into a [`DataValue::Array`].
    /// 对应 Java: com.alibaba.qlexpress4.runtime.data.convert.ParametersTypeConvertor#cast。
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
        let item_type = *param_types
            .last()
            .expect("vararg param types must be non-empty");
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
        let result =
            ParametersTypeConvertor::cast(&args, &[TargetType::Long, TargetType::Int], false);
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
        let args = vec![DataValue::Int(1), DataValue::Int(2), DataValue::Int(3)];
        let result =
            ParametersTypeConvertor::cast(&args, &[TargetType::Long, TargetType::Int], true);
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
        let result =
            ParametersTypeConvertor::cast(&args, &[TargetType::Long, TargetType::Int], true);
        assert_eq!(result, vec![DataValue::Long(1), DataValue::array(vec![])]);
    }
}
