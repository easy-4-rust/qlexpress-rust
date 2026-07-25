//! Invocation instructions, mirroring Java `CallInstruction`,
//! `CallFunctionInstruction`, `MethodInvokeInstruction`,
//! `SpreadMethodInvokeInstruction`.

use std::rc::Rc;

use crate::exception::error_codes;
use crate::exception::error_reporter::ErrorReporter;
use crate::exception::QLException;
use crate::ql_options::QLOptions;
use crate::ql_result::QResult;
use crate::runtime::instruction::{with_trace, QLInstruction};
use crate::runtime::member::{find_method_and_invoke, invoke_native_method};
use crate::runtime::qcontext::QContext;
use crate::runtime::util::throw_utils::{report_user_defined_exception, wrap_throwable};
use crate::runtime::value::{DataValue, QValue};
use crate::utils::println_utils::PrintlnUtils;

/// Whether a lambda-call error came from a user-defined exception
/// (Java `catch (UserDefineException e)`).
fn is_user_defined(err: &QLException) -> bool {
    err.error_code() == error_codes::INVALID_ARGUMENT
        || err.error_code() == error_codes::BIZ_EXCEPTION
}

/// Java `ThrowUtils.reportUserDefinedException` / `wrapThrowable` dispatch
/// used by the call instructions.
fn rethrow_call_error(
    error_reporter: &Rc<dyn ErrorReporter>,
    err: QLException,
    wrap_code: &str,
    wrap_msg: &str,
    wrap_args: &[String],
) -> QLException {
    if is_user_defined(&err) {
        report_user_defined_exception(&**error_reporter, &err)
    } else {
        wrap_throwable(err, &**error_reporter, wrap_code, wrap_msg, wrap_args)
    }
}

/// Operation: call a lambda with fixed number of arguments
/// Input: ${argNum} + 1
/// Output: 1, lambda return result
///
/// Mirrors Java `CallInstruction`.
pub struct CallInstruction {
    error_reporter: Rc<dyn ErrorReporter>,
    arg_num: usize,
}

impl CallInstruction {
    pub fn new(error_reporter: Rc<dyn ErrorReporter>, arg_num: usize) -> Self {
        CallInstruction {
            error_reporter,
            arg_num,
        }
    }

    pub fn arg_num(&self) -> usize {
        self.arg_num
    }
}

impl QLInstruction for CallInstruction {
    fn execute(
        &self,
        q_context: &mut dyn QContext,
        ql_options: &QLOptions,
    ) -> Result<QResult, QLException> {
        let parameters = q_context.pop_n(self.arg_num + 1);
        let bean = parameters
            .get(0)
            .expect("lambda slot popped")
            .get();
        if bean.is_null() {
            if ql_options.is_avoid_null_pointer() {
                q_context.push(QValue::Data(DataValue::NULL_VALUE));
                return Ok(QResult::NEXT_INSTRUCTION);
            }
            return Err(self.error_reporter.report(
                error_codes::NULL_CALL,
                error_codes::error_msg(error_codes::NULL_CALL),
            ));
        }
        let DataValue::Lambda(q_lambda) = &bean else {
            return Err(self.error_reporter.report_format(
                error_codes::OBJECT_NOT_CALLABLE,
                error_codes::error_msg(error_codes::OBJECT_NOT_CALLABLE),
                &[bean.data_type_name().to_string()],
            ));
        };
        let params: Vec<DataValue> = (0..self.arg_num)
            .map(|i| parameters.get_value(i + 1))
            .collect();
        match q_lambda.call(&params) {
            Ok(result) => {
                q_context.push(QValue::Data(result.value()).to_immutable());
                Ok(QResult::NEXT_INSTRUCTION)
            }
            Err(err) => Err(rethrow_call_error(
                &self.error_reporter,
                err,
                error_codes::INVOKE_LAMBDA_ERROR,
                error_codes::error_msg(error_codes::INVOKE_LAMBDA_ERROR),
                &[],
            )),
        }
    }

    fn stack_input(&self) -> i32 {
        self.arg_num as i32 + 1
    }

    fn stack_output(&self) -> i32 {
        1
    }

    fn println(&self, index: usize, depth: usize, debug: &mut dyn FnMut(String)) {
        PrintlnUtils::println_by_cur_depth(
            depth as i32,
            &format!("{}: Call with argNum {}", index, self.arg_num),
            debug,
        );
    }

    fn error_reporter(&self) -> &Rc<dyn ErrorReporter> {
        &self.error_reporter
    }
}

/// Operation: call ql function from function table
/// Input: ${argNum}
/// Output: 1 function result
///
/// Mirrors Java `CallFunctionInstruction`.
pub struct CallFunctionInstruction {
    error_reporter: Rc<dyn ErrorReporter>,
    function_name: String,
    arg_num: usize,
    trace_key: Option<i32>,
}

impl CallFunctionInstruction {
    pub fn new(
        error_reporter: Rc<dyn ErrorReporter>,
        function_name: impl Into<String>,
        arg_num: usize,
        trace_key: Option<i32>,
    ) -> Self {
        CallFunctionInstruction {
            error_reporter,
            function_name: function_name.into(),
            arg_num,
            trace_key,
        }
    }

    pub fn function_name(&self) -> &str {
        &self.function_name
    }

    pub fn arg_num(&self) -> usize {
        self.arg_num
    }

    pub fn trace_key(&self) -> Option<i32> {
        self.trace_key
    }

    /// Java `callLambda`: fall back to a lambda-valued variable of the same
    /// name.
    fn call_lambda(
        &self,
        q_context: &mut dyn QContext,
        ql_options: &QLOptions,
    ) -> Result<(), QLException> {
        let lambda_symbol = q_context.get_symbol_value(&self.function_name);
        let Some(lambda_symbol) = lambda_symbol else {
            if ql_options.is_avoid_null_pointer() {
                q_context.pop_n(self.arg_num);
                q_context.push(QValue::Data(DataValue::NULL_VALUE));
                return Ok(());
            }
            return Err(self.error_reporter.report_format(
                error_codes::FUNCTION_NOT_FOUND,
                error_codes::error_msg(error_codes::FUNCTION_NOT_FOUND),
                &[self.function_name.clone()],
            ));
        };
        let DataValue::Lambda(q_lambda) = &lambda_symbol else {
            return Err(self.error_reporter.report_format(
                error_codes::FUNCTION_TYPE_MISMATCH,
                error_codes::error_msg(error_codes::FUNCTION_TYPE_MISMATCH),
                &[self.function_name.clone()],
            ));
        };
        let parameters = q_context.pop_n(self.arg_num);
        let parameters_arr = parameters.values();
        match q_lambda.call(&parameters_arr) {
            Ok(result) => {
                q_context.push(QValue::Data(result.value()).to_immutable());
                Ok(())
            }
            Err(err) => Err(rethrow_call_error(
                &self.error_reporter,
                err,
                error_codes::INVOKE_LAMBDA_ERROR,
                error_codes::error_msg(error_codes::INVOKE_LAMBDA_ERROR),
                &[],
            )),
        }
    }
}

impl QLInstruction for CallFunctionInstruction {
    fn execute(
        &self,
        q_context: &mut dyn QContext,
        ql_options: &QLOptions,
    ) -> Result<QResult, QLException> {
        let function = q_context.get_function(&self.function_name);
        let Some(function) = function else {
            self.call_lambda(q_context, ql_options)?;
            return Ok(QResult::NEXT_INSTRUCTION);
        };
        let parameters = q_context.pop_n(self.arg_num);
        match function.call(q_context, &parameters) {
            Ok(function_result_obj) => {
                q_context.push(QValue::Data(function_result_obj.clone()));

                // trace
                with_trace(q_context, self.trace_key, |trace| {
                    trace.value_evaluated(function_result_obj);
                });

                Ok(QResult::NEXT_INSTRUCTION)
            }
            Err(err) => Err(rethrow_call_error(
                &self.error_reporter,
                err,
                error_codes::INVOKE_FUNCTION_INNER_ERROR,
                error_codes::error_msg(error_codes::INVOKE_FUNCTION_INNER_ERROR),
                &[
                    self.function_name.clone(),
                    err.reason().to_string(),
                ],
            )),
        }
    }

    fn stack_input(&self) -> i32 {
        self.arg_num as i32
    }

    fn stack_output(&self) -> i32 {
        1
    }

    fn println(&self, index: usize, depth: usize, debug: &mut dyn FnMut(String)) {
        PrintlnUtils::println_by_cur_depth(
            depth as i32,
            &format!("{}: CallFunction {} {}", index, self.function_name, self.arg_num),
            debug,
        );
    }

    fn error_reporter(&self) -> &Rc<dyn ErrorReporter> {
        &self.error_reporter
    }
}

/// Operation: invoke specified method of object on the top of stack
/// Input: ${argNum} + 1
/// Output: 1, method return value, null for void method
///
/// equivalent to GetMethodInstruction + CallInstruction
///
/// Mirrors Java `MethodInvokeInstruction`.
pub struct MethodInvokeInstruction {
    error_reporter: Rc<dyn ErrorReporter>,
    method_name: String,
    arg_num: usize,
    optional: bool,
}

impl MethodInvokeInstruction {
    pub fn new(
        error_reporter: Rc<dyn ErrorReporter>,
        method_name: impl Into<String>,
        arg_num: usize,
        optional: bool,
    ) -> Self {
        MethodInvokeInstruction {
            error_reporter,
            method_name: method_name.into(),
            arg_num,
            optional,
        }
    }

    pub fn method_name(&self) -> &str {
        &self.method_name
    }

    pub fn arg_num(&self) -> usize {
        self.arg_num
    }

    pub fn is_optional(&self) -> bool {
        self.optional
    }
}

impl QLInstruction for MethodInvokeInstruction {
    fn execute(
        &self,
        q_context: &mut dyn QContext,
        ql_options: &QLOptions,
    ) -> Result<QResult, QLException> {
        let parameters = q_context.pop_n(self.arg_num + 1);
        let bean = parameters.get(0).expect("bean slot popped").get();
        let params: Vec<DataValue> = (0..self.arg_num)
            .map(|i| parameters.get_value(i + 1))
            .collect();
        if bean.is_null() {
            if ql_options.is_avoid_null_pointer() || self.optional {
                q_context.push(QValue::Data(DataValue::NULL_VALUE));
                return Ok(QResult::NEXT_INSTRUCTION);
            }
            return Err(self.error_reporter.report(
                error_codes::NULL_METHOD_ACCESS,
                error_codes::error_msg(error_codes::NULL_METHOD_ACCESS),
            ));
        }
        let invoke_res = find_method_and_invoke(
            &bean,
            &self.method_name,
            &params,
            q_context.registry(),
            &*self.error_reporter,
        )?;
        q_context.push(invoke_res);
        Ok(QResult::NEXT_INSTRUCTION)
    }

    fn stack_input(&self) -> i32 {
        self.arg_num as i32 + 1
    }

    fn stack_output(&self) -> i32 {
        1
    }

    fn println(&self, index: usize, depth: usize, debug: &mut dyn FnMut(String)) {
        PrintlnUtils::println_by_cur_depth(
            depth as i32,
            &format!(
                "{}: MethodInvoke {} with argNum {}",
                index, self.method_name, self.arg_num
            ),
            debug,
        );
    }

    fn error_reporter(&self) -> &Rc<dyn ErrorReporter> {
        &self.error_reporter
    }
}

/// Operation: Invoke specified method of each object in the list
/// Input: ${argNum} + 1
/// Output: 1, a list composed of return values from methods.
///
/// Mirrors Java `SpreadMethodInvokeInstruction`.
pub struct SpreadMethodInvokeInstruction {
    error_reporter: Rc<dyn ErrorReporter>,
    method_name: String,
    arg_num: usize,
}

impl SpreadMethodInvokeInstruction {
    pub fn new(
        error_reporter: Rc<dyn ErrorReporter>,
        method_name: impl Into<String>,
        arg_num: usize,
    ) -> Self {
        SpreadMethodInvokeInstruction {
            error_reporter,
            method_name: method_name.into(),
            arg_num,
        }
    }

    pub fn method_name(&self) -> &str {
        &self.method_name
    }

    pub fn arg_num(&self) -> usize {
        self.arg_num
    }

    /// Java `isTraversable` (Iterable or array → List/Array here).
    fn is_traversable(obj: &DataValue) -> bool {
        matches!(obj, DataValue::List(_) | DataValue::Array(_))
    }

    /// Java `spreadMethodInvokeRecursive`.
    fn spread_recursive(
        &self,
        traversable: &DataValue,
        params: &[DataValue],
        q_context: &dyn QContext,
        ql_options: &QLOptions,
        result: &mut Vec<DataValue>,
    ) -> Result<(), QLException> {
        let items = match traversable {
            DataValue::List(l) => l.borrow().clone(),
            DataValue::Array(a) => a.borrow().clone(),
            _ => vec![],
        };
        for item in items {
            self.process_item(&item, params, q_context, ql_options, result)?;
        }
        Ok(())
    }

    /// Java `processItem`.
    fn process_item(
        &self,
        item: &DataValue,
        params: &[DataValue],
        q_context: &dyn QContext,
        ql_options: &QLOptions,
        result: &mut Vec<DataValue>,
    ) -> Result<(), QLException> {
        if item.is_null() {
            if ql_options.is_avoid_null_pointer() {
                result.push(DataValue::Null);
                return Ok(());
            }
            return Err(self.error_reporter.report(
                error_codes::NULL_METHOD_ACCESS,
                error_codes::error_msg(error_codes::NULL_METHOD_ACCESS),
            ));
        }

        if !Self::is_traversable(item) {
            // Leaf node - invoke method directly
            let invoke_res = find_method_and_invoke(
                item,
                &self.method_name,
                params,
                q_context.registry(),
                &*self.error_reporter,
            )?;
            result.push(invoke_res.get());
            return Ok(());
        }
        // If item itself is traversable, try to invoke method on it first
        if let Some(method) = q_context.registry().resolve_method(item, &self.method_name) {
            let invoke_res = invoke_native_method(item, &method, params)?;
            result.push(invoke_res.get());
            return Ok(());
        }
        // Then recursively flatten and invoke on nested elements
        self.spread_recursive(item, params, q_context, ql_options, result)
    }
}

impl QLInstruction for SpreadMethodInvokeInstruction {
    fn execute(
        &self,
        q_context: &mut dyn QContext,
        ql_options: &QLOptions,
    ) -> Result<QResult, QLException> {
        let parameters = q_context.pop_n(self.arg_num + 1);
        let traversable = parameters.get(0).expect("bean slot popped").get();
        if traversable.is_null() {
            if ql_options.is_avoid_null_pointer() {
                q_context.push(QValue::Data(DataValue::NULL_VALUE));
                return Ok(QResult::NEXT_INSTRUCTION);
            }
            return Err(self.error_reporter.report_format(
                error_codes::NONTRAVERSABLE_OBJECT,
                error_codes::error_msg(error_codes::NONTRAVERSABLE_OBJECT),
                &["null".to_string()],
            ));
        }
        let params: Vec<DataValue> = (0..self.arg_num)
            .map(|i| parameters.get_value(i + 1))
            .collect();

        if Self::is_traversable(&traversable) {
            let mut result = Vec::new();
            self.spread_recursive(&traversable, &params, q_context, ql_options, &mut result)?;
            q_context.push(QValue::Data(DataValue::list(result)));
        } else {
            return Err(self.error_reporter.report_format(
                error_codes::NONTRAVERSABLE_OBJECT,
                error_codes::error_msg(error_codes::NONTRAVERSABLE_OBJECT),
                &[traversable.data_type_name().to_string()],
            ));
        }
        Ok(QResult::NEXT_INSTRUCTION)
    }

    fn stack_input(&self) -> i32 {
        self.arg_num as i32 + 1
    }

    fn stack_output(&self) -> i32 {
        1
    }

    fn println(&self, index: usize, depth: usize, debug: &mut dyn FnMut(String)) {
        PrintlnUtils::println_by_cur_depth(
            depth as i32,
            &format!("{}: SpreadMethodInvoke {}", index, self.method_name),
            debug,
        );
    }

    fn error_reporter(&self) -> &Rc<dyn ErrorReporter> {
        &self.error_reporter
    }
}
