//! 编译缓存导入器,对应 Java `com.alibaba.qlexpress4.api.parsecache.SerializableParseCacheImporter`。
//! 职责:把 [`SerializableParseCache`](可来自 JSON 反序列化)还原为可执行的
//! 编译产物(指令序列 + Lambda 定义 + trace 点)。

use std::rc::Rc;

use num_bigint::BigInt;
use serde_json::{Map, Value};

use crate::aparser::compile_cache::QCompileCache;
use crate::aparser::operator_factory::OperatorFactory;
use crate::class_supplier::ClassSupplier;
use crate::exception::default_err_reporter::DefaultErrReporter;
use crate::exception::error_codes;
use crate::exception::error_reporter::ErrorReporter;
use crate::runtime::instruction::{
    BreakContinueInstruction, CallFunctionInstruction, CallInstruction, CastInstruction,
    CheckTimeOutInstruction, CloseScopeInstruction, ConstInstruction, DefineFunctionInstruction,
    DefineLocalInstruction, ForEachInstruction, ForInstruction, GetFieldInstruction,
    GetMethodInstruction, IndexInstruction, Instruction, JumpIfInstruction, JumpIfPopInstruction,
    JumpInstruction, LoadInstruction, LoadLambdaInstruction, MethodInvokeInstruction,
    MultiNewArrayInstruction, NewArrayInstruction, NewFilledInstanceInstruction,
    NewInstanceInstruction, NewListInstruction, NewMapInstruction, NewScopeInstruction,
    OperatorInstruction, PopInstruction, ReturnInstruction, ReturnResultType, SliceInstruction,
    SliceMode, SpreadGetFieldInstruction, SpreadMethodInvokeInstruction, StringJoinInstruction,
    ThrowInstruction, TraceEvaluatedInstruction, TracePeekInstruction, TryCatchInstruction,
    UnaryInstruction, WhileInstruction,
};
use crate::runtime::member::{ClassRef, MetaClass};
use crate::runtime::qlambda_definition::QLambdaDefinition;
use crate::runtime::qlambda_definition_inner::{Param, QLambdaDefinitionInner};
use crate::runtime::trace::{TracePointTree, TraceType};
use crate::runtime::value::DataValue;

use super::loaded_parse_cache::LoadedParseCache;
use super::serializable_catch_entry::SerializableCatchEntry;
use super::serializable_constant::SerializableConstant;
use super::serializable_instruction::SerializableInstruction;
use super::serializable_lambda_definition::SerializableLambdaDefinition;
use super::serializable_param::SerializableParam;
use super::serializable_parse_cache::SerializableParseCache;
use super::serializable_parse_cache_exception::SerializableParseCacheException;
use super::serializable_parse_cache_exporter::MODEL_VERSION;
use super::serializable_source::SerializableSource;
use super::serializable_trace_point::SerializableTracePoint;

/// 导入结果(失败即 [`SerializableParseCacheException`],对应 Java 抛异常)。
pub type ImportResult<T> = Result<T, SerializableParseCacheException>;

/// 编译缓存导入器。对应 Java: com.alibaba.qlexpress4.api.parsecache.SerializableParseCacheImporter
///
/// 偏差说明:
/// - Java `load(null)` 抛错;Rust 以引用接收,无 null 入参(其余校验一致);
/// - Java `runnerIdentity` 为任意 `Object`(引用相等);Rust 为 `usize`
///   身份令牌(见 [`LoadedParseCache`]);
/// - Java 类对象由 Rust [`ClassRef`] 保存规范类名与原语目标。
pub struct SerializableParseCacheImporter<'a> {
    /// 操作符管理器。对应 Java 字段 `operatorManager`。
    operator_manager: &'a dyn OperatorFactory,
    /// 类型供应器。对应 Java 字段 `classSupplier`。
    class_supplier: &'a dyn ClassSupplier,
    /// 当前脚本(load 时设置)。对应 Java 字段 `script`。
    script: String,
}

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
                Some(self.load_class(
                    &self.required_string(operands, "className", inst)?,
                    inst,
                )?),
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
            "STRING" => Ok(DataValue::Str(self.as_string(
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

    /// 对应 Java `binaryOperator`。
    fn binary_operator(
        &self,
        operator: &str,
        owner: Option<&SerializableInstruction>,
    ) -> ImportResult<Rc<dyn crate::runtime::operator::BinaryOperator>> {
        self.operator_manager
            .get_binary_operator(operator)
            .ok_or_else(|| self.operator_not_found(owner, operator))
    }

    /// 对应 Java `prefixUnaryOperator`。
    fn prefix_unary_operator(
        &self,
        operator: &str,
        owner: Option<&SerializableInstruction>,
    ) -> ImportResult<Rc<dyn crate::runtime::operator::UnaryOperator>> {
        self.operator_manager
            .get_prefix_unary_operator(operator)
            .ok_or_else(|| self.operator_not_found(owner, operator))
    }

    /// 对应 Java `suffixUnaryOperator`。
    fn suffix_unary_operator(
        &self,
        operator: &str,
        owner: Option<&SerializableInstruction>,
    ) -> ImportResult<Rc<dyn crate::runtime::operator::UnaryOperator>> {
        self.operator_manager
            .get_suffix_unary_operator(operator)
            .ok_or_else(|| self.operator_not_found(owner, operator))
    }

    /// 对应 Java `loadClass` + `primitiveClass`:原始类型名直接命中,
    /// 其余委托 [`ClassSupplier`](找不到即 `CLASS_NOT_FOUND`)。
    fn load_class(
        &self,
        class_name: &str,
        owner: Option<&SerializableInstruction>,
    ) -> ImportResult<ClassRef> {
        if let Some(component_name) = class_name.strip_suffix("[]") {
            let component = self.load_class(component_name, owner)?;
            return Ok(ClassRef::Named(format!("{}[]", component.java_name())));
        }
        // Java primitiveClass:boolean/byte/char/short/int/long/float/double/void
        match class_name {
            "boolean" | "byte" | "char" | "short" | "int" | "long" | "float" | "double" => {
                return Ok(ClassRef::from_name(class_name));
            }
            "void" => return Ok(ClassRef::Named("void".to_string())),
            _ => {}
        }
        // Rust 补充:Java 包装类名同样可确定为转换目标(ClassRef::from_name)
        if let ClassRef::Primitive(_) = ClassRef::from_name(class_name) {
            return Ok(ClassRef::from_name(class_name));
        }
        // `java.lang.Object`:Java 的 Class.forName 恒可加载(Rust 无 classpath,
        // 编译器自身会为无类型参数/局部变量导出此名,故内建放行)
        if class_name == "java.lang.Object" {
            return Ok(ClassRef::Named(class_name.to_string()));
        }
        match self.class_supplier.load_cls(class_name) {
            Some(canonical) => Ok(ClassRef::from_name(&canonical)),
            None => Err(SerializableParseCacheException::new(
                Some(&self.script),
                owner.and_then(|inst| inst.source.as_ref()),
                error_codes::SERIALIZABLE_PARSE_CACHE_CLASS_NOT_FOUND,
                &error_codes::format_msg(
                    error_codes::error_msg(error_codes::SERIALIZABLE_PARSE_CACHE_CLASS_NOT_FOUND),
                    &[class_name.to_string()],
                ),
            )),
        }
    }

    /// 对应 Java `reporter(SerializableSource)`:
    /// line <= 0 归一为 1;col 取 max(0, col) + 1(转回 1 基)。
    fn reporter(&self, source: Option<&SerializableSource>) -> Rc<dyn ErrorReporter> {
        let default_source = SerializableSource::default();
        let normalized = source.unwrap_or(&default_source);
        let line = if normalized.line <= 0 {
            1
        } else {
            normalized.line
        };
        let col = normalized.col.max(0) + 1;
        Rc::new(DefaultErrReporter::new(
            self.script.clone(),
            normalized.start.max(0),
            line,
            col,
            normalized.lexeme.clone().unwrap_or_default(),
        ))
    }

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

/// Java `TraceType.valueOf(String)` 的 Rust 对应(大写名 → 枚举)。
fn trace_type_from_java_name(name: &str) -> Option<TraceType> {
    match name {
        "OPERATOR" => Some(TraceType::Operator),
        "FUNCTION" => Some(TraceType::Function),
        "METHOD" => Some(TraceType::Method),
        "FIELD" => Some(TraceType::Field),
        "LIST" => Some(TraceType::List),
        "MAP" => Some(TraceType::Map),
        "IF" => Some(TraceType::If),
        "SWITCH" => Some(TraceType::Switch),
        "RETURN" => Some(TraceType::Return),
        "BLOCK" => Some(TraceType::Block),
        "VARIABLE" => Some(TraceType::Variable),
        "VALUE" => Some(TraceType::Value),
        "DEFINE_FUNCTION" => Some(TraceType::DefineFunction),
        "DEFINE_MACRO" => Some(TraceType::DefineMacro),
        "PRIMARY" => Some(TraceType::Primary),
        "STATEMENT" => Some(TraceType::Statement),
        _ => None,
    }
}
