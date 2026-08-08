impl<'a> SerializableParseCacheImporter<'a> {
    /// 对应 Java 私有方法 `importForInstruction`。
    fn import_for_instruction(
        &self,
        reporter: Rc<dyn ErrorReporter>,
        operands: &Map<String, Value>,
        instruction: &SerializableInstruction,
    ) -> ImportResult<Instruction> {
        let inst = Some(instruction);
        let for_init = self.optional_lambda(operands, "forInit", inst)?;
        let condition = self.optional_lambda(operands, "condition", inst)?;
        let for_update = self.optional_lambda(operands, "forUpdate", inst)?;
        // Java: conditionSource 存在则以其构造 condition 的 reporter,否则复用本指令 reporter
        let condition_reporter = if operands.contains_key("conditionSource") {
            self.reporter(
                self.optional_source(operands, "conditionSource", inst)?
                    .as_ref(),
            )
        } else {
            Rc::clone(&reporter)
        };
        Ok(Box::new(ForInstruction::new(
            reporter,
            for_init,
            condition,
            condition_reporter,
            for_update,
            self.required_int(operands, "forScopeMaxStackSize", inst)? as usize,
            self.import_lambda_definition(&self.required_lambda(operands, "forBody", inst)?, inst)?,
        )))
    }

    /// 对应 Java 私有方法 `importExceptionTable`。
    fn import_exception_table(
        &self,
        operands: &Map<String, Value>,
        owner: Option<&SerializableInstruction>,
    ) -> ImportResult<Vec<(ClassRef, Rc<dyn QLambdaDefinition>)>> {
        let raw_entries = self.required_list(operands, "exceptionTable", owner)?;
        let mut result = Vec::with_capacity(raw_entries.len());
        for raw_entry in raw_entries {
            let entry: SerializableCatchEntry = serde_json::from_value(raw_entry.clone())
                .map_err(|_| self.invalid(owner, "catch entry must not be null"))?;
            let exception_class_name = entry
                .exception_class_name
                .ok_or_else(|| self.invalid(owner, "catch entry exceptionClassName is required"))?;
            let handler = entry
                .handler
                .ok_or_else(|| self.invalid(owner, "catch entry handler is required"))?;
            result.push((
                self.load_class(&exception_class_name, owner)?,
                self.import_lambda_definition(&handler, owner)?,
            ));
        }
        Ok(result)
    }

    /// 对应 Java 私有方法 `importConstant`(switch 全分支一一对应)。
    fn import_constant(
        &self,
        raw: &Value,
        owner: Option<&SerializableInstruction>,
    ) -> ImportResult<DataValue> {
        let constant: SerializableConstant = serde_json::from_value(raw.clone())
            .map_err(|_| self.invalid(owner, "constant is required"))?;
        let const_type = constant
            .const_type
            .as_deref()
            .ok_or_else(|| self.unsupported_constant(owner, "null"))?;
        let value = constant.value.unwrap_or(Value::Null);
        match const_type {
            "NULL" => Ok(DataValue::Null),
            "BOOLEAN" => Ok(DataValue::Bool(self.as_boolean(
                &value,
                owner,
                "constant.value",
            )?)),
            "STRING" => Ok(DataValue::string(self.as_string(
                &value,
                owner,
                "constant.value",
            )?)),
            "CHAR" => {
                let char_value = self.as_string(&value, owner, "constant.value")?;
                let mut units = char_value.encode_utf16();
                match (units.next(), units.next()) {
                    (Some(unit), None) => Ok(DataValue::Char(unit)),
                    _ => Err(self.invalid(
                        owner,
                        "CHAR constant value must contain exactly one UTF-16 code unit",
                    )),
                }
            }
            "INT" => Ok(DataValue::Int(self.as_int(
                &value,
                owner,
                "constant.value",
            )?)),
            "LONG" => Ok(DataValue::Long(self.as_long(
                &value,
                owner,
                "constant.value",
            )?)),
            "BIG_INTEGER" => {
                let decimal = self.as_decimal_string(&value, owner, "constant.value")?;
                BigInt::parse_bytes(decimal.as_bytes(), 10)
                    .map(DataValue::BigInt)
                    .ok_or_else(|| self.invalid(owner, "constant.value must be a decimal string"))
            }
            "FLOAT" => Ok(DataValue::Float(
                self.as_f64(&value, owner, "constant.value")? as f32,
            )),
            "DOUBLE" => Ok(DataValue::Double(self.as_f64(
                &value,
                owner,
                "constant.value",
            )?)),
            "BIG_DECIMAL" => Ok(DataValue::BigDec(self.as_decimal_string(
                &value,
                owner,
                "constant.value",
            )?)),
            "META_CLASS" => {
                let class_name = self.as_string(&value, owner, "constant.value")?;
                let class_ref = self.load_class(&class_name, owner)?;
                Ok(MetaClass::new(class_ref).into_data_value())
            }
            other => Err(self.unsupported_constant(owner, other)),
        }
    }

    /// 对应 Java 私有方法 `importTracePoints` / `importTracePoint`。
    fn import_trace_points(
        &self,
        raw_trace_points: &[SerializableTracePoint],
        owner: Option<&SerializableInstruction>,
    ) -> ImportResult<Vec<TracePointTree>> {
        let mut result = Vec::with_capacity(raw_trace_points.len());
        for trace_point in raw_trace_points {
            result.push(self.import_trace_point(trace_point, owner)?);
        }
        Ok(result)
    }

    /// 对应 Java 私有方法 `importTracePoint`。
    fn import_trace_point(
        &self,
        trace_point: &SerializableTracePoint,
        owner: Option<&SerializableInstruction>,
    ) -> ImportResult<TracePointTree> {
        let trace_type = trace_type_from_java_name(trace_point.trace_type.as_deref().unwrap_or(""))
            .ok_or_else(|| {
                self.invalid(
                    owner,
                    &format!(
                        "invalid trace point type: {}",
                        trace_point.trace_type.as_deref().unwrap_or("")
                    ),
                )
            })?;
        let children = match &trace_point.children {
            Some(children) => self.import_trace_points(children, owner)?,
            None => Vec::new(),
        };
        Ok(TracePointTree::new(
            trace_type,
            trace_point.token.clone().unwrap_or_default(),
            children,
            trace_point.line,
            trace_point.col,
            trace_point.position,
        ))
    }

    // ---- 以下对应 Java 的一串私有辅助方法 ----

    /// 对应 Java `optionalLambda`。
    fn optional_lambda(
        &self,
        operands: &Map<String, Value>,
        name: &str,
        owner: Option<&SerializableInstruction>,
    ) -> ImportResult<Option<Rc<dyn QLambdaDefinition>>> {
        match operands.get(name) {
            None | Some(Value::Null) => Ok(None),
            Some(raw) => {
                let definition: SerializableLambdaDefinition = serde_json::from_value(raw.clone())
                    .map_err(|_| self.invalid(owner, "lambda definition is required"))?;
                Ok(Some(self.import_lambda_definition(&definition, owner)?))
            }
        }
    }

    /// 对应 Java `required` + `toLambdaDefinition` 组合:取出并反序列化嵌套 Lambda。
    fn required_lambda(
        &self,
        operands: &Map<String, Value>,
        name: &str,
        owner: Option<&SerializableInstruction>,
    ) -> ImportResult<SerializableLambdaDefinition> {
        let raw = self.required(operands, name, owner)?;
        serde_json::from_value(raw.clone())
            .map_err(|_| self.invalid(owner, "lambda definition is required"))
    }

    /// 对应 Java `resultType`。
    fn result_type(
        &self,
        value: &str,
        owner: Option<&SerializableInstruction>,
    ) -> ImportResult<ReturnResultType> {
        match value {
            "RETURN" => Ok(ReturnResultType::Return),
            "BREAK" => Ok(ReturnResultType::Break),
            "CONTINUE" => Ok(ReturnResultType::Continue),
            _ => Err(self.invalid(owner, &format!("invalid resultType: {value}"))),
        }
    }

    /// 对应 Java `breakContinueResult`(BREAK/CONTINUE → is_break)。
    fn break_continue_is_break(
        &self,
        value: &str,
        owner: Option<&SerializableInstruction>,
    ) -> ImportResult<bool> {
        match value {
            "BREAK" => Ok(true),
            "CONTINUE" => Ok(false),
            _ => Err(self.invalid(owner, "BREAK_CONTINUE resultType must be BREAK or CONTINUE")),
        }
    }

    /// 对应 Java `sliceMode`。
    fn slice_mode(
        &self,
        value: &str,
        owner: Option<&SerializableInstruction>,
    ) -> ImportResult<SliceMode> {
        match value {
            "LEFT" => Ok(SliceMode::Left),
            "RIGHT" => Ok(SliceMode::Right),
            "BOTH" => Ok(SliceMode::Both),
            "COPY" => Ok(SliceMode::Copy),
            _ => Err(self.invalid(owner, &format!("invalid slice mode: {value}"))),
        }
    }
}
