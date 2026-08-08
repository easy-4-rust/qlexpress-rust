impl<'a> SerializableParseCacheImporter<'a> {
    /// 构造导入器。对应 Java 构造器
    /// `SerializableParseCacheImporter(OperatorManager, ClassSupplier)`。
    pub fn new(
        operator_manager: &'a dyn OperatorFactory,
        class_supplier: &'a dyn ClassSupplier,
    ) -> Self {
        SerializableParseCacheImporter {
            operator_manager,
            class_supplier,
            script: String::new(),
        }
    }

    /// 加载编译缓存。对应 Java 方法 `load(SerializableParseCache, Object)`。
    pub fn load(
        &mut self,
        cache: &SerializableParseCache,
        runner_identity: usize,
    ) -> ImportResult<LoadedParseCache> {
        self.script = cache.script.clone().unwrap_or_default();
        // Java: modelVersion 校验
        if cache.model_version != MODEL_VERSION {
            return Err(self.model_error(
                None,
                error_codes::SERIALIZABLE_PARSE_CACHE_UNSUPPORTED_VERSION,
                &error_codes::format_msg(
                    error_codes::error_msg(
                        error_codes::SERIALIZABLE_PARSE_CACHE_UNSUPPORTED_VERSION,
                    ),
                    &[cache.model_version.to_string()],
                ),
            ));
        }
        // Java: script is required / main lambda is required
        if cache.script.is_none() {
            return Err(self.model_error(
                None,
                error_codes::SERIALIZABLE_PARSE_CACHE_INVALID_MODEL,
                &error_codes::format_msg(
                    error_codes::error_msg(error_codes::SERIALIZABLE_PARSE_CACHE_INVALID_MODEL),
                    &["script is required".to_string()],
                ),
            ));
        }
        let main_def = match &cache.main {
            Some(main) => main,
            None => {
                return Err(self.model_error(
                    None,
                    error_codes::SERIALIZABLE_PARSE_CACHE_INVALID_MODEL,
                    &error_codes::format_msg(
                        error_codes::error_msg(error_codes::SERIALIZABLE_PARSE_CACHE_INVALID_MODEL),
                        &["main lambda is required".to_string()],
                    ),
                ))
            }
        };
        let main = self.import_lambda_definition(main_def, None)?;
        let trace_points = match &cache.trace_points {
            Some(trace_points) => self.import_trace_points(trace_points, None)?,
            None => Vec::new(),
        };
        Ok(LoadedParseCache::new(
            QCompileCache::new(main, trace_points),
            cache.clone(),
            runner_identity,
        ))
    }

    /// 对应 Java 私有方法 `importLambdaDefinition`(含全部必填校验)。
    fn import_lambda_definition(
        &self,
        definition: &SerializableLambdaDefinition,
        owner: Option<&SerializableInstruction>,
    ) -> ImportResult<Rc<dyn QLambdaDefinition>> {
        let name = definition
            .name
            .clone()
            .ok_or_else(|| self.invalid(owner, "lambda name is required"))?;
        let raw_instructions = definition
            .instructions
            .as_ref()
            .ok_or_else(|| self.invalid(owner, "lambda instructions are required"))?;
        let raw_params = definition
            .params
            .as_ref()
            .ok_or_else(|| self.invalid(owner, "lambda params are required"))?;
        if definition.max_stack_size < 0 {
            return Err(self.invalid(owner, "lambda maxStackSize must not be negative"));
        }
        let mut params = Vec::with_capacity(raw_params.len());
        for param in raw_params {
            params.push(self.import_param(param, owner)?);
        }
        let mut instructions = Vec::with_capacity(raw_instructions.len());
        for instruction in raw_instructions {
            instructions.push(self.import_instruction(instruction, owner)?);
        }
        Ok(Rc::new(QLambdaDefinitionInner::new(
            name,
            instructions,
            params,
            definition.max_stack_size as usize,
        )))
    }

    /// 对应 Java 私有方法 `importParam`。
    fn import_param(
        &self,
        param: &SerializableParam,
        owner: Option<&SerializableInstruction>,
    ) -> ImportResult<Param> {
        let name = param
            .name
            .clone()
            .ok_or_else(|| self.invalid(owner, "lambda param name is required"))?;
        let class_name = param
            .class_name
            .clone()
            .ok_or_else(|| self.invalid(owner, "lambda param className is required"))?;
        let class_ref = self.load_class(&class_name, owner)?;
        Ok(Param::new(name, Some(class_ref)))
    }

    /// 对应 Java 私有方法 `importInstruction` 的 opcode 分派
    /// (switch 全分支一一对应)。
    fn import_instruction(
        &self,
        instruction: &SerializableInstruction,
        parent: Option<&SerializableInstruction>,
    ) -> ImportResult<Instruction> {
        let opcode = instruction
            .opcode
            .as_deref()
            .ok_or_else(|| self.invalid(Some(instruction), "opcode is required"))?;
        let operands = instruction
            .operands
            .as_ref()
            .ok_or_else(|| self.invalid(Some(instruction), "operands are required"))?;
        let reporter = self.reporter(instruction.source.as_ref());
        let inst = Some(instruction);
        let instruction_boxed: Instruction = match opcode {
            "CONST" => Box::new(ConstInstruction::new(
                Rc::clone(&reporter),
                self.import_constant(self.required(operands, "constant", inst)?, inst)?,
                self.optional_int(operands, "traceKey", inst)?,
            )),
            "LOAD" => Box::new(LoadInstruction::new(
                Rc::clone(&reporter),
                self.required_string(operands, "name", inst)?,
                self.optional_int(operands, "traceKey", inst)?,
            )),
            "POP" => Box::new(PopInstruction::new(Rc::clone(&reporter))),
            "RETURN" => Box::new(ReturnInstruction::new(
                Rc::clone(&reporter),
                self.result_type(&self.required_string(operands, "resultType", inst)?, inst)?,
                self.optional_int(operands, "traceKey", inst)?,
            )),
            "BREAK_CONTINUE" => Box::new(BreakContinueInstruction::new(
                Rc::clone(&reporter),
                self.break_continue_is_break(
                    &self.required_string(operands, "resultType", inst)?,
                    inst,
                )?,
            )),
            "THROW" => Box::new(ThrowInstruction::new(Rc::clone(&reporter))),
            "CHECK_TIMEOUT" => Box::new(CheckTimeOutInstruction::new(Rc::clone(&reporter))),
            "JUMP" => Box::new(JumpInstruction::new(
                Rc::clone(&reporter),
                self.required_int(operands, "position", inst)?,
            )),
            "JUMP_IF" => Box::new(JumpIfInstruction::new(
                Rc::clone(&reporter),
                self.required_boolean(operands, "expect", inst)?,
                self.required_int(operands, "position", inst)?,
                self.optional_int(operands, "traceKey", inst)?,
            )),
            "JUMP_IF_POP" => Box::new(JumpIfPopInstruction::new(
                Rc::clone(&reporter),
                self.required_boolean(operands, "expect", inst)?,
                self.required_int(operands, "position", inst)?,
            )),
            "BINARY_OP" => Box::new(OperatorInstruction::new(
                Rc::clone(&reporter),
                self.binary_operator(&self.required_string(operands, "operator", inst)?, inst)?,
                self.optional_int(operands, "traceKey", inst)?,
            )),
            "PREFIX_UNARY_OP" => Box::new(UnaryInstruction::new(
                Rc::clone(&reporter),
                self.prefix_unary_operator(
                    &self.required_string(operands, "operator", inst)?,
                    inst,
                )?,
                self.optional_int(operands, "traceKey", inst)?,
            )),
            "SUFFIX_UNARY_OP" => Box::new(UnaryInstruction::new(
                Rc::clone(&reporter),
                self.suffix_unary_operator(
                    &self.required_string(operands, "operator", inst)?,
                    inst,
                )?,
                self.optional_int(operands, "traceKey", inst)?,
            )),
            "CALL_FUNCTION" => Box::new(CallFunctionInstruction::new(
                Rc::clone(&reporter),
                self.required_string(operands, "functionName", inst)?,
                self.required_int(operands, "argNum", inst)? as usize,
                self.optional_int(operands, "traceKey", inst)?,
            )),
            "CALL" => Box::new(CallInstruction::new(
                Rc::clone(&reporter),
                self.required_int(operands, "argNum", inst)? as usize,
            )),
            "LOAD_LAMBDA" => Box::new(LoadLambdaInstruction::new(
                Rc::clone(&reporter),
                self.import_lambda_definition(
                    &self.required_lambda(operands, "lambda", inst)?,
                    inst,
                )?,
            )),
            "DEFINE_FUNCTION" => Box::new(DefineFunctionInstruction::new(
                Rc::clone(&reporter),
                self.required_string(operands, "name", inst)?,
                self.import_lambda_definition(
                    &self.required_lambda(operands, "lambda", inst)?,
                    inst,
                )?,
            )),
            "NEW_SCOPE" => Box::new(NewScopeInstruction::new(
                Rc::clone(&reporter),
                self.required_string(operands, "scopeName", inst)?,
            )),
            "CLOSE_SCOPE" => Box::new(CloseScopeInstruction::new(
                Rc::clone(&reporter),
                self.required_string(operands, "scopeName", inst)?,
            )),
            "DEFINE_LOCAL" => Box::new(DefineLocalInstruction::new(
                Rc::clone(&reporter),
                self.required_string(operands, "variableName", inst)?,
                Some(self.load_class(&self.required_string(operands, "className", inst)?, inst)?),
            )),
            "NEW_INSTANCE" => Box::new(NewInstanceInstruction::new(
                Rc::clone(&reporter),
                self.load_class(&self.required_string(operands, "className", inst)?, inst)?,
                self.required_int(operands, "argNum", inst)? as usize,
            )),
            "NEW_FILLED_INSTANCE" => Box::new(NewFilledInstanceInstruction::new(
                Rc::clone(&reporter),
                self.load_class(&self.required_string(operands, "className", inst)?, inst)?,
                self.required_string_list(operands, "keys", inst)?,
            )),
            "NEW_ARRAY" => Box::new(NewArrayInstruction::new(
                Rc::clone(&reporter),
                self.load_class(
                    &self.required_string(operands, "componentClassName", inst)?,
                    inst,
                )?,
                self.required_int(operands, "length", inst)? as usize,
            )),
            "MULTI_NEW_ARRAY" => Box::new(MultiNewArrayInstruction::new(
                Rc::clone(&reporter),
                self.load_class(
                    &self.required_string(operands, "componentClassName", inst)?,
                    inst,
                )?,
                self.required_int(operands, "dims", inst)? as usize,
            )),
            "NEW_LIST" => Box::new(NewListInstruction::new(
                Rc::clone(&reporter),
                self.required_int(operands, "initLength", inst)? as usize,
            )),
            "NEW_MAP" => Box::new(NewMapInstruction::new(
                Rc::clone(&reporter),
                self.required_string_list(operands, "keys", inst)?,
            )),
            "GET_FIELD" => Box::new(GetFieldInstruction::new(
                Rc::clone(&reporter),
                self.required_string(operands, "fieldName", inst)?,
                self.required_boolean(operands, "optional", inst)?,
            )),
            "SPREAD_GET_FIELD" => Box::new(SpreadGetFieldInstruction::new(
                Rc::clone(&reporter),
                self.required_string(operands, "fieldName", inst)?,
            )),
            "METHOD_INVOKE" => Box::new(MethodInvokeInstruction::new(
                Rc::clone(&reporter),
                self.required_string(operands, "methodName", inst)?,
                self.required_int(operands, "argNum", inst)? as usize,
                self.required_boolean(operands, "optional", inst)?,
            )),
            "SPREAD_METHOD_INVOKE" => Box::new(SpreadMethodInvokeInstruction::new(
                Rc::clone(&reporter),
                self.required_string(operands, "methodName", inst)?,
                self.required_int(operands, "argNum", inst)? as usize,
            )),
            "GET_METHOD" => Box::new(GetMethodInstruction::new(
                Rc::clone(&reporter),
                self.required_string(operands, "methodName", inst)?,
            )),
            "INDEX" => Box::new(IndexInstruction::new(Rc::clone(&reporter))),
            "SLICE" => Box::new(SliceInstruction::new(
                Rc::clone(&reporter),
                self.slice_mode(&self.required_string(operands, "mode", inst)?, inst)?,
            )),
            "CAST" => Box::new(CastInstruction::new(Rc::clone(&reporter))),
            "WHILE" => Box::new(WhileInstruction::new(
                Rc::clone(&reporter),
                self.import_lambda_definition(
                    &self.required_lambda(operands, "condition", inst)?,
                    inst,
                )?,
                self.import_lambda_definition(
                    &self.required_lambda(operands, "body", inst)?,
                    inst,
                )?,
                self.required_int(operands, "whileScopeMaxStackSize", inst)? as usize,
            )),
            "FOR" => self.import_for_instruction(reporter, operands, instruction)?,
            "FOR_EACH" => Box::new(ForEachInstruction::new(
                Rc::clone(&reporter),
                self.import_lambda_definition(
                    &self.required_lambda(operands, "body", inst)?,
                    inst,
                )?,
                self.load_class(
                    &self.required_string(operands, "itemClassName", inst)?,
                    inst,
                )?,
                self.reporter(Some(&self.required_source(
                    operands,
                    "targetSource",
                    inst,
                )?)),
            )),
            "TRY_CATCH" => Box::new(TryCatchInstruction::new(
                Rc::clone(&reporter),
                self.import_lambda_definition(
                    &self.required_lambda(operands, "body", inst)?,
                    inst,
                )?,
                self.import_exception_table(operands, inst)?,
                self.optional_lambda(operands, "finalBody", inst)?,
            )),
            "TRACE_PEEK" => Box::new(TracePeekInstruction::new(
                Rc::clone(&reporter),
                self.optional_int(operands, "traceKey", inst)?,
            )),
            "TRACE_EVALUATED" => Box::new(TraceEvaluatedInstruction::new(
                Rc::clone(&reporter),
                self.optional_int(operands, "traceKey", inst)?,
            )),
            "STRING_JOIN" => Box::new(StringJoinInstruction::new(
                Rc::clone(&reporter),
                self.required_int(operands, "n", inst)? as usize,
            )),
            // Java default 分支: UNSUPPORTED_INSTRUCTION
            other => {
                return Err(SerializableParseCacheException::new(
                    Some(&self.script),
                    instruction.source.as_ref(),
                    error_codes::SERIALIZABLE_PARSE_CACHE_UNSUPPORTED_INSTRUCTION,
                    &error_codes::format_msg(
                        error_codes::error_msg(
                            error_codes::SERIALIZABLE_PARSE_CACHE_UNSUPPORTED_INSTRUCTION,
                        ),
                        &[other.to_string()],
                    ),
                ))
            }
        };
        let _ = parent;
        Ok(instruction_boxed)
    }
}
