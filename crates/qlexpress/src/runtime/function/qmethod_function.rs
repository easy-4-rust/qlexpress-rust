//! Java 方法的函数包装,对应 Java `com.alibaba.qlexpress4.runtime.function.QMethodFunction`。

use std::rc::Rc;

use crate::exception::error_codes;
use crate::exception::ql_exception::QLExceptionKind;
use crate::exception::QLException;
use crate::member::method_handler::Access;
use crate::runtime::class_ref::ClassRef;
use crate::runtime::data::convert::parameters_type_convertor::ParametersTypeConvertor;
use crate::runtime::function::custom_function::CustomFunction;
use crate::runtime::i_method::IMethod;
use crate::runtime::member_resolver::MemberResolver;
use crate::runtime::parameters::Parameters;
use crate::runtime::qcontext::QContext;
use crate::runtime::value::DataValue;

/// 把(静态/实例)方法包装为脚本函数。
/// 对应 Java: com.alibaba.qlexpress4.runtime.function.QMethodFunction
/// (`Express4Runner.addFunctionOfClassMethod` 系列 API 的底层包装)。
///
/// 适配说明(SPEC §4):Java 构造器接收 `java.lang.reflect.Method` 并包成
/// `JvmIMethod`;Rust 接收显式注册的 [`IMethod`]
/// (通常由 [`crate::runtime::jvm_i_method::NativeIMethod`] 包装原生闭包),
/// 解析/转换/调用流程与 Java 完全一致。
pub struct QMethodFunction {
    /// 接收者对象(Java `Object object`;静态方法为 `null`,
    /// Rust 用 `None` 表示,调用时传 `DataValue::Null`)。
    object: Option<DataValue>,
    /// 被包装的方法(Java `IMethod method`,实为 `JvmIMethod`)。
    method: Rc<dyn IMethod>,
}

impl QMethodFunction {
    /// 对应 Java 构造器 `QMethodFunction(Object object, Method method)`。
    pub fn new(object: Option<DataValue>, method: Rc<dyn IMethod>) -> Self {
        QMethodFunction { object, method }
    }
}

impl CustomFunction for QMethodFunction {
    /// 对应 Java 方法 `call(QContext, Parameters)`:
    /// 1. 取实参值与类型;2. `MemberResolver.resolveMethod` 校验匹配;
    /// 3. `ParametersTypeConvertor.cast` 转换;4. `MethodHandler.Access`
    ///
    /// 带访问控制调用。
    fn call(
        &self,
        _q_context: &mut dyn QContext,
        parameters: &Parameters,
    ) -> Result<DataValue, QLException> {
        let mut types = Vec::with_capacity(parameters.size());
        let mut params = Vec::with_capacity(parameters.size());
        for i in 0..parameters.size() {
            // Java: Value v = parameters.get(i); params[i] = v.get(); type[i] = v.getType();
            let value = parameters.get(i);
            let (data, type_name) = match value {
                Some(v) => (v.get(), v.type_name()),
                None => (DataValue::Null, "com.alibaba.qlexpress4.runtime.Nothing"),
            };
            params.push(data);
            types.push(ClassRef::from_name(type_name));
        }

        // Java: resolved == null 即实参类型不匹配,抛 INVALID_ARGUMENT。
        let resolved = MemberResolver::resolve_method(&[Rc::clone(&self.method)], &types);
        if resolved.is_none() {
            let types_render = types
                .iter()
                .map(|t| t.java_name().to_string())
                .collect::<Vec<_>>()
                .join(", ");
            return Err(QLException::for_test(
                QLExceptionKind::Runtime,
                format!(
                    "invalid argument types [{}] for java method '{}' in declaring java class '{}'",
                    types_render,
                    self.method.name(),
                    self.method.declaring_class().java_name()
                ),
                error_codes::INVALID_ARGUMENT,
            ));
        }

        // Java: ParametersTypeConvertor.cast(params, method.getParameterTypes(), method.isVarArgs())。
        let target_types: Vec<_> = self
            .method
            .parameter_types()
            .iter()
            .map(|class_ref| class_ref.to_target_type())
            .collect();
        let convert_result =
            ParametersTypeConvertor::cast(&params, &target_types, self.method.is_var_args());
        // Java: MethodHandler.Access.accessMethodValue(method, object, convertResult)。
        Access::access_method_value(
            &self.method,
            self.object.as_ref().unwrap_or(&DataValue::Null),
            &convert_result,
        )
    }
}
