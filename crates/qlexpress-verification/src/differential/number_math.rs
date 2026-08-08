fn execute_number_math(
    id: String,
    invocation: NumberMathInvocation,
) -> Result<DifferentialRecord, String> {
    let left = typed_number_to_data_value(&invocation.left)?;
    let right = invocation
        .right
        .as_ref()
        .map(typed_number_to_data_value)
        .transpose()?;
    let result = if let Some(implementation) = invocation.implementation.as_deref() {
        execute_concrete_number_math(implementation, &invocation.operation, &left, right.as_ref())?
    } else {
        execute_number_math_facade(&id, &invocation.operation, &left, right.as_ref())?
    };
    match result {
        Ok(value) => Ok(DifferentialRecord {
            id,
            outcome: "ok",
            normalized: Some(normalize(&value)),
            error_code: None,
            line: None,
            column: None,
            trace_count: 0,
        }),
        Err(error) => {
            let category = number_math_error_category(&error);
            Ok(DifferentialRecord {
                id,
                outcome: "error",
                normalized: Some(format!("error:{category}:{}", error.reason())),
                error_code: Some(category.to_string()),
                line: Some(0),
                column: Some(0),
                trace_count: 0,
            })
        }
    }
}

fn execute_number_math_facade(
    id: &str,
    operation: &str,
    left: &DataValue,
    right: Option<&DataValue>,
) -> Result<Result<DataValue, qlexpress::exception::QLException>, String> {
    let binary = || right.ok_or_else(|| format!("number_math {operation} requires right operand"));
    let result = match operation {
        "abs" => NumberMath::abs(left),
        "add" => NumberMath::add(left, binary()?),
        "subtract" => NumberMath::subtract(left, binary()?),
        "multiply" => NumberMath::multiply(left, binary()?),
        "divide" => NumberMath::divide(left, binary()?),
        "compareTo" => NumberMath::compare_to(left, binary()?).map(DataValue::Int),
        "or" => NumberMath::or(left, binary()?),
        "and" => NumberMath::and(left, binary()?),
        "xor" => NumberMath::xor(left, binary()?),
        "intDiv" => NumberMath::int_div(left, binary()?),
        "mod" => NumberMath::mod_op(left, binary()?),
        "remainder" => NumberMath::remainder(left, binary()?),
        "leftShift" => NumberMath::left_shift(left, binary()?),
        "rightShift" => NumberMath::right_shift(left, binary()?),
        "rightShiftUnsigned" => NumberMath::right_shift_unsigned(left, binary()?),
        "bitwiseNegate" => NumberMath::bitwise_negate(left),
        "unaryMinus" => NumberMath::unary_minus(left),
        "unaryPlus" => NumberMath::unary_plus(left),
        "toBigDecimal" => NumberMath::to_big_decimal(left),
        "toBigInteger" => NumberMath::to_big_integer(left),
        "isFloatingPoint" => Ok(DataValue::Bool(NumberMath::is_floating_point(left))),
        "isInteger" => Ok(DataValue::Bool(NumberMath::is_integer(left))),
        "isShort" => Ok(DataValue::Bool(NumberMath::is_short(left))),
        "isByte" => Ok(DataValue::Bool(NumberMath::is_byte(left))),
        "isLong" => Ok(DataValue::Bool(NumberMath::is_long(left))),
        "isBigDecimal" => Ok(DataValue::Bool(NumberMath::is_big_decimal(left))),
        "isBigInteger" => Ok(DataValue::Bool(NumberMath::is_big_integer(left))),
        "getMath" => Ok(DataValue::string(math_domain_name(
            convert::math_domain(left, binary()?).ok_or_else(|| {
                format!("number_math getMath received non-number operand in {id}")
            })?,
        ))),
        operation => return Err(format!("unsupported number_math operation: {operation}")),
    };
    Ok(result)
}

fn execute_concrete_number_math(
    implementation: &str,
    operation: &str,
    left: &DataValue,
    right: Option<&DataValue>,
) -> Result<Result<DataValue, qlexpress::exception::QLException>, String> {
    let binary = || right.ok_or_else(|| format!("number_math {operation} requires right operand"));
    let result = match (implementation, operation) {
        ("IntegerMath", "absImpl") => IntegerMath::abs_impl(left),
        ("IntegerMath", "addImpl") => IntegerMath::add_impl(left, binary()?),
        ("IntegerMath", "subtractImpl") => IntegerMath::subtract_impl(left, binary()?),
        ("IntegerMath", "multiplyImpl") => IntegerMath::multiply_impl(left, binary()?),
        ("IntegerMath", "divideImpl") => IntegerMath::divide_impl(left, binary()?),
        ("IntegerMath", "compareToImpl") => Ok(DataValue::Int(IntegerMath::compare_to_impl(
            left,
            binary()?,
        ))),
        ("IntegerMath", "orImpl") => IntegerMath::or_impl(left, binary()?),
        ("IntegerMath", "andImpl") => IntegerMath::and_impl(left, binary()?),
        ("IntegerMath", "xorImpl") => IntegerMath::xor_impl(left, binary()?),
        ("IntegerMath", "intDivImpl") => IntegerMath::int_div_impl(left, binary()?),
        ("IntegerMath", "modImpl") => IntegerMath::mod_impl(left, binary()?),
        ("IntegerMath", "remainderImpl") => IntegerMath::remainder_impl(left, binary()?),
        ("IntegerMath", "unaryMinusImpl") => IntegerMath::unary_minus_impl(left),
        ("IntegerMath", "unaryPlusImpl") => IntegerMath::unary_plus_impl(left),
        ("IntegerMath", "bitwiseNegateImpl") => IntegerMath::bitwise_negate_impl(left),
        ("IntegerMath", "leftShiftImpl") => IntegerMath::left_shift_impl(left, binary()?),
        ("IntegerMath", "rightShiftImpl") => IntegerMath::right_shift_impl(left, binary()?),
        ("IntegerMath", "rightShiftUnsignedImpl") => {
            IntegerMath::right_shift_unsigned_impl(left, binary()?)
        }
        ("LongMath", "absImpl") => LongMath::abs_impl(left),
        ("LongMath", "addImpl") => LongMath::add_impl(left, binary()?),
        ("LongMath", "subtractImpl") => LongMath::subtract_impl(left, binary()?),
        ("LongMath", "multiplyImpl") => LongMath::multiply_impl(left, binary()?),
        ("LongMath", "divideImpl") => LongMath::divide_impl(left, binary()?),
        ("LongMath", "compareToImpl") => {
            Ok(DataValue::Int(LongMath::compare_to_impl(left, binary()?)))
        }
        ("LongMath", "intDivImpl") => LongMath::int_div_impl(left, binary()?),
        ("LongMath", "remainderImpl") => LongMath::remainder_impl(left, binary()?),
        ("LongMath", "modImpl") => LongMath::mod_impl(left, binary()?),
        ("LongMath", "unaryMinusImpl") => LongMath::unary_minus_impl(left),
        ("LongMath", "unaryPlusImpl") => LongMath::unary_plus_impl(left),
        ("LongMath", "bitwiseNegateImpl") => LongMath::bitwise_negate_impl(left),
        ("LongMath", "orImpl") => LongMath::or_impl(left, binary()?),
        ("LongMath", "andImpl") => LongMath::and_impl(left, binary()?),
        ("LongMath", "bitAndImpl") => LongMath::bit_and_impl(left, binary()?),
        ("LongMath", "xorImpl") => LongMath::xor_impl(left, binary()?),
        ("LongMath", "leftShiftImpl") => LongMath::left_shift_impl(left, binary()?),
        ("LongMath", "rightShiftImpl") => LongMath::right_shift_impl(left, binary()?),
        ("LongMath", "rightShiftUnsignedImpl") => {
            LongMath::right_shift_unsigned_impl(left, binary()?)
        }
        ("BigIntegerMath", "absImpl") => BigIntegerMath::abs_impl(left),
        ("BigIntegerMath", "addImpl") => BigIntegerMath::add_impl(left, binary()?),
        ("BigIntegerMath", "subtractImpl") => BigIntegerMath::subtract_impl(left, binary()?),
        ("BigIntegerMath", "multiplyImpl") => BigIntegerMath::multiply_impl(left, binary()?),
        ("BigIntegerMath", "divideImpl") => BigIntegerMath::divide_impl(left, binary()?),
        ("BigIntegerMath", "compareToImpl") => Ok(DataValue::Int(BigIntegerMath::compare_to_impl(
            left,
            binary()?,
        ))),
        ("BigIntegerMath", "intDivImpl") => BigIntegerMath::int_div_impl(left, binary()?),
        ("BigIntegerMath", "modImpl") => BigIntegerMath::mod_impl(left, binary()?),
        ("BigIntegerMath", "remainderImpl") => BigIntegerMath::remainder_impl(left, binary()?),
        ("BigIntegerMath", "unaryMinusImpl") => BigIntegerMath::unary_minus_impl(left),
        ("BigIntegerMath", "unaryPlusImpl") => BigIntegerMath::unary_plus_impl(left),
        ("BigIntegerMath", "bitwiseNegateImpl") => BigIntegerMath::bitwise_negate_impl(left),
        ("BigIntegerMath", "orImpl") => BigIntegerMath::or_impl(left, binary()?),
        ("BigIntegerMath", "andImpl") => BigIntegerMath::and_impl(left, binary()?),
        ("BigIntegerMath", "xorImpl") => BigIntegerMath::xor_impl(left, binary()?),
        ("BigIntegerMath", "leftShiftImpl") => BigIntegerMath::left_shift_impl(left, binary()?),
        ("BigIntegerMath", "rightShiftImpl") => BigIntegerMath::right_shift_impl(left, binary()?),
        ("BigDecimalMath", "absImpl") => BigDecimalMath::abs_impl(left),
        ("BigDecimalMath", "addImpl") => BigDecimalMath::add_impl(left, binary()?),
        ("BigDecimalMath", "subtractImpl") => BigDecimalMath::subtract_impl(left, binary()?),
        ("BigDecimalMath", "multiplyImpl") => BigDecimalMath::multiply_impl(left, binary()?),
        ("BigDecimalMath", "divideImpl") => BigDecimalMath::divide_impl(left, binary()?),
        ("BigDecimalMath", "compareToImpl") => Ok(DataValue::Int(BigDecimalMath::compare_to_impl(
            left,
            binary()?,
        ))),
        ("BigDecimalMath", "unaryMinusImpl") => BigDecimalMath::unary_minus_impl(left),
        ("BigDecimalMath", "unaryPlusImpl") => BigDecimalMath::unary_plus_impl(left),
        ("BigDecimalMath", "remainderImpl") => BigDecimalMath::remainder_impl(left, binary()?),
        ("BigDecimalMath", "modImpl") => BigDecimalMath::mod_impl(left, binary()?),
        ("FloatingPointMath", "absImpl") => FloatingPointMath::abs_impl(left),
        ("FloatingPointMath", "addImpl") => FloatingPointMath::add_impl(left, binary()?),
        ("FloatingPointMath", "subtractImpl") => FloatingPointMath::subtract_impl(left, binary()?),
        ("FloatingPointMath", "multiplyImpl") => FloatingPointMath::multiply_impl(left, binary()?),
        ("FloatingPointMath", "divideImpl") => FloatingPointMath::divide_impl(left, binary()?),
        ("FloatingPointMath", "compareToImpl") => Ok(DataValue::Int(
            FloatingPointMath::compare_to_impl(left, binary()?),
        )),
        ("FloatingPointMath", "remainderImpl") => {
            FloatingPointMath::remainder_impl(left, binary()?)
        }
        ("FloatingPointMath", "modImpl") => FloatingPointMath::mod_impl(left, binary()?),
        ("FloatingPointMath", "unaryMinusImpl") => FloatingPointMath::unary_minus_impl(left),
        ("FloatingPointMath", "unaryPlusImpl") => FloatingPointMath::unary_plus_impl(left),
        _ => {
            return Err(format!(
                "unsupported concrete number_math operation: {implementation}.{operation}"
            ));
        }
    };
    Ok(result)
}

fn typed_number_to_data_value(number: &TypedNumber) -> Result<DataValue, String> {
    let parse_error = |error: &dyn std::fmt::Display| {
        format!(
            "invalid {} number value {:?}: {error}",
            number.number_type, number.value
        )
    };
    match number.number_type.as_str() {
        "byte" => number
            .value
            .parse::<i8>()
            .map(DataValue::Byte)
            .map_err(|error| parse_error(&error)),
        "short" => number
            .value
            .parse::<i16>()
            .map(DataValue::Short)
            .map_err(|error| parse_error(&error)),
        "int" => number
            .value
            .parse::<i32>()
            .map(DataValue::Int)
            .map_err(|error| parse_error(&error)),
        "long" => number
            .value
            .parse::<i64>()
            .map(DataValue::Long)
            .map_err(|error| parse_error(&error)),
        "float" => number
            .value
            .parse::<f32>()
            .map(DataValue::Float)
            .map_err(|error| parse_error(&error)),
        "double" => number
            .value
            .parse::<f64>()
            .map(DataValue::Double)
            .map_err(|error| parse_error(&error)),
        "bigint" => number
            .value
            .parse::<num_bigint::BigInt>()
            .map(DataValue::BigInt)
            .map_err(|error| parse_error(&error)),
        "bigdec" => Ok(DataValue::BigDec(number.value.clone())),
        number_type => Err(format!("unsupported number type: {number_type}")),
    }
}

fn math_domain_name(domain: MathDomain) -> &'static str {
    match domain {
        MathDomain::Integer => "IntegerMath",
        MathDomain::Long => "LongMath",
        MathDomain::FloatingPoint => "FloatingPointMath",
        MathDomain::BigInteger => "BigIntegerMath",
        MathDomain::BigDecimal => "BigDecimalMath",
    }
}

fn number_math_error_category(error: &qlexpress::exception::QLException) -> &'static str {
    if error.error_code() == "java.lang.NumberFormatException" {
        "NUMBER_FORMAT_EXCEPTION"
    } else if error.reason().starts_with("Cannot use")
        || error
            .reason()
            .starts_with("Shift distance must be an integral type")
    {
        "UNSUPPORTED_OPERATION"
    } else {
        "ARITHMETIC_EXCEPTION"
    }
}

fn build_options(options: &DifferentialOptions) -> QLOptions {
    let mut builder: QLOptionsBuilder = QLOptions::builder();
    if let Some(value) = options.precise {
        builder = builder.precise(value);
    }
    if let Some(value) = options.cache {
        builder = builder.cache(value);
    }
    if let Some(value) = options.avoid_null_pointer {
        builder = builder.avoid_null_pointer(value);
    }
    if let Some(value) = options.max_arr_length {
        builder = builder.max_arr_length(value);
    }
    if let Some(value) = options.trace_expression {
        builder = builder.trace_expression(value);
    }
    if let Some(value) = options.short_circuit_disable {
        builder = builder.short_circuit_disable(value);
    }
    if let Some(value) = options.timeout_millis {
        builder = builder.timeout_millis(value);
    }
    builder.build()
}

fn json_to_data_value(value: serde_json::Value) -> Result<DataValue, String> {
    match value {
        serde_json::Value::Null => Ok(DataValue::Null),
        serde_json::Value::Bool(value) => Ok(DataValue::Bool(value)),
        serde_json::Value::Number(value) => {
            if let Some(value) = value.as_i64() {
                if let Ok(value) = i32::try_from(value) {
                    Ok(DataValue::Int(value))
                } else {
                    Ok(DataValue::Long(value))
                }
            } else if let Some(value) = value.as_f64() {
                Ok(DataValue::Double(value))
            } else {
                Err(format!("unsupported JSON number: {value}"))
            }
        }
        serde_json::Value::String(value) => Ok(DataValue::string(value)),
        serde_json::Value::Array(values) => values
            .into_iter()
            .map(json_to_data_value)
            .collect::<Result<Vec<_>, _>>()
            .map(DataValue::list),
        serde_json::Value::Object(values) => {
            let entries = values
                .into_iter()
                .map(|(key, value)| {
                    json_to_data_value(value).map(|value| (DataValue::string(key), value))
                })
                .collect::<Result<Vec<_>, _>>()?;
            Ok(DataValue::map(IndexMap::from_entries(entries)))
        }
    }
}
