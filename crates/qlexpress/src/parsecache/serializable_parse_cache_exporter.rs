//! 编译缓存导出器,对应 Java `com.alibaba.qlexpress4.api.parsecache.SerializableParseCacheExporter`。
//! 职责:把编译产物(主 Lambda 定义 + 指令序列 + 常量 + trace 点)转为
//! 可 JSON 序列化的 [`SerializableParseCache`]。

use std::rc::Rc;

use serde_json::{Map, Value};

use crate::aparser::operator_factory::OperatorFactory;
use crate::exception::default_err_reporter::DefaultErrReporter;
use crate::exception::error_codes;
use crate::exception::error_reporter::ErrorReporter;
use crate::runtime::instruction::{
    BreakContinueInstruction, CallConstInstruction, CallFunctionInstruction, CallInstruction,
    CastInstruction, CheckTimeOutInstruction, CloseScopeInstruction, ConstInstruction,
    DefineFunctionInstruction, DefineLocalInstruction, ForEachInstruction, ForInstruction,
    GetFieldInstruction, GetMethodInstruction, IndexInstruction, Instruction, JumpIfInstruction,
    JumpIfPopInstruction, JumpInstruction, LoadInstruction, LoadLambdaInstruction,
    MethodInvokeInstruction, MultiNewArrayInstruction, NewArrayInstruction,
    NewFilledInstanceInstruction, NewInstanceInstruction, NewListInstruction, NewMapInstruction,
    NewScopeInstruction, OperatorInstruction, PopInstruction, ReturnInstruction, ReturnResultType,
    SliceInstruction, SliceMode, SpreadGetFieldInstruction, SpreadMethodInvokeInstruction,
    StringJoinInstruction, ThrowInstruction, TraceEvaluatedInstruction, TracePeekInstruction,
    TryCatchInstruction, UnaryInstruction, WhileInstruction,
};
use crate::runtime::member::{as_meta_class, ClassRef};
use crate::runtime::qlambda_definition::QLambdaDefinition;
use crate::runtime::qlambda_definition_inner::QLambdaDefinitionInner;
use crate::runtime::trace::trace_type;
use crate::runtime::trace::TracePointTree;
use crate::runtime::value::DataValue;

use super::loaded_parse_cache::LoadedCompileCache;
use super::serializable_catch_entry::SerializableCatchEntry;
use super::serializable_constant::SerializableConstant;
use super::serializable_instruction::SerializableInstruction;
use super::serializable_lambda_definition::SerializableLambdaDefinition;
use super::serializable_param::SerializableParam;
use super::serializable_parse_cache::SerializableParseCache;
use super::serializable_parse_cache_exception::SerializableParseCacheException;
use super::serializable_source::SerializableSource;
use super::serializable_trace_point::SerializableTracePoint;

/// 模型版本。对应 Java `SerializableParseCacheExporter.MODEL_VERSION`。
pub const MODEL_VERSION: i32 = 1;

/// 导出结果(失败即 [`SerializableParseCacheException`],对应 Java 抛异常)。
pub type ExportResult<T> = Result<T, SerializableParseCacheException>;

/// 编译缓存导出器。对应 Java: com.alibaba.qlexpress4.api.parsecache.SerializableParseCacheExporter
///
/// `operator_manager` 承担 Java 构造器中的 `OperatorManager`(用于一元
/// 操作符的前/后缀判定);Rust 通过 [`OperatorFactory`] trait 访问。
pub struct SerializableParseCacheExporter<'a> {
    /// 脚本原文。对应 Java 字段 `script`。
    script: String,
    /// 操作符管理器。对应 Java 字段 `operatorManager`。
    operator_manager: &'a dyn OperatorFactory,
    /// 是否导出 trace 点。对应 Java 字段 `includeTracePoints`。
    include_trace_points: bool,
}

impl<'a> SerializableParseCacheExporter<'a> {
    /// 构造导出器。对应 Java 构造器
    /// `SerializableParseCacheExporter(String, OperatorManager, boolean)`。
    pub fn new(
        script: impl Into<String>,
        operator_manager: &'a dyn OperatorFactory,
        include_trace_points: bool,
    ) -> Self {
        SerializableParseCacheExporter {
            script: script.into(),
            operator_manager,
            include_trace_points,
        }
    }

    /// 导出编译缓存。对应 Java 方法 `export(QCompileCache)`。
    ///
    /// producerVersion 说明:Java 取包实现版本(IDE 运行时为 null);
    /// Rust 取 crate 版本(`CARGO_PKG_VERSION`)。
    pub fn export(
        &self,
        compile_cache: &LoadedCompileCache,
    ) -> ExportResult<SerializableParseCache> {
        let main =
            self.export_lambda_definition(compile_cache.q_lambda_definition().as_ref(), None)?;
        let trace_points = if self.include_trace_points {
            Some(self.export_trace_points(compile_cache.expression_trace_points()))
        } else {
            None
        };
        Ok(SerializableParseCache {
            model_version: MODEL_VERSION,
            producer_version: Some(env!("CARGO_PKG_VERSION").to_string()),
            script: Some(self.script.clone()),
            script_hash: Some(sha256_hex(&self.script)),
            main: Some(main),
            trace_points,
        })
    }

    /// 对应 Java 私有方法 `exportLambdaDefinition`:
    /// `QLambdaDefinitionInner` 导出全量;空定义导出空壳;其余报错。
    fn export_lambda_definition(
        &self,
        definition: &dyn QLambdaDefinition,
        owner: Option<&Instruction>,
    ) -> ExportResult<SerializableLambdaDefinition> {
        // Java: definition instanceof QLambdaDefinitionInner
        if let Some(inner) = definition
            .as_any()
            .and_then(|any| any.downcast_ref::<QLambdaDefinitionInner>())
        {
            let mut instructions = Vec::with_capacity(inner.instructions().len());
            for instruction in inner.instructions() {
                instructions.push(self.export_instruction(instruction)?);
            }
            return Ok(SerializableLambdaDefinition {
                name: Some(inner.name().to_string()),
                max_stack_size: inner.max_stack_size() as i32,
                params: Some(self.export_params(inner.params_type())),
                instructions: Some(instructions),
            });
        }
        // Java: definition == QLambdaDefinitionEmpty.INSTANCE || "EmptyLambdaDefinition".equals(...)
        if definition.name() == "EmptyLambdaDefinition" {
            return Ok(SerializableLambdaDefinition {
                name: Some(definition.name().to_string()),
                max_stack_size: 0,
                params: Some(Vec::new()),
                instructions: Some(Vec::new()),
            });
        }
        Err(self.unsupported_instruction(owner, "unknown lambda definition"))
    }

    /// 对应 Java 私有方法 `exportParams`。
    fn export_params(
        &self,
        params: &[crate::runtime::qlambda_definition_inner::Param],
    ) -> Vec<SerializableParam> {
        params
            .iter()
            .map(|param| SerializableParam {
                name: Some(param.name().to_string()),
                // Java: className(param.getClazz())；None 只服务手工构造的
                // 无声明类型参数，按 Object 导出。
                class_name: Some(
                    param
                        .clazz()
                        .map(ClassRef::java_name)
                        .unwrap_or("java.lang.Object")
                        .to_string(),
                ),
            })
            .collect()
    }

    /// 对应 Java 私有方法 `exportInstruction`:按具体指令类型分派导出
    /// (Java `instanceof` 链 ↔ Rust `as_any().downcast_ref()` 链),
    /// opcode 与操作数键名与 Java 完全一致。
    fn export_instruction(
        &self,
        instruction: &Instruction,
    ) -> ExportResult<SerializableInstruction> {
        let any = instruction.as_any();
        // Java: CallConstInstruction 不可序列化(编译期常量 Lambda 调用)
        if any
            .and_then(|a| a.downcast_ref::<CallConstInstruction>())
            .is_some()
        {
            return Err(self.unsupported_instruction(Some(instruction), "CallConstInstruction"));
        }
        let mut operands = Map::new();
        let opcode: &str;

        if let Some(inst) = any.and_then(|a| a.downcast_ref::<ConstInstruction>()) {
            opcode = "CONST";
            let constant = self.export_constant(inst.const_obj(), Some(instruction))?;
            operands.insert(
                "constant".to_string(),
                serde_json::to_value(constant).unwrap(),
            );
            put_optional(&mut operands, "traceKey", inst.trace_key().map(Value::from));
        } else if let Some(inst) = any.and_then(|a| a.downcast_ref::<LoadInstruction>()) {
            opcode = "LOAD";
            operands.insert("name".to_string(), Value::from(inst.name()));
            put_optional(&mut operands, "traceKey", inst.trace_key().map(Value::from));
        } else if any
            .and_then(|a| a.downcast_ref::<PopInstruction>())
            .is_some()
        {
            opcode = "POP";
        } else if let Some(inst) = any.and_then(|a| a.downcast_ref::<ReturnInstruction>()) {
            opcode = "RETURN";
            operands.insert(
                "resultType".to_string(),
                Value::from(return_result_type_name(inst.result_type())),
            );
            put_optional(&mut operands, "traceKey", inst.trace_key().map(Value::from));
        } else if let Some(inst) = any.and_then(|a| a.downcast_ref::<BreakContinueInstruction>()) {
            opcode = "BREAK_CONTINUE";
            operands.insert(
                "resultType".to_string(),
                Value::from(if inst.is_break() { "BREAK" } else { "CONTINUE" }),
            );
        } else if any
            .and_then(|a| a.downcast_ref::<ThrowInstruction>())
            .is_some()
        {
            opcode = "THROW";
        } else if any
            .and_then(|a| a.downcast_ref::<CheckTimeOutInstruction>())
            .is_some()
        {
            opcode = "CHECK_TIMEOUT";
        } else if let Some(inst) = any.and_then(|a| a.downcast_ref::<JumpInstruction>()) {
            opcode = "JUMP";
            operands.insert("position".to_string(), Value::from(inst.position()));
        } else if let Some(inst) = any.and_then(|a| a.downcast_ref::<JumpIfInstruction>()) {
            opcode = "JUMP_IF";
            operands.insert("expect".to_string(), Value::from(inst.is_expect()));
            operands.insert("position".to_string(), Value::from(inst.position()));
            put_optional(&mut operands, "traceKey", inst.trace_key().map(Value::from));
        } else if let Some(inst) = any.and_then(|a| a.downcast_ref::<JumpIfPopInstruction>()) {
            opcode = "JUMP_IF_POP";
            operands.insert("expect".to_string(), Value::from(inst.is_expect()));
            operands.insert("position".to_string(), Value::from(inst.position()));
        } else if let Some(inst) = any.and_then(|a| a.downcast_ref::<OperatorInstruction>()) {
            opcode = "BINARY_OP";
            operands.insert(
                "operator".to_string(),
                Value::from(inst.operator().operator()),
            );
            put_optional(&mut operands, "traceKey", inst.trace_key().map(Value::from));
        } else if let Some(inst) = any.and_then(|a| a.downcast_ref::<UnaryInstruction>()) {
            // Java unaryOpcode:依 OperatorManager 判定前缀/后缀(引用比较 ↔ Rc::ptr_eq)
            let unary_operator = inst.unary_operator();
            opcode = if self
                .operator_manager
                .get_prefix_unary_operator(unary_operator.operator())
                .map(|op| Rc::ptr_eq(&op, unary_operator))
                .unwrap_or(false)
            {
                "PREFIX_UNARY_OP"
            } else if self
                .operator_manager
                .get_suffix_unary_operator(unary_operator.operator())
                .map(|op| Rc::ptr_eq(&op, unary_operator))
                .unwrap_or(false)
            {
                "SUFFIX_UNARY_OP"
            } else {
                return Err(
                    self.unsupported_instruction(Some(instruction), "unknown unary operator")
                );
            };
            operands.insert(
                "operator".to_string(),
                Value::from(unary_operator.operator()),
            );
            put_optional(&mut operands, "traceKey", inst.trace_key().map(Value::from));
        } else if let Some(inst) = any.and_then(|a| a.downcast_ref::<CallFunctionInstruction>()) {
            opcode = "CALL_FUNCTION";
            operands.insert(
                "functionName".to_string(),
                Value::from(inst.function_name()),
            );
            operands.insert("argNum".to_string(), Value::from(inst.arg_num() as i64));
            put_optional(&mut operands, "traceKey", inst.trace_key().map(Value::from));
        } else if let Some(inst) = any.and_then(|a| a.downcast_ref::<CallInstruction>()) {
            opcode = "CALL";
            operands.insert("argNum".to_string(), Value::from(inst.arg_num() as i64));
        } else if let Some(inst) = any.and_then(|a| a.downcast_ref::<LoadLambdaInstruction>()) {
            opcode = "LOAD_LAMBDA";
            let lambda = self
                .export_lambda_definition(inst.lambda_definition().as_ref(), Some(instruction))?;
            operands.insert("lambda".to_string(), serde_json::to_value(lambda).unwrap());
        } else if let Some(inst) = any.and_then(|a| a.downcast_ref::<DefineFunctionInstruction>()) {
            opcode = "DEFINE_FUNCTION";
            operands.insert("name".to_string(), Value::from(inst.name()));
            let lambda = self
                .export_lambda_definition(inst.lambda_definition().as_ref(), Some(instruction))?;
            operands.insert("lambda".to_string(), serde_json::to_value(lambda).unwrap());
        } else if let Some(inst) = any.and_then(|a| a.downcast_ref::<NewScopeInstruction>()) {
            opcode = "NEW_SCOPE";
            operands.insert("scopeName".to_string(), Value::from(inst.scope_name()));
        } else if let Some(inst) = any.and_then(|a| a.downcast_ref::<CloseScopeInstruction>()) {
            opcode = "CLOSE_SCOPE";
            operands.insert("scopeName".to_string(), Value::from(inst.scope_name()));
        } else if let Some(inst) = any.and_then(|a| a.downcast_ref::<DefineLocalInstruction>()) {
            opcode = "DEFINE_LOCAL";
            operands.insert(
                "variableName".to_string(),
                Value::from(inst.variable_name()),
            );
            operands.insert(
                "className".to_string(),
                Value::from(
                    inst.define_clz()
                        .map(ClassRef::java_name)
                        .unwrap_or("java.lang.Object"),
                ),
            );
        } else if let Some(inst) = any.and_then(|a| a.downcast_ref::<NewInstanceInstruction>()) {
            opcode = "NEW_INSTANCE";
            operands.insert(
                "className".to_string(),
                Value::from(inst.new_clz().java_name()),
            );
            operands.insert("argNum".to_string(), Value::from(inst.arg_num() as i64));
        } else if let Some(inst) =
            any.and_then(|a| a.downcast_ref::<NewFilledInstanceInstruction>())
        {
            opcode = "NEW_FILLED_INSTANCE";
            operands.insert(
                "className".to_string(),
                Value::from(inst.new_cls().java_name()),
            );
            operands.insert("keys".to_string(), Value::from(inst.keys().to_vec()));
        } else if let Some(inst) = any.and_then(|a| a.downcast_ref::<NewArrayInstruction>()) {
            opcode = "NEW_ARRAY";
            operands.insert(
                "componentClassName".to_string(),
                Value::from(inst.clz().java_name()),
            );
            operands.insert("length".to_string(), Value::from(inst.length() as i64));
        } else if let Some(inst) = any.and_then(|a| a.downcast_ref::<MultiNewArrayInstruction>()) {
            opcode = "MULTI_NEW_ARRAY";
            operands.insert(
                "componentClassName".to_string(),
                Value::from(inst.clz().java_name()),
            );
            operands.insert("dims".to_string(), Value::from(inst.dims() as i64));
        } else if let Some(inst) = any.and_then(|a| a.downcast_ref::<NewListInstruction>()) {
            opcode = "NEW_LIST";
            operands.insert(
                "initLength".to_string(),
                Value::from(inst.init_length() as i64),
            );
        } else if let Some(inst) = any.and_then(|a| a.downcast_ref::<NewMapInstruction>()) {
            opcode = "NEW_MAP";
            operands.insert("keys".to_string(), Value::from(inst.keys().to_vec()));
        } else if let Some(inst) = any.and_then(|a| a.downcast_ref::<GetFieldInstruction>()) {
            opcode = "GET_FIELD";
            operands.insert("fieldName".to_string(), Value::from(inst.field_name()));
            operands.insert("optional".to_string(), Value::from(inst.is_optional()));
        } else if let Some(inst) = any.and_then(|a| a.downcast_ref::<SpreadGetFieldInstruction>()) {
            opcode = "SPREAD_GET_FIELD";
            operands.insert("fieldName".to_string(), Value::from(inst.field_name()));
        } else if let Some(inst) = any.and_then(|a| a.downcast_ref::<MethodInvokeInstruction>()) {
            opcode = "METHOD_INVOKE";
            operands.insert("methodName".to_string(), Value::from(inst.method_name()));
            operands.insert("argNum".to_string(), Value::from(inst.arg_num() as i64));
            operands.insert("optional".to_string(), Value::from(inst.is_optional()));
        } else if let Some(inst) =
            any.and_then(|a| a.downcast_ref::<SpreadMethodInvokeInstruction>())
        {
            opcode = "SPREAD_METHOD_INVOKE";
            operands.insert("methodName".to_string(), Value::from(inst.method_name()));
            operands.insert("argNum".to_string(), Value::from(inst.arg_num() as i64));
        } else if let Some(inst) = any.and_then(|a| a.downcast_ref::<GetMethodInstruction>()) {
            opcode = "GET_METHOD";
            operands.insert("methodName".to_string(), Value::from(inst.method_name()));
        } else if any
            .and_then(|a| a.downcast_ref::<IndexInstruction>())
            .is_some()
        {
            opcode = "INDEX";
        } else if let Some(inst) = any.and_then(|a| a.downcast_ref::<SliceInstruction>()) {
            opcode = "SLICE";
            operands.insert(
                "mode".to_string(),
                Value::from(slice_mode_name(inst.mode())),
            );
        } else if any
            .and_then(|a| a.downcast_ref::<CastInstruction>())
            .is_some()
        {
            opcode = "CAST";
        } else if let Some(inst) = any.and_then(|a| a.downcast_ref::<WhileInstruction>()) {
            opcode = "WHILE";
            let condition =
                self.export_lambda_definition(inst.condition().as_ref(), Some(instruction))?;
            let body = self.export_lambda_definition(inst.body().as_ref(), Some(instruction))?;
            operands.insert(
                "condition".to_string(),
                serde_json::to_value(condition).unwrap(),
            );
            operands.insert("body".to_string(), serde_json::to_value(body).unwrap());
            operands.insert(
                "whileScopeMaxStackSize".to_string(),
                Value::from(inst.while_scope_max_stack_size() as i64),
            );
        } else if let Some(inst) = any.and_then(|a| a.downcast_ref::<ForInstruction>()) {
            opcode = "FOR";
            if let Some(for_init) = inst.for_init() {
                let lambda = self.export_lambda_definition(for_init.as_ref(), Some(instruction))?;
                operands.insert("forInit".to_string(), serde_json::to_value(lambda).unwrap());
            }
            if let Some(condition) = inst.condition() {
                let lambda =
                    self.export_lambda_definition(condition.as_ref(), Some(instruction))?;
                operands.insert(
                    "condition".to_string(),
                    serde_json::to_value(lambda).unwrap(),
                );
            }
            put_optional(
                &mut operands,
                "conditionSource",
                Some(serde_json::to_value(source_of(inst.condition_error_reporter())).unwrap()),
            );
            if let Some(for_update) = inst.for_update() {
                let lambda =
                    self.export_lambda_definition(for_update.as_ref(), Some(instruction))?;
                operands.insert(
                    "forUpdate".to_string(),
                    serde_json::to_value(lambda).unwrap(),
                );
            }
            operands.insert(
                "forScopeMaxStackSize".to_string(),
                Value::from(inst.for_scope_max_stack_size() as i64),
            );
            let for_body =
                self.export_lambda_definition(inst.for_body().as_ref(), Some(instruction))?;
            operands.insert(
                "forBody".to_string(),
                serde_json::to_value(for_body).unwrap(),
            );
        } else if let Some(inst) = any.and_then(|a| a.downcast_ref::<ForEachInstruction>()) {
            opcode = "FOR_EACH";
            let body = self.export_lambda_definition(inst.body().as_ref(), Some(instruction))?;
            operands.insert("body".to_string(), serde_json::to_value(body).unwrap());
            operands.insert(
                "itemClassName".to_string(),
                Value::from(inst.it_cls().java_name()),
            );
            operands.insert(
                "targetSource".to_string(),
                serde_json::to_value(source_of(inst.target_error_reporter())).unwrap(),
            );
        } else if let Some(inst) = any.and_then(|a| a.downcast_ref::<TryCatchInstruction>()) {
            opcode = "TRY_CATCH";
            let body = self.export_lambda_definition(inst.body().as_ref(), Some(instruction))?;
            operands.insert("body".to_string(), serde_json::to_value(body).unwrap());
            let exception_table = self.export_exception_table(inst, Some(instruction))?;
            operands.insert(
                "exceptionTable".to_string(),
                serde_json::to_value(exception_table).unwrap(),
            );
            if let Some(final_body) = inst.final_body() {
                let lambda =
                    self.export_lambda_definition(final_body.as_ref(), Some(instruction))?;
                operands.insert(
                    "finalBody".to_string(),
                    serde_json::to_value(lambda).unwrap(),
                );
            }
        } else if let Some(inst) = any.and_then(|a| a.downcast_ref::<TracePeekInstruction>()) {
            opcode = "TRACE_PEEK";
            put_optional(&mut operands, "traceKey", inst.trace_key().map(Value::from));
        } else if let Some(inst) = any.and_then(|a| a.downcast_ref::<TraceEvaluatedInstruction>()) {
            opcode = "TRACE_EVALUATED";
            put_optional(&mut operands, "traceKey", inst.trace_key().map(Value::from));
        } else if let Some(inst) = any.and_then(|a| a.downcast_ref::<StringJoinInstruction>()) {
            opcode = "STRING_JOIN";
            operands.insert("n".to_string(), Value::from(inst.n() as i64));
        } else {
            // Java default 分支: unsupportedInstruction(instruction, class.getName())
            return Err(self.unsupported_instruction(Some(instruction), "unknown instruction"));
        }

        Ok(SerializableInstruction {
            opcode: Some(opcode.to_string()),
            source: Some(source_of(instruction.error_reporter())),
            operands: Some(operands),
        })
    }

    /// 对应 Java 私有方法 `exportExceptionTable`。
    fn export_exception_table(
        &self,
        instruction: &TryCatchInstruction,
        owner: Option<&Instruction>,
    ) -> ExportResult<Vec<SerializableCatchEntry>> {
        let mut result = Vec::with_capacity(instruction.exception_table().len());
        for (class_ref, handler) in instruction.exception_table() {
            result.push(SerializableCatchEntry {
                exception_class_name: Some(class_ref.java_name().to_string()),
                handler: Some(self.export_lambda_definition(handler.as_ref(), owner)?),
            });
        }
        Ok(result)
    }

    /// 对应 Java 私有方法 `exportConstant`:类型标签与值形态与 Java 完全一致。
    ///
    /// 偏差说明:Java 常量对象为 `Object`,类型按 `instanceof` 分派;Rust
    /// 常量为 [`DataValue`]。`Byte`/`Short` 在 Java 侧不存在(字面量即
    /// `Integer`),Rust 统一导出为 `INT`;`List/Map/Array/Lambda` 等运行期
    /// 值与 Java 一致地报 `UNSUPPORTED_CONSTANT`。
    fn export_constant(
        &self,
        value: &DataValue,
        owner: Option<&Instruction>,
    ) -> ExportResult<SerializableConstant> {
        let constant = match value {
            DataValue::Null => SerializableConstant {
                const_type: Some("NULL".to_string()),
                value: None,
            },
            DataValue::Bool(v) => typed_constant("BOOLEAN", Value::from(*v)),
            DataValue::Str(v) => {
                let Some(value) = v.to_rust_string() else {
                    return Err(SerializableParseCacheException::new(
                        Some(&self.script),
                        owner.map(|inst| source_of(inst.error_reporter())).as_ref(),
                        error_codes::SERIALIZABLE_PARSE_CACHE_UNSUPPORTED_CONSTANT,
                        &crate::exception::error_codes::format_msg(
                            error_codes::error_msg(
                                error_codes::SERIALIZABLE_PARSE_CACHE_UNSUPPORTED_CONSTANT,
                            ),
                            &["java.lang.String(unpaired UTF-16 surrogate)".to_string()],
                        ),
                    ));
                };
                typed_constant("STRING", Value::from(value))
            }
            DataValue::Char(v) => match char::from_u32(u32::from(*v)) {
                Some(character) => typed_constant("CHAR", Value::from(character.to_string())),
                None => {
                    // Rust/serde_json 的字符串不能承载 Java 未配对 surrogate。
                    // 禁止静默替换为 U+FFFD；由平台适配台账记录该不可无损边界。
                    return Err(SerializableParseCacheException::new(
                        Some(&self.script),
                        owner.map(|inst| source_of(inst.error_reporter())).as_ref(),
                        error_codes::SERIALIZABLE_PARSE_CACHE_UNSUPPORTED_CONSTANT,
                        &crate::exception::error_codes::format_msg(
                            error_codes::error_msg(
                                error_codes::SERIALIZABLE_PARSE_CACHE_UNSUPPORTED_CONSTANT,
                            ),
                            &[format!("java.lang.Character(U+{v:04X})")],
                        ),
                    ));
                }
            },
            DataValue::Byte(v) => typed_constant("INT", Value::from(*v)),
            DataValue::Short(v) => typed_constant("INT", Value::from(*v)),
            DataValue::Int(v) => typed_constant("INT", Value::from(*v)),
            DataValue::Long(v) => typed_constant("LONG", Value::from(*v)),
            DataValue::BigInt(v) => typed_constant("BIG_INTEGER", Value::from(v.to_string())),
            DataValue::Float(v) => typed_constant("FLOAT", Value::from(*v)),
            DataValue::Double(v) => typed_constant("DOUBLE", Value::from(*v)),
            DataValue::BigDec(v) => typed_constant("BIG_DECIMAL", Value::from(v.clone())),
            other => {
                // Java: value instanceof MetaClass / Class → META_CLASS
                if let Some(class_ref) = as_meta_class(other) {
                    typed_constant("META_CLASS", Value::from(class_ref.java_name()))
                } else {
                    return Err(SerializableParseCacheException::new(
                        Some(&self.script),
                        owner.map(|inst| source_of(inst.error_reporter())).as_ref(),
                        error_codes::SERIALIZABLE_PARSE_CACHE_UNSUPPORTED_CONSTANT,
                        &crate::exception::error_codes::format_msg(
                            error_codes::error_msg(
                                error_codes::SERIALIZABLE_PARSE_CACHE_UNSUPPORTED_CONSTANT,
                            ),
                            &[other.data_type_name().to_string()],
                        ),
                    ));
                }
            }
        };
        Ok(constant)
    }

    /// 对应 Java 私有方法 `exportTracePoints` / `exportTracePoint`。
    fn export_trace_points(&self, trace_points: &[TracePointTree]) -> Vec<SerializableTracePoint> {
        trace_points.iter().map(export_trace_point).collect()
    }

    /// 对应 Java 私有方法 `unsupportedInstruction`。
    fn unsupported_instruction(
        &self,
        instruction: Option<&Instruction>,
        instruction_name: &str,
    ) -> SerializableParseCacheException {
        SerializableParseCacheException::new(
            Some(&self.script),
            instruction
                .map(|inst| source_of(inst.error_reporter()))
                .as_ref(),
            error_codes::SERIALIZABLE_PARSE_CACHE_UNSUPPORTED_INSTRUCTION,
            &crate::exception::error_codes::format_msg(
                error_codes::error_msg(
                    error_codes::SERIALIZABLE_PARSE_CACHE_UNSUPPORTED_INSTRUCTION,
                ),
                &[instruction_name.to_string()],
            ),
        )
    }
}

/// 构造带值的常量(辅助,对应 Java `constant.setType/setValue` 片段)。
fn typed_constant(const_type: &str, value: Value) -> SerializableConstant {
    SerializableConstant {
        const_type: Some(const_type.to_string()),
        value: Some(value),
    }
}

/// 导出单个 trace 点(递归子节点)。对应 Java `exportTracePoint`。
fn export_trace_point(trace_point: &TracePointTree) -> SerializableTracePoint {
    SerializableTracePoint {
        trace_type: Some(trace_type::java_name(trace_point.trace_type()).to_string()),
        token: Some(trace_point.token().to_string()),
        line: trace_point.line(),
        col: trace_point.col(),
        position: trace_point.position(),
        children: Some(
            trace_point
                .children()
                .iter()
                .map(export_trace_point)
                .collect(),
        ),
    }
}

/// 对应 Java 私有静态方法 `sourceOf(ErrorReporter)`:
/// `DefaultErrReporter` 取实际位置(col 转 0 基);其余归一为 (0,1,0,"")。
pub(crate) fn source_of(error_reporter: &Rc<dyn ErrorReporter>) -> SerializableSource {
    if let Some(reporter) = error_reporter
        .as_any()
        .and_then(|any| any.downcast_ref::<DefaultErrReporter>())
    {
        return SerializableSource {
            start: reporter.token_start_pos(),
            line: reporter.line(),
            col: (reporter.col() - 1).max(0),
            lexeme: Some(reporter.lexeme().to_string()),
        };
    }
    SerializableSource {
        start: 0,
        line: 1,
        col: 0,
        lexeme: Some(String::new()),
    }
}

/// 对应 Java 私有静态方法 `putOptional`。
fn put_optional(operands: &mut Map<String, Value>, key: &str, value: Option<Value>) {
    if let Some(value) = value {
        operands.insert(key.to_string(), value);
    }
}

/// `ReturnResultType` → Java `QResult.ResultType.name()`。
fn return_result_type_name(result_type: ReturnResultType) -> &'static str {
    match result_type {
        ReturnResultType::Return => "RETURN",
        ReturnResultType::Break => "BREAK",
        ReturnResultType::Continue => "CONTINUE",
    }
}

/// `SliceMode` → Java `SliceInstruction.Mode.name()`。
fn slice_mode_name(mode: SliceMode) -> &'static str {
    match mode {
        SliceMode::Left => "LEFT",
        SliceMode::Right => "RIGHT",
        SliceMode::Both => "BOTH",
        SliceMode::Copy => "COPY",
    }
}

/// 纯 std 的 SHA-256(十六进制小写)。对应 Java 私有静态方法 `sha256`
/// (`MessageDigest.getInstance("SHA-256")` + `%02x` 拼接)。
/// SPEC 要求零外部依赖(serde 除外),故此处自实现。
fn sha256_hex(value: &str) -> String {
    // SHA-256 常量(K 表)
    const K: [u32; 64] = [
        0x428a2f98, 0x71374491, 0xb5c0fbcf, 0xe9b5dba5, 0x3956c25b, 0x59f111f1, 0x923f82a4,
        0xab1c5ed5, 0xd807aa98, 0x12835b01, 0x243185be, 0x550c7dc3, 0x72be5d74, 0x80deb1fe,
        0x9bdc06a7, 0xc19bf174, 0xe49b69c1, 0xefbe4786, 0x0fc19dc6, 0x240ca1cc, 0x2de92c6f,
        0x4a7484aa, 0x5cb0a9dc, 0x76f988da, 0x983e5152, 0xa831c66d, 0xb00327c8, 0xbf597fc7,
        0xc6e00bf3, 0xd5a79147, 0x06ca6351, 0x14292967, 0x27b70a85, 0x2e1b2138, 0x4d2c6dfc,
        0x53380d13, 0x650a7354, 0x766a0abb, 0x81c2c92e, 0x92722c85, 0xa2bfe8a1, 0xa81a664b,
        0xc24b8b70, 0xc76c51a3, 0xd192e819, 0xd6990624, 0xf40e3585, 0x106aa070, 0x19a4c116,
        0x1e376c08, 0x2748774c, 0x34b0bcb5, 0x391c0cb3, 0x4ed8aa4a, 0x5b9cca4f, 0x682e6ff3,
        0x748f82ee, 0x78a5636f, 0x84c87814, 0x8cc70208, 0x90befffa, 0xa4506ceb, 0xbef9a3f7,
        0xc67178f2,
    ];
    let mut h: [u32; 8] = [
        0x6a09e667, 0xbb67ae85, 0x3c6ef372, 0xa54ff53a, 0x510e527f, 0x9b05688c, 0x1f83d9ab,
        0x5be0cd19,
    ];
    // 填充:消息 + 0x80 + 0 填充 + 8 字节大端位长度
    let mut data = value.as_bytes().to_vec();
    let bit_len = (data.len() as u64).wrapping_mul(8);
    data.push(0x80);
    while data.len() % 64 != 56 {
        data.push(0);
    }
    data.extend_from_slice(&bit_len.to_be_bytes());
    for chunk in data.chunks_exact(64) {
        let mut w = [0u32; 64];
        for (i, word) in w.iter_mut().take(16).enumerate() {
            *word = u32::from_be_bytes([
                chunk[i * 4],
                chunk[i * 4 + 1],
                chunk[i * 4 + 2],
                chunk[i * 4 + 3],
            ]);
        }
        for i in 16..64 {
            let s0 = w[i - 15].rotate_right(7) ^ w[i - 15].rotate_right(18) ^ (w[i - 15] >> 3);
            let s1 = w[i - 2].rotate_right(17) ^ w[i - 2].rotate_right(19) ^ (w[i - 2] >> 10);
            w[i] = w[i - 16]
                .wrapping_add(s0)
                .wrapping_add(w[i - 7])
                .wrapping_add(s1);
        }
        let [mut a, mut b, mut c, mut d, mut e, mut f, mut g, mut hh] = h;
        for i in 0..64 {
            let s1 = e.rotate_right(6) ^ e.rotate_right(11) ^ e.rotate_right(25);
            let ch = (e & f) ^ ((!e) & g);
            let temp1 = hh
                .wrapping_add(s1)
                .wrapping_add(ch)
                .wrapping_add(K[i])
                .wrapping_add(w[i]);
            let s0 = a.rotate_right(2) ^ a.rotate_right(13) ^ a.rotate_right(22);
            let maj = (a & b) ^ (a & c) ^ (b & c);
            let temp2 = s0.wrapping_add(maj);
            hh = g;
            g = f;
            f = e;
            e = d.wrapping_add(temp1);
            d = c;
            c = b;
            b = a;
            a = temp1.wrapping_add(temp2);
        }
        h[0] = h[0].wrapping_add(a);
        h[1] = h[1].wrapping_add(b);
        h[2] = h[2].wrapping_add(c);
        h[3] = h[3].wrapping_add(d);
        h[4] = h[4].wrapping_add(e);
        h[5] = h[5].wrapping_add(f);
        h[6] = h[6].wrapping_add(g);
        h[7] = h[7].wrapping_add(hh);
    }
    let mut result = String::with_capacity(64);
    for word in h {
        result.push_str(&format!("{:08x}", word));
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sha256_matches_java_vector() {
        // Java `MessageDigest("SHA-256")` 标准向量
        assert_eq!(
            sha256_hex(""),
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );
        assert_eq!(
            sha256_hex("abc"),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
    }
}
