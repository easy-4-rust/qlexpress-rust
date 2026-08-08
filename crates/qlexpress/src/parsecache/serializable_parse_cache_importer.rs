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

include!("serializable_parse_cache_importer/instructions.rs");
include!("serializable_parse_cache_importer/control_and_constants.rs");
include!("serializable_parse_cache_importer/operators_and_reporters.rs");
include!("serializable_parse_cache_importer/model_validation.rs");

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
