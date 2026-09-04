//! 方法查找与调用工具,对应 Java `com.alibaba.qlexpress4.runtime.util.MethodInvokeUtils`。
//!
//! 本文件同时承接 Stage 3a 落在 `runtime/member.rs` 的同名逻辑
//! (SPEC §5.5.6 拆分:`member.rs` 仅保留 re-export,实现归位到与 Java
//! 包位置对应的 `runtime/util/`)。

use std::rc::Rc;

use crate::exception::error_codes;
use crate::exception::error_reporter::ErrorReporter;
use crate::exception::QLException;
use crate::runtime::data::convert::parameters_type_convertor::ParametersTypeConvertor;
use crate::runtime::i_method::IMethod;
use crate::runtime::meta_class::as_meta_class;
use crate::runtime::native_registry::NativeRegistry;
use crate::runtime::native_type::NativeMethod;
use crate::runtime::qlambda::QLambda;
use crate::runtime::reflect_loader::ReflectLoader;
use crate::runtime::value::{DataValue, QValue};

/// 调用已解析的原生方法。对应 Java `MethodInvokeUtils.invokeIMethod`
/// 的调用半段(类型转换已在注册闭包内按 Java 语义完成):
/// 转换错误/内部错误以 `QLException` 传播,如同 Java 重抛 `QLRuntimeException`。
pub fn invoke_native_method(
    bean: &DataValue,
    method: &NativeMethod,
    params: &[DataValue],
) -> Result<QValue, QLException> {
    method(bean, params).map(QValue::Data)
}

/// 对应 Java 方法 `MethodInvokeUtils.invokeIMethod(Object, String, IMethod,
/// Object[], ErrorReporter)`:先按形参类型转换实参,再经访问检查调用。
///
/// Java 语义要点:Java 调用前 `ParametersTypeConvertor.cast`,调用异常经
/// `ReflectLoader.unwrapMethodInvokeEx` 解包(`QLRuntimeException` 原样重抛);
/// Rust 侧脚本失败已是 `QLException`,与 Java 的「原样重抛」分支一致。
pub fn invoke_i_method(
    bean: &DataValue,
    method_name: &str,
    method: &Rc<dyn IMethod>,
    params: &[DataValue],
    error_reporter: &dyn ErrorReporter,
) -> Result<QValue, QLException> {
    let parameter_types = method.parameter_types();
    let convert_result =
        ParametersTypeConvertor::cast(params, &parameter_types, method.is_var_args())?;
    // Java: MethodHandler.Access.accessMethodValue(必要时 setAccessible(true))。
    if !method.is_access() {
        method.set_accessible(true);
    }
    method
        .invoke(bean, &convert_result)
        .map(QValue::Data)
        .map_err(|error| ReflectLoader::unwrap_method_invoke_ex(error_reporter, method_name, error))
}

/// 对应 Java 私有方法 `MethodInvokeUtils.findQLambdaInstance`:
/// Map 中以方法名为 key 存储的 Lambda 可作为「方法」调用。
fn find_q_lambda_instance(bean: &DataValue, method_name: &str) -> Option<Rc<QLambda>> {
    if let DataValue::Map(map) = bean {
        if let Some(DataValue::Lambda(lambda)) = map.borrow().get(&DataValue::string(method_name)) {
            return Some(Rc::clone(lambda));
        }
    }
    None
}

/// 对应 Java 方法 `MethodInvokeUtils.findMethodAndInvoke(Object, String,
/// Object[], Class<?>[], ReflectLoader, ErrorReporter)`:
/// 注册表方法 → 宿主对象动态分派 → Map 内 Lambda → 报 `METHOD_NOT_FOUND`。
///
/// Java 语义要点:Java 在 `method == null` 时才尝试 `findQLambdaInstance`;
/// Rust 额外保留 `NativeObject::call_method` 动态分派分支,对应 Java 对
/// 任意宿主对象的反射调用能力(SPEC §4)。
pub fn find_method_and_invoke(
    bean: &DataValue,
    method_name: &str,
    params: &[DataValue],
    registry: &NativeRegistry,
    error_reporter: &dyn ErrorReporter,
) -> Result<QValue, QLException> {
    if let Some(method) = registry.resolve_method_for_args(bean, method_name, params) {
        return invoke_native_method(bean, &method, params).map_err(|error| {
            ReflectLoader::unwrap_method_invoke_ex(error_reporter, method_name, error)
        });
    }
    // 宿主对象动态分派(NativeObject::call_method,对应 Java 反射调用)。
    if let DataValue::Object(obj) = bean {
        if as_meta_class(bean).is_none() {
            let type_name = obj.borrow().native_type_name().to_string();
            if registry.has_registered_method_candidates(&type_name, method_name) {
                return Err(error_reporter.report_format(
                    error_codes::METHOD_NOT_FOUND,
                    error_codes::error_msg(error_codes::METHOD_NOT_FOUND),
                    &[method_name.to_string(), format!("{params:?}")],
                ));
            }
            if !registry.is_member_allowed(&type_name, method_name) {
                return Err(error_reporter.report_format(
                    error_codes::METHOD_NOT_FOUND,
                    error_codes::error_msg(error_codes::METHOD_NOT_FOUND),
                    &[method_name.to_string(), format!("{params:?}")],
                ));
            }
            let result = obj
                .borrow_mut()
                .call_method(method_name, params)
                .map_err(|error| {
                    ReflectLoader::unwrap_method_invoke_ex(error_reporter, method_name, error)
                })?;
            return Ok(QValue::Data(result));
        }
    }
    if let Some(q_lambda) = find_q_lambda_instance(bean, method_name) {
        // Java:lambda 调用失败按 UserDefineException/Throwable 分类包装;
        // Rust 的 QLException 已是脚本期错误的统一形态,原样上抛(同 Java
        // 对 QLRuntimeException 的重抛分支)。
        let q_result = q_lambda.call(params)?;
        return Ok(q_result.value().into());
    }
    let params_render = params
        .iter()
        .map(DataValue::string_value_of)
        .collect::<Vec<_>>()
        .join(", ");
    Err(error_reporter.report_format(
        error_codes::METHOD_NOT_FOUND,
        error_codes::error_msg(error_codes::METHOD_NOT_FOUND),
        &[method_name.to_string(), format!("[{params_render}]")],
    ))
}
