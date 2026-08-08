impl<'a> SerializableParseCacheImporter<'a> {
    /// 对应 Java `required`。
    fn required<'v>(
        &self,
        operands: &'v Map<String, Value>,
        name: &str,
        owner: Option<&SerializableInstruction>,
    ) -> ImportResult<&'v Value> {
        match operands.get(name) {
            None | Some(Value::Null) => {
                Err(self.invalid(owner, &format!("operand '{name}' is required")))
            }
            Some(value) => Ok(value),
        }
    }

    /// 对应 Java `requiredString`。
    fn required_string(
        &self,
        operands: &Map<String, Value>,
        name: &str,
        owner: Option<&SerializableInstruction>,
    ) -> ImportResult<String> {
        self.as_string(
            self.required(operands, name, owner)?,
            owner,
            &format!("operand '{name}'"),
        )
    }

    /// 对应 Java `requiredBoolean`。
    fn required_boolean(
        &self,
        operands: &Map<String, Value>,
        name: &str,
        owner: Option<&SerializableInstruction>,
    ) -> ImportResult<bool> {
        self.as_boolean(
            self.required(operands, name, owner)?,
            owner,
            &format!("operand '{name}'"),
        )
    }

    /// 对应 Java `requiredInt`。
    fn required_int(
        &self,
        operands: &Map<String, Value>,
        name: &str,
        owner: Option<&SerializableInstruction>,
    ) -> ImportResult<i32> {
        self.as_int(
            self.required(operands, name, owner)?,
            owner,
            &format!("operand '{name}'"),
        )
    }

    /// 对应 Java `optionalInt`。
    fn optional_int(
        &self,
        operands: &Map<String, Value>,
        name: &str,
        owner: Option<&SerializableInstruction>,
    ) -> ImportResult<Option<i32>> {
        match operands.get(name) {
            None | Some(Value::Null) => Ok(None),
            Some(value) => Ok(Some(self.as_int(
                value,
                owner,
                &format!("operand '{name}'"),
            )?)),
        }
    }

    /// 对应 Java `requiredList`。
    fn required_list<'v>(
        &self,
        operands: &'v Map<String, Value>,
        name: &str,
        owner: Option<&SerializableInstruction>,
    ) -> ImportResult<&'v Vec<Value>> {
        match self.required(operands, name, owner)? {
            Value::Array(values) => Ok(values),
            _ => Err(self.invalid(owner, &format!("operand '{name}' must be a list"))),
        }
    }

    /// 对应 Java `requiredStringList`。
    fn required_string_list(
        &self,
        operands: &Map<String, Value>,
        name: &str,
        owner: Option<&SerializableInstruction>,
    ) -> ImportResult<Vec<String>> {
        let values = self.required_list(operands, name, owner)?;
        values
            .iter()
            .map(|value| self.as_string(value, owner, &format!("operand '{name}' element")))
            .collect()
    }

    /// 对应 Java `requiredSource`。
    fn required_source(
        &self,
        operands: &Map<String, Value>,
        name: &str,
        owner: Option<&SerializableInstruction>,
    ) -> ImportResult<SerializableSource> {
        let raw = self.required(operands, name, owner)?;
        serde_json::from_value(raw.clone())
            .map_err(|_| self.invalid(owner, &format!("operand '{name}' must be an object")))
    }

    /// 对应 Java `optionalSource`。
    fn optional_source(
        &self,
        operands: &Map<String, Value>,
        name: &str,
        owner: Option<&SerializableInstruction>,
    ) -> ImportResult<Option<SerializableSource>> {
        match operands.get(name) {
            None | Some(Value::Null) => Ok(None),
            Some(raw) => serde_json::from_value(raw.clone())
                .map(Some)
                .map_err(|_| self.invalid(owner, &format!("operand '{name}' must be an object"))),
        }
    }

    /// 对应 Java `asInt`(浮点需为整数值,范围限制在 int 内)。
    fn as_int(
        &self,
        value: &Value,
        owner: Option<&SerializableInstruction>,
        name: &str,
    ) -> ImportResult<i32> {
        let long_value = self.as_long(value, owner, name)?;
        if long_value < i32::MIN as i64 || long_value > i32::MAX as i64 {
            return Err(self.invalid(owner, &format!("{name} must be an int")));
        }
        Ok(long_value as i32)
    }

    /// 对应 Java `asLong`(浮点需为整数值)。
    fn as_long(
        &self,
        value: &Value,
        owner: Option<&SerializableInstruction>,
        name: &str,
    ) -> ImportResult<i64> {
        match value {
            Value::Number(number) => {
                if let Some(v) = number.as_i64() {
                    return Ok(v);
                }
                if let Some(v) = number.as_u64() {
                    return i64::try_from(v)
                        .map_err(|_| self.invalid(owner, &format!("{name} must be a long")));
                }
                // Java: doubleValue != rint(doubleValue) → invalid
                let double_value = number.as_f64().unwrap();
                if double_value != double_value.round() {
                    return Err(self.invalid(owner, &format!("{name} must be a long")));
                }
                Ok(double_value as i64)
            }
            _ => Err(self.invalid(owner, &format!("{name} must be a number"))),
        }
    }

    /// 对应 Java `asNumber(...).doubleValue()`。
    fn as_f64(
        &self,
        value: &Value,
        owner: Option<&SerializableInstruction>,
        name: &str,
    ) -> ImportResult<f64> {
        match value {
            Value::Number(number) => Ok(number.as_f64().unwrap()),
            _ => Err(self.invalid(owner, &format!("{name} must be a number"))),
        }
    }

    /// 对应 Java `asBoolean`。
    fn as_boolean(
        &self,
        value: &Value,
        owner: Option<&SerializableInstruction>,
        name: &str,
    ) -> ImportResult<bool> {
        value
            .as_bool()
            .ok_or_else(|| self.invalid(owner, &format!("{name} must be a boolean")))
    }

    /// 对应 Java `asString`。
    fn as_string(
        &self,
        value: &Value,
        owner: Option<&SerializableInstruction>,
        name: &str,
    ) -> ImportResult<String> {
        match value {
            Value::String(s) => Ok(s.clone()),
            _ => Err(self.invalid(owner, &format!("{name} must be a string"))),
        }
    }

    /// 对应 Java `asDecimalString`(字符串或数字)。
    fn as_decimal_string(
        &self,
        value: &Value,
        owner: Option<&SerializableInstruction>,
        name: &str,
    ) -> ImportResult<String> {
        match value {
            Value::String(s) => Ok(s.clone()),
            Value::Number(n) => Ok(n.to_string()),
            _ => Err(self.invalid(owner, &format!("{name} must be a decimal string"))),
        }
    }

    /// 对应 Java `invalid`。
    fn invalid(
        &self,
        instruction: Option<&SerializableInstruction>,
        detail: &str,
    ) -> SerializableParseCacheException {
        self.model_error(
            instruction.and_then(|inst| inst.source.as_ref()),
            error_codes::SERIALIZABLE_PARSE_CACHE_INVALID_MODEL,
            &error_codes::format_msg(
                error_codes::error_msg(error_codes::SERIALIZABLE_PARSE_CACHE_INVALID_MODEL),
                &[detail.to_string()],
            ),
        )
    }

    /// 对应 Java `unsupportedConstant`。
    fn unsupported_constant(
        &self,
        instruction: Option<&SerializableInstruction>,
        const_type: &str,
    ) -> SerializableParseCacheException {
        SerializableParseCacheException::new(
            Some(&self.script),
            instruction.and_then(|inst| inst.source.as_ref()),
            error_codes::SERIALIZABLE_PARSE_CACHE_UNSUPPORTED_CONSTANT,
            &error_codes::format_msg(
                error_codes::error_msg(error_codes::SERIALIZABLE_PARSE_CACHE_UNSUPPORTED_CONSTANT),
                &[const_type.to_string()],
            ),
        )
    }

    /// 对应 Java `operatorNotFound`。
    fn operator_not_found(
        &self,
        instruction: Option<&SerializableInstruction>,
        operator: &str,
    ) -> SerializableParseCacheException {
        SerializableParseCacheException::new(
            Some(&self.script),
            instruction.and_then(|inst| inst.source.as_ref()),
            error_codes::SERIALIZABLE_PARSE_CACHE_OPERATOR_NOT_FOUND,
            &error_codes::format_msg(
                error_codes::error_msg(error_codes::SERIALIZABLE_PARSE_CACHE_OPERATOR_NOT_FOUND),
                &[operator.to_string()],
            ),
        )
    }

    /// 对应 Java `modelError`。
    fn model_error(
        &self,
        source: Option<&SerializableSource>,
        code: &str,
        reason: &str,
    ) -> SerializableParseCacheException {
        SerializableParseCacheException::new(Some(&self.script), source, code, reason)
    }
}
