//! Method-parameter conversion (incl. varargs), mirroring Java
//! `ParametersTypeConvertor`.

use crate::exception::error_codes;
use crate::exception::ql_exception::QLExceptionKind;
use crate::exception::QLException;
use crate::runtime::class_ref::ClassRef;
use crate::runtime::value::DataValue;

use super::obj_type_convertor::ObjTypeConvertor;

/// 将脚本参数逐项转换为宿主方法声明类型，并负责收集变长参数。
/// 对应 Java: `com.alibaba.qlexpress4.runtime.data.convert.ParametersTypeConvertor`。
pub struct ParametersTypeConvertor;

impl ParametersTypeConvertor {
    /// 按 Java 类型转换规则转换输入值。
    /// 参数：`arguments`、`param_types`、`is_var_arg`；返回：`Result<Vec<DataValue>, QLException>`。
    /// 对应或承接 Java 源文件：`com/alibaba/qlexpress4/runtime/data/convert/ParametersTypeConvertor.java`，方法 `cast`；Rust 侧按所有权与 `Result` 语义适配。
    /// Java `ParametersTypeConvertor.cast(Object[], Class<?>[], boolean)`.
    ///
    /// Like the Java version, convertibility is *not* checked: an
    /// unconvertible argument becomes `DataValue::Null` (Java's `null`).
    ///
    /// For varargs, the last entry of `param_types` is the Java array type;
    /// its component type is used to convert and collect trailing arguments.
    /// 对应 Java: com.alibaba.qlexpress4.runtime.data.convert.ParametersTypeConvertor#cast。
    pub fn cast(
        arguments: &[DataValue],
        param_types: &[ClassRef],
        is_var_arg: bool,
    ) -> Result<Vec<DataValue>, QLException> {
        if !is_var_arg {
            // Java allocates by `arguments.length` and indexes `paramTypes[i]`.
            // Do not use `zip`: it would silently discard an overlong argument
            // list, whereas Java throws ArrayIndexOutOfBoundsException.
            if arguments.len() > param_types.len() {
                return Err(QLException::for_test(
                    QLExceptionKind::Runtime,
                    format!(
                        "argument count {} exceeds declared parameter count {}",
                        arguments.len(),
                        param_types.len()
                    ),
                    error_codes::INVOKE_METHOD_WITH_WRONG_ARGUMENTS,
                ));
            }
            return Ok((0..arguments.len())
                .map(|index| cast_parameter(&arguments[index], &param_types[index]))
                .collect());
        }

        // These direct indexes intentionally preserve Java's invalid-shape
        // failure behavior. Normal runtime callers resolve arity first.
        let var_arg_start = param_types.len().checked_sub(1).ok_or_else(|| {
            QLException::for_test(
                QLExceptionKind::Runtime,
                "Java ParametersTypeConvertor requires a vararg array parameter",
                error_codes::INVOKE_METHOD_WITH_WRONG_ARGUMENTS,
            )
        })?;
        if arguments.len() < var_arg_start {
            return Err(QLException::for_test(
                QLExceptionKind::Runtime,
                format!(
                    "argument count {} is less than fixed parameter count {}",
                    arguments.len(),
                    var_arg_start
                ),
                error_codes::INVOKE_METHOD_WITH_WRONG_ARGUMENTS,
            ));
        }
        let item_type = param_types[var_arg_start]
            .component_type()
            .unwrap_or_else(|| ClassRef::Named("java.lang.Object".to_string()));

        let var_args: Vec<DataValue> = arguments[var_arg_start..]
            .iter()
            .map(|argument| cast_parameter(argument, &item_type))
            .collect();

        let mut result: Vec<DataValue> = (0..var_arg_start)
            .map(|index| cast_parameter(&arguments[index], &param_types[index]))
            .collect();
        result.push(DataValue::array_with_component(var_args, item_type));
        Ok(result)
    }
}

/// 在重载解析已验证引用可赋值性的前提下执行 Java 参数转换。
///
/// 原语、包装类型和 BigNumber 走 `ObjTypeConvertor`；其他引用类型保持
/// 原对象，等价于 Java `type.isInstance(value)` 的 no-need-convert 分支。
fn cast_parameter(argument: &DataValue, parameter_type: &ClassRef) -> DataValue {
    match parameter_type {
        ClassRef::Primitive(target) | ClassRef::Boxed(target) => {
            ObjTypeConvertor::cast(argument, *target).into_converted()
        }
        ClassRef::Named(_) => argument.clone(),
    }
}

#[cfg(test)]
mod tests {
    use super::super::obj_type_convertor::TargetType;
    use super::*;

    #[test]
    fn non_vararg_casts_elementwise() {
        let args = vec![DataValue::Int(1), DataValue::Double(2.5)];
        let result = ParametersTypeConvertor::cast(
            &args,
            &[
                ClassRef::Primitive(TargetType::Long),
                ClassRef::Primitive(TargetType::Int),
            ],
            false,
        )
        .expect("cast should succeed");
        assert_eq!(result, vec![DataValue::Long(1), DataValue::Int(2)]);
    }

    #[test]
    fn unconvertible_becomes_null_like_java() {
        let args = vec![DataValue::Str("x".into())];
        let result =
            ParametersTypeConvertor::cast(&args, &[ClassRef::Primitive(TargetType::Int)], false)
                .expect("cast should succeed");
        assert_eq!(result, vec![DataValue::Null]);
    }

    #[test]
    fn vararg_tail_is_collected_into_array() {
        let args = vec![DataValue::Int(1), DataValue::Int(2), DataValue::Int(3)];
        let result = ParametersTypeConvertor::cast(
            &args,
            &[
                ClassRef::Primitive(TargetType::Long),
                ClassRef::array_of(ClassRef::Primitive(TargetType::Int)),
            ],
            true,
        )
        .expect("cast should succeed");
        assert_eq!(result[0], DataValue::Long(1));
        let DataValue::Array(var_args) = &result[1] else {
            panic!("varargs array expected");
        };
        assert_eq!(
            var_args.borrow().as_slice(),
            &[DataValue::Int(2), DataValue::Int(3)]
        );
        assert_eq!(
            var_args.borrow().component_type(),
            &ClassRef::Primitive(TargetType::Int)
        );
    }

    #[test]
    fn vararg_with_no_tail_arguments() {
        let args = vec![DataValue::Int(1)];
        let result = ParametersTypeConvertor::cast(
            &args,
            &[
                ClassRef::Primitive(TargetType::Long),
                ClassRef::array_of(ClassRef::Primitive(TargetType::Int)),
            ],
            true,
        )
        .expect("cast should succeed");
        assert_eq!(result[0], DataValue::Long(1));
        let DataValue::Array(var_args) = &result[1] else {
            panic!("empty varargs array expected");
        };
        assert!(var_args.borrow().is_empty());
        assert_eq!(
            var_args.borrow().component_type(),
            &ClassRef::Primitive(TargetType::Int)
        );
    }

    #[test]
    fn non_vararg_overflow_returns_error() {
        // Java source parity: ParametersTypeConvertor#cast indexes
        // paramTypes[i] for every argument and throws on this shape.
        let err = ParametersTypeConvertor::cast(
            &[DataValue::Int(1), DataValue::Int(2)],
            &[ClassRef::Primitive(TargetType::Int)],
            false,
        )
        .expect_err("should fail on argument count overflow");
        assert_eq!(
            err.error_code(),
            error_codes::INVOKE_METHOD_WITH_WRONG_ARGUMENTS
        );
    }

    #[test]
    fn vararg_underflow_returns_error() {
        // Java source parity: ParametersTypeConvertor#cast indexes
        // arguments[i] for every fixed parameter before the vararg tail.
        let err = ParametersTypeConvertor::cast(
            &[],
            &[
                ClassRef::Primitive(TargetType::Int),
                ClassRef::array_of(ClassRef::Primitive(TargetType::Int)),
            ],
            true,
        )
        .expect_err("should fail on argument count underflow");
        assert_eq!(
            err.error_code(),
            error_codes::INVOKE_METHOD_WITH_WRONG_ARGUMENTS
        );
    }

    #[test]
    fn vararg_with_empty_param_types_returns_error() {
        // Empty param_types has no vararg array parameter to unpack.
        let err = ParametersTypeConvertor::cast(&[DataValue::Int(1)], &[], true)
            .expect_err("should fail when param_types is empty for vararg");
        assert_eq!(
            err.error_code(),
            error_codes::INVOKE_METHOD_WITH_WRONG_ARGUMENTS
        );
    }
}
