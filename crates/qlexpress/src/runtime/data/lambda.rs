//! Method reference as a lambda, mirroring Java
//! `com.alibaba.qlexpress4.runtime.data.lambda.QLambdaMethod`.

use std::rc::Rc;

use crate::exception::error_codes;
use crate::exception::error_reporter::ErrorReporter;
use crate::exception::pure_err_reporter::PureErrReporter;
use crate::exception::QLException;
use crate::runtime::member::{as_meta_class, find_method_and_invoke, NativeRegistry};
use crate::runtime::q_result::QResult;
use crate::runtime::value::DataValue;

/// A bound method (`obj.method` / `Cls.method`) usable as a lambda,
/// mirroring Java `QLambdaMethod` (produced by `GetMethodInstruction`).
pub struct QLambdaMethod {
    method_name: String,
    registry: Rc<NativeRegistry>,
    bean: DataValue,
}

impl QLambdaMethod {
    pub fn new(
        method_name: impl Into<String>,
        registry: Rc<NativeRegistry>,
        bean: DataValue,
    ) -> Self {
        QLambdaMethod {
            method_name: method_name.into(),
            registry,
            bean,
        }
    }

    pub fn method_name(&self) -> &str {
        &self.method_name
    }

    /// Java `call(Object... params)`.
    pub fn call(&self, params: &[DataValue]) -> Result<QResult, QLException> {
        if let Some(meta_clz) = as_meta_class(&self.bean) {
            // Static method path (Java `bean instanceof MetaClass`).
            if let Some(method) = self.registry.resolve_method(&self.bean, &self.method_name) {
                let value =
                    crate::runtime::member::invoke_native_method(&self.bean, &method, params)?;
                return Ok(QResult::Return(value.get()));
            }
            if params.is_empty() {
                return Err(self.method_not_found(params));
            }
            // Java: first argument must be an instance of the class; the
            // method is then resolved on it with the remaining arguments.
            if params[0].data_type_name() != meta_clz.java_name() {
                return Err(self.method_not_found(params));
            }
            let rest = &params[1..];
            if self
                .registry
                .resolve_method(&params[0], &self.method_name)
                .is_none()
            {
                return Err(self.method_not_found(rest));
            }
            let value = find_method_and_invoke(
                &params[0],
                &self.method_name,
                rest,
                &self.registry,
                &PureErrReporter::INSTANCE,
            )?;
            Ok(QResult::Return(value.get()))
        } else {
            let value = find_method_and_invoke(
                &self.bean,
                &self.method_name,
                params,
                &self.registry,
                &PureErrReporter::INSTANCE,
            )?;
            Ok(QResult::Return(value.get()))
        }
    }

    /// Java `createMethodNotFoundException`: `UserDefineException` of type
    /// `INVALID_ARGUMENT`.
    fn method_not_found(&self, types: &[DataValue]) -> QLException {
        let types_render = types
            .iter()
            .map(|t| t.data_type_name().to_string())
            .collect::<Vec<_>>()
            .join(", ");
        PureErrReporter::INSTANCE.report(
            error_codes::INVALID_ARGUMENT,
            &format!(
                "method reference '{}' not found for argument types [{}]",
                self.method_name, types_render
            ),
        )
    }
}
