//! Control-flow instructions, mirroring Java `JumpInstruction`,
//! `JumpIfInstruction`, `JumpIfPopInstruction`, `ReturnInstruction`,
//! `BreakContinueInstruction`, `PopInstruction`, `ThrowInstruction`,
//! `CheckTimeOutInstruction`, `ForInstruction`, `ForEachInstruction`,
//! `WhileInstruction`, `TryCatchInstruction`.

use std::cell::Cell;
use std::collections::HashMap;
use std::rc::Rc;

use crate::exception::error_codes;
use crate::exception::error_reporter::ErrorReporter;
use crate::exception::QLException;
use crate::ql_options::QLOptions;
use crate::ql_result::QResult;
use crate::runtime::delegate_qcontext::DelegateQContext;
use crate::runtime::instruction::{with_trace, QLInstruction};
use crate::runtime::member::ClassRef;
use crate::runtime::qcontext::QContext;
use crate::runtime::qlambda::{QLambda, QLambdaDefinition};
use crate::runtime::qvm_runtime::current_time_millis;
use crate::runtime::scope::Scope;
use crate::runtime::util::throw_utils::wrap_throwable;
use crate::runtime::value::{DataValue, QValue};
use crate::utils::println_utils::PrintlnUtils;

/// Java `QResult.ResultType` values usable by `ReturnInstruction`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ReturnResultType {
    /// `QResult.ResultType.RETURN`
    Return,
    /// `QResult.ResultType.BREAK`
    Break,
    /// `QResult.ResultType.CONTINUE`
    Continue,
}

impl ReturnResultType {
    fn build(self, value: DataValue) -> QResult {
        match self {
            ReturnResultType::Return => QResult::Return(value),
            ReturnResultType::Break => QResult::Break,
            ReturnResultType::Continue => QResult::Continue,
        }
    }
}

/// Operation: jump to a position
/// Input: 0
/// Output: 0
///
/// Mirrors Java `JumpInstruction` (position is a relative offset, applied
/// by the VM loop with `i += position`).
pub struct JumpInstruction {
    error_reporter: Rc<dyn ErrorReporter>,
    position: Cell<i32>,
}

impl JumpInstruction {
    pub fn new(error_reporter: Rc<dyn ErrorReporter>, position: i32) -> Self {
        JumpInstruction {
            error_reporter,
            position: Cell::new(position),
        }
    }

    pub fn set_position(&self, position: i32) {
        self.position.set(position);
    }

    pub fn position(&self) -> i32 {
        self.position.get()
    }
}

impl QLInstruction for JumpInstruction {
    fn execute(
        &self,
        _q_context: &mut dyn QContext,
        _ql_options: &QLOptions,
    ) -> Result<QResult, QLException> {
        Ok(QResult::Jump(self.position.get()))
    }

    fn stack_input(&self) -> i32 {
        0
    }

    fn stack_output(&self) -> i32 {
        0
    }

    fn println(&self, index: usize, depth: usize, debug: &mut dyn FnMut(String)) {
        PrintlnUtils::println_by_cur_depth(
            depth as i32,
            &format!("{}: Jump {}", index, self.position.get()),
            debug,
        );
    }

    fn error_reporter(&self) -> &Rc<dyn ErrorReporter> {
        &self.error_reporter
    }
}

/// Operation: if the element is ${expect}, jump to position, else execute
/// next instruction as normal. not jump if null
/// Input: 0
/// Output: 0
///
/// Mirrors Java `JumpIfInstruction`.
pub struct JumpIfInstruction {
    error_reporter: Rc<dyn ErrorReporter>,
    expect: bool,
    position: Cell<i32>,
    trace_key: Option<i32>,
}

impl JumpIfInstruction {
    pub fn new(
        error_reporter: Rc<dyn ErrorReporter>,
        expect: bool,
        position: i32,
        trace_key: Option<i32>,
    ) -> Self {
        JumpIfInstruction {
            error_reporter,
            expect,
            position: Cell::new(position),
            trace_key,
        }
    }

    pub fn set_position(&self, position: i32) {
        self.position.set(position);
    }

    pub fn is_expect(&self) -> bool {
        self.expect
    }

    pub fn position(&self) -> i32 {
        self.position.get()
    }

    pub fn trace_key(&self) -> Option<i32> {
        self.trace_key
    }

    /// Java `conditionToBool`: `null` becomes `!expect`, non-Boolean is an
    /// error.
    fn condition_to_bool(&self, condition: &DataValue) -> Result<bool, QLException> {
        if condition.is_null() {
            return Ok(!self.expect);
        }
        match condition {
            DataValue::Bool(b) => Ok(*b),
            _ => Err(self.error_reporter.report(
                error_codes::CONDITION_BOOL_REQUIRED,
                error_codes::error_msg(error_codes::CONDITION_BOOL_REQUIRED),
            )),
        }
    }
}

impl QLInstruction for JumpIfInstruction {
    fn execute(
        &self,
        q_context: &mut dyn QContext,
        ql_options: &QLOptions,
    ) -> Result<QResult, QLException> {
        let condition_bool = self.condition_to_bool(&q_context.peek().get())?;
        if condition_bool == self.expect && !ql_options.is_short_circuit_disable() {
            // short circuit; trace
            with_trace(q_context, self.trace_key, |trace| {
                trace.value_evaluated(DataValue::Bool(condition_bool));
                if let Some(child) = trace.children_mut().get_mut(0) {
                    child.value_evaluated(DataValue::Bool(condition_bool));
                }
            });
            Ok(QResult::Jump(self.position.get()))
        } else {
            Ok(QResult::NEXT_INSTRUCTION)
        }
    }

    fn stack_input(&self) -> i32 {
        0
    }

    fn stack_output(&self) -> i32 {
        0
    }

    fn println(&self, index: usize, depth: usize, debug: &mut dyn FnMut(String)) {
        PrintlnUtils::println_by_cur_depth(
            depth as i32,
            &format!("{}: JumpIf {} {}", index, self.expect, self.position.get()),
            debug,
        );
    }

    fn error_reporter(&self) -> &Rc<dyn ErrorReporter> {
        &self.error_reporter
    }
}

/// Operation: pop the top of stack, if the element is ${expect}, jump to
/// position, else execute next instruction as normal. jump if null
/// Input: 1
/// Output: 0
///
/// Mirrors Java `JumpIfPopInstruction`.
pub struct JumpIfPopInstruction {
    error_reporter: Rc<dyn ErrorReporter>,
    expect: bool,
    position: Cell<i32>,
}

impl JumpIfPopInstruction {
    pub fn new(error_reporter: Rc<dyn ErrorReporter>, expect: bool, position: i32) -> Self {
        JumpIfPopInstruction {
            error_reporter,
            expect,
            position: Cell::new(position),
        }
    }

    pub fn set_position(&self, position: i32) {
        self.position.set(position);
    }

    pub fn is_expect(&self) -> bool {
        self.expect
    }

    pub fn position(&self) -> i32 {
        self.position.get()
    }

    /// Java `conditionToBool`: `null` becomes `expect`, non-Boolean errors.
    fn condition_to_bool(&self, condition: &DataValue) -> Result<bool, QLException> {
        if condition.is_null() {
            return Ok(self.expect);
        }
        match condition {
            DataValue::Bool(b) => Ok(*b),
            _ => Err(self.error_reporter.report(
                error_codes::CONDITION_BOOL_REQUIRED,
                error_codes::error_msg(error_codes::CONDITION_BOOL_REQUIRED),
            )),
        }
    }
}

impl QLInstruction for JumpIfPopInstruction {
    fn execute(
        &self,
        q_context: &mut dyn QContext,
        _ql_options: &QLOptions,
    ) -> Result<QResult, QLException> {
        let condition_bool = self.condition_to_bool(&q_context.pop().get())?;
        if condition_bool == self.expect {
            Ok(QResult::Jump(self.position.get()))
        } else {
            Ok(QResult::NEXT_INSTRUCTION)
        }
    }

    fn stack_input(&self) -> i32 {
        1
    }

    fn stack_output(&self) -> i32 {
        0
    }

    fn println(&self, index: usize, depth: usize, debug: &mut dyn FnMut(String)) {
        PrintlnUtils::println_by_cur_depth(
            depth as i32,
            &format!("{}: JumpIfPop {} {}", index, self.expect, self.position.get()),
            debug,
        );
    }

    fn error_reporter(&self) -> &Rc<dyn ErrorReporter> {
        &self.error_reporter
    }
}

/// Operation: return top element and exit lambda
/// Input: 1
/// Output: 0
///
/// Mirrors Java `ReturnInstruction`.
pub struct ReturnInstruction {
    error_reporter: Rc<dyn ErrorReporter>,
    result_type: ReturnResultType,
    trace_key: Option<i32>,
}

impl ReturnInstruction {
    pub fn new(
        error_reporter: Rc<dyn ErrorReporter>,
        result_type: ReturnResultType,
        trace_key: Option<i32>,
    ) -> Self {
        ReturnInstruction {
            error_reporter,
            result_type,
            trace_key,
        }
    }

    pub fn result_type(&self) -> ReturnResultType {
        self.result_type
    }

    pub fn trace_key(&self) -> Option<i32> {
        self.trace_key
    }
}

impl QLInstruction for ReturnInstruction {
    fn execute(
        &self,
        q_context: &mut dyn QContext,
        _ql_options: &QLOptions,
    ) -> Result<QResult, QLException> {
        let return_value = q_context.pop();
        // Java traces only when traceKey != null (with_trace handles it).
        with_trace(q_context, self.trace_key, |trace| {
            trace.value_evaluated(return_value.get());
        });
        Ok(self.result_type.build(return_value.get()))
    }

    fn stack_input(&self) -> i32 {
        1
    }

    fn stack_output(&self) -> i32 {
        0
    }

    fn println(&self, index: usize, depth: usize, debug: &mut dyn FnMut(String)) {
        PrintlnUtils::println_by_cur_depth(depth as i32, &format!("{index}: Return"), debug);
    }

    fn error_reporter(&self) -> &Rc<dyn ErrorReporter> {
        &self.error_reporter
    }
}

/// Operation: return break object and exit lambda
/// Input: 0
/// Output: 0
///
/// Mirrors Java `BreakContinueInstruction`.
pub struct BreakContinueInstruction {
    error_reporter: Rc<dyn ErrorReporter>,
    /// `true` = break (Java `QResult.LOOP_BREAK_RESULT`), `false` =
    /// continue (`QResult.LOOP_CONTINUE_RESULT`).
    is_break: bool,
}

impl BreakContinueInstruction {
    /// Java `new BreakContinueInstruction(errorReporter, result)` where
    /// `result` is `LOOP_BREAK_RESULT` or `LOOP_CONTINUE_RESULT`.
    pub fn new(error_reporter: Rc<dyn ErrorReporter>, is_break: bool) -> Self {
        BreakContinueInstruction {
            error_reporter,
            is_break,
        }
    }

    /// Java `getResult()`.
    pub fn result(&self) -> QResult {
        if self.is_break {
            QResult::LOOP_BREAK_RESULT
        } else {
            QResult::LOOP_CONTINUE_RESULT
        }
    }

    pub fn is_break(&self) -> bool {
        self.is_break
    }
}

impl QLInstruction for BreakContinueInstruction {
    fn execute(
        &self,
        _q_context: &mut dyn QContext,
        _ql_options: &QLOptions,
    ) -> Result<QResult, QLException> {
        Ok(self.result())
    }

    fn stack_input(&self) -> i32 {
        0
    }

    fn stack_output(&self) -> i32 {
        0
    }

    fn println(&self, index: usize, depth: usize, debug: &mut dyn FnMut(String)) {
        let break_continue = if self.is_break { "Break" } else { "Continue" };
        PrintlnUtils::println_by_cur_depth(
            depth as i32,
            &format!("{index}: {break_continue}"),
            debug,
        );
    }

    fn error_reporter(&self) -> &Rc<dyn ErrorReporter> {
        &self.error_reporter
    }
}

/// Operation: pop top element
/// Input: 1
/// Output: 0
///
/// Mirrors Java `PopInstruction`.
pub struct PopInstruction {
    error_reporter: Rc<dyn ErrorReporter>,
}

impl PopInstruction {
    pub fn new(error_reporter: Rc<dyn ErrorReporter>) -> Self {
        PopInstruction { error_reporter }
    }
}

impl QLInstruction for PopInstruction {
    fn execute(
        &self,
        q_context: &mut dyn QContext,
        _ql_options: &QLOptions,
    ) -> Result<QResult, QLException> {
        q_context.pop();
        Ok(QResult::NEXT_INSTRUCTION)
    }

    fn stack_input(&self) -> i32 {
        1
    }

    fn stack_output(&self) -> i32 {
        0
    }

    fn println(&self, index: usize, depth: usize, debug: &mut dyn FnMut(String)) {
        PrintlnUtils::println_by_cur_depth(depth as i32, &format!("{index}: Pop"), debug);
    }

    fn error_reporter(&self) -> &Rc<dyn ErrorReporter> {
        &self.error_reporter
    }
}

/// Operation: throw top element on the stack
/// Input: 1
/// Output: 0
///
/// Mirrors Java `ThrowInstruction`.
pub struct ThrowInstruction {
    error_reporter: Rc<dyn ErrorReporter>,
}

impl ThrowInstruction {
    pub fn new(error_reporter: Rc<dyn ErrorReporter>) -> Self {
        ThrowInstruction { error_reporter }
    }
}

impl QLInstruction for ThrowInstruction {
    fn execute(
        &self,
        q_context: &mut dyn QContext,
        _ql_options: &QLOptions,
    ) -> Result<QResult, QLException> {
        let throw_obj = q_context.pop().get();
        Err(self.error_reporter.report_with_catch(
            Some(throw_obj),
            error_codes::QL_THROW,
            error_codes::error_msg(error_codes::QL_THROW),
        ))
    }

    fn stack_input(&self) -> i32 {
        1
    }

    fn stack_output(&self) -> i32 {
        0
    }

    fn println(&self, index: usize, depth: usize, debug: &mut dyn FnMut(String)) {
        PrintlnUtils::println_by_cur_depth(depth as i32, &format!("{index}: Throw"), debug);
    }

    fn error_reporter(&self) -> &Rc<dyn ErrorReporter> {
        &self.error_reporter
    }
}

/// Operation: check if program timeout
/// Input: 0
/// Output: 0
///
/// Mirrors Java `CheckTimeOutInstruction`.
pub struct CheckTimeOutInstruction {
    error_reporter: Rc<dyn ErrorReporter>,
}

impl CheckTimeOutInstruction {
    pub fn new(error_reporter: Rc<dyn ErrorReporter>) -> Self {
        CheckTimeOutInstruction { error_reporter }
    }
}

impl QLInstruction for CheckTimeOutInstruction {
    fn execute(
        &self,
        q_context: &mut dyn QContext,
        ql_options: &QLOptions,
    ) -> Result<QResult, QLException> {
        if ql_options.timeout_millis() <= 0 {
            return Ok(QResult::NEXT_INSTRUCTION);
        }
        if current_time_millis() - q_context.script_start_time_stamp() > ql_options.timeout_millis()
        {
            // timeout
            return Err(self.error_reporter.report_format(
                error_codes::SCRIPT_TIME_OUT,
                error_codes::error_msg(error_codes::SCRIPT_TIME_OUT),
                &[ql_options.timeout_millis().to_string()],
            ));
        }
        Ok(QResult::NEXT_INSTRUCTION)
    }

    fn stack_input(&self) -> i32 {
        0
    }

    fn stack_output(&self) -> i32 {
        0
    }

    fn println(&self, index: usize, depth: usize, debug: &mut dyn FnMut(String)) {
        PrintlnUtils::println_by_cur_depth(depth as i32, &format!("{index}: CheckTimeout"), debug);
    }

    fn error_reporter(&self) -> &Rc<dyn ErrorReporter> {
        &self.error_reporter
    }
}

/// Helper: build a delegate context whose current scope is a fresh-stack
/// block scope over the current scope (Java `new DelegateQContext(qContext,
/// new QvmBlockScope(qContext, symbols, maxStackSize, ExceptionTable.EMPTY))`).
fn child_fresh_scope_context(
    q_context: &mut dyn QContext,
    symbols: crate::runtime::scope::SymbolTable,
) -> DelegateQContext {
    let scope = Scope::block_fresh_stack(&q_context.current_scope(), symbols);
    DelegateQContext::new(Rc::clone(q_context.q_runtime()), scope)
}

/// Helper: delegate context sharing the current scope (Java
/// `new DelegateQContext(qContext, qContext.getCurrentScope())`).
fn delegate_current(q_context: &mut dyn QContext) -> DelegateQContext {
    DelegateQContext::new(
        Rc::clone(q_context.q_runtime()),
        q_context.current_scope(),
    )
}

/// Operation: traditional for loop
/// Input: 0
/// Output: 0
///
/// Mirrors Java `ForInstruction`.
pub struct ForInstruction {
    error_reporter: Rc<dyn ErrorReporter>,
    /// nullable
    for_init: Option<Rc<dyn QLambdaDefinition>>,
    /// nullable
    condition: Option<Rc<dyn QLambdaDefinition>>,
    condition_error_reporter: Rc<dyn ErrorReporter>,
    /// nullable
    for_update: Option<Rc<dyn QLambdaDefinition>>,
    for_scope_max_stack_size: usize,
    for_body: Rc<dyn QLambdaDefinition>,
}

impl ForInstruction {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        error_reporter: Rc<dyn ErrorReporter>,
        for_init: Option<Rc<dyn QLambdaDefinition>>,
        condition: Option<Rc<dyn QLambdaDefinition>>,
        condition_error_reporter: Rc<dyn ErrorReporter>,
        for_update: Option<Rc<dyn QLambdaDefinition>>,
        for_scope_max_stack_size: usize,
        for_body: Rc<dyn QLambdaDefinition>,
    ) -> Self {
        ForInstruction {
            error_reporter,
            for_init,
            condition,
            condition_error_reporter,
            for_update,
            for_scope_max_stack_size,
            for_body,
        }
    }

    pub fn for_init(&self) -> Option<&Rc<dyn QLambdaDefinition>> {
        self.for_init.as_ref()
    }

    pub fn condition(&self) -> Option<&Rc<dyn QLambdaDefinition>> {
        self.condition.as_ref()
    }

    pub fn condition_error_reporter(&self) -> &Rc<dyn ErrorReporter> {
        &self.condition_error_reporter
    }

    pub fn for_update(&self) -> Option<&Rc<dyn QLambdaDefinition>> {
        self.for_update.as_ref()
    }

    pub fn for_scope_max_stack_size(&self) -> usize {
        self.for_scope_max_stack_size
    }

    pub fn for_body(&self) -> &Rc<dyn QLambdaDefinition> {
        &self.for_body
    }

    /// Java `needForScope()`.
    fn need_for_scope(&self) -> bool {
        self.for_init.is_some() || self.condition.is_some() || self.for_update.is_some()
    }

    /// Java `evalCondition`.
    fn eval_condition(&self, condition_lambda: &QLambda) -> Result<bool, QLException> {
        match condition_lambda.call(&[]) {
            Ok(result) => match result.value() {
                DataValue::Bool(b) => Ok(b),
                _ => Err(self.condition_error_reporter.report(
                    error_codes::FOR_CONDITION_BOOL_REQUIRED,
                    error_codes::error_msg(error_codes::FOR_CONDITION_BOOL_REQUIRED),
                )),
            },
            Err(err) => Err(wrap_throwable(
                err,
                &*self.condition_error_reporter,
                error_codes::FOR_CONDITION_ERROR,
                error_codes::error_msg(error_codes::FOR_CONDITION_ERROR),
                &[],
            )),
        }
    }

    /// Java `runUpdate`.
    fn run_update(&self, update_lambda: &QLambda) -> Result<(), QLException> {
        update_lambda.call(&[]).map(|_| ()).map_err(|err| {
            wrap_throwable(
                err,
                &*self.error_reporter,
                error_codes::FOR_UPDATE_ERROR,
                error_codes::error_msg(error_codes::FOR_UPDATE_ERROR),
                &[],
            )
        })
    }
}

impl QLInstruction for ForInstruction {
    fn execute(
        &self,
        q_context: &mut dyn QContext,
        ql_options: &QLOptions,
    ) -> Result<QResult, QLException> {
        let mut for_scope_context = if self.need_for_scope() {
            child_fresh_scope_context(q_context, HashMap::with_capacity(1))
        } else {
            delegate_current(q_context)
        };
        if let Some(for_init) = &self.for_init {
            let init_lambda = Rc::clone(for_init).to_lambda(&mut for_scope_context, ql_options, false);
            init_lambda.call(&[]).map_err(|err| {
                wrap_throwable(
                    err,
                    &*self.error_reporter,
                    error_codes::FOR_INIT_ERROR,
                    error_codes::error_msg(error_codes::FOR_INIT_ERROR),
                    &[],
                )
            })?;
        }

        let condition_lambda = self
            .condition
            .as_ref()
            .map(|c| Rc::clone(c).to_lambda(&mut for_scope_context, ql_options, false));
        let update_lambda = self
            .for_update
            .as_ref()
            .map(|u| Rc::clone(u).to_lambda(&mut for_scope_context, ql_options, false));
        let body_lambda = Rc::clone(&self.for_body).to_lambda(&mut for_scope_context, ql_options, true);

        // forBody:
        while match &condition_lambda {
            Some(condition) => self.eval_condition(condition)?,
            None => true,
        } {
            match body_lambda.call(&[]) {
                Ok(body_result) => match body_result {
                    QResult::Return(_) => return Ok(body_result),
                    QResult::Break => break,
                    _ => {}
                },
                Err(err) => {
                    return Err(wrap_throwable(
                        err,
                        &*self.error_reporter,
                        error_codes::FOR_BODY_ERROR,
                        error_codes::error_msg(error_codes::FOR_BODY_ERROR),
                        &[],
                    ))
                }
            }
            if let Some(update) = &update_lambda {
                self.run_update(update)?;
            }
        }
        Ok(QResult::NEXT_INSTRUCTION)
    }

    fn stack_input(&self) -> i32 {
        0
    }

    fn stack_output(&self) -> i32 {
        0
    }

    fn println(&self, index: usize, depth: usize, debug: &mut dyn FnMut(String)) {
        PrintlnUtils::println_by_cur_depth(depth as i32, &format!("{index}: For"), debug);
        PrintlnUtils::println_by_cur_depth(depth as i32 + 1, "Init", debug);
        if let Some(for_init) = &self.for_init {
            for_init.println(depth + 2, debug);
        }
        PrintlnUtils::println_by_cur_depth(depth as i32 + 1, "Condition", debug);
        if let Some(condition) = &self.condition {
            condition.println(depth + 2, debug);
        }
        PrintlnUtils::println_by_cur_depth(depth as i32 + 1, "Update", debug);
        if let Some(for_update) = &self.for_update {
            for_update.println(depth + 2, debug);
        }
        PrintlnUtils::println_by_cur_depth(depth as i32 + 1, "Body", debug);
        self.for_body.println(depth + 2, debug);
    }

    fn error_reporter(&self) -> &Rc<dyn ErrorReporter> {
        &self.error_reporter
    }
}

/// Operation: process each element in iterable object on top of stack,
/// Input: 1
/// Output: 0
///
/// Mirrors Java `ForEachInstruction`.
pub struct ForEachInstruction {
    error_reporter: Rc<dyn ErrorReporter>,
    body: Rc<dyn QLambdaDefinition>,
    target_error_reporter: Rc<dyn ErrorReporter>,
    it_cls: ClassRef,
}

impl ForEachInstruction {
    pub fn new(
        error_reporter: Rc<dyn ErrorReporter>,
        body: Rc<dyn QLambdaDefinition>,
        it_cls: ClassRef,
        target_error_reporter: Rc<dyn ErrorReporter>,
    ) -> Self {
        ForEachInstruction {
            error_reporter,
            body,
            target_error_reporter,
            it_cls,
        }
    }

    pub fn body(&self) -> &Rc<dyn QLambdaDefinition> {
        &self.body
    }

    pub fn target_error_reporter(&self) -> &Rc<dyn ErrorReporter> {
        &self.target_error_reporter
    }

    pub fn it_cls(&self) -> &ClassRef {
        &self.it_cls
    }
}

impl QLInstruction for ForEachInstruction {
    fn execute(
        &self,
        q_context: &mut dyn QContext,
        ql_options: &QLOptions,
    ) -> Result<QResult, QLException> {
        let may_be_iterable = q_context.pop().get();
        // Java: array → ReflectArrayIterable; Iterable → as-is; else error.
        let items: Vec<DataValue> = match &may_be_iterable {
            DataValue::Array(arr) => arr.borrow().clone(),
            DataValue::List(list) => list.borrow().clone(),
            _ => {
                return Err(self.target_error_reporter.report(
                    error_codes::FOR_EACH_ITERABLE_REQUIRED,
                    error_codes::error_msg(error_codes::FOR_EACH_ITERABLE_REQUIRED),
                ))
            }
        };
        let body_lambda = Rc::clone(&self.body).to_lambda(q_context, ql_options, true);
        // forEachBody:
        for item in items {
            match body_lambda.call(std::slice::from_ref(&item)) {
                Ok(body_result) => match body_result {
                    QResult::Return(_) => return Ok(body_result),
                    QResult::Break => break,
                    _ => {}
                },
                Err(err) => {
                    // Java: UserDefineException (lambda argument conversion)
                    // → FOR_EACH_TYPE_MISMATCH; QLRuntimeException → rethrow;
                    // else FOR_EACH_UNKNOWN_ERROR.
                    if err.error_code() == error_codes::INVALID_ARGUMENT {
                        return Err(self.error_reporter.report_format(
                            error_codes::FOR_EACH_TYPE_MISMATCH,
                            error_codes::error_msg(error_codes::FOR_EACH_TYPE_MISMATCH),
                            &[
                                self.it_cls.java_name().to_string(),
                                if item.is_null() {
                                    "null".to_string()
                                } else {
                                    item.data_type_name().to_string()
                                },
                            ],
                        ));
                    }
                    return Err(err);
                }
            }
        }
        Ok(QResult::NEXT_INSTRUCTION)
    }

    fn stack_input(&self) -> i32 {
        1
    }

    fn stack_output(&self) -> i32 {
        0
    }

    fn println(&self, index: usize, depth: usize, debug: &mut dyn FnMut(String)) {
        PrintlnUtils::println_by_cur_depth(depth as i32, &format!("{index}: ForEach"), debug);
        self.body.println(depth + 1, debug);
    }

    fn error_reporter(&self) -> &Rc<dyn ErrorReporter> {
        &self.error_reporter
    }
}

/// Operation: while (condition) do body
/// Input: 0
/// Output: 0
///
/// Mirrors Java `WhileInstruction`.
pub struct WhileInstruction {
    error_reporter: Rc<dyn ErrorReporter>,
    condition: Rc<dyn QLambdaDefinition>,
    body: Rc<dyn QLambdaDefinition>,
    while_scope_max_stack_size: usize,
}

impl WhileInstruction {
    pub fn new(
        error_reporter: Rc<dyn ErrorReporter>,
        condition: Rc<dyn QLambdaDefinition>,
        body: Rc<dyn QLambdaDefinition>,
        while_scope_max_stack_size: usize,
    ) -> Self {
        WhileInstruction {
            error_reporter,
            condition,
            body,
            while_scope_max_stack_size,
        }
    }

    pub fn condition(&self) -> &Rc<dyn QLambdaDefinition> {
        &self.condition
    }

    pub fn body(&self) -> &Rc<dyn QLambdaDefinition> {
        &self.body
    }

    pub fn while_scope_max_stack_size(&self) -> usize {
        self.while_scope_max_stack_size
    }

    /// Java `evalCondition`.
    fn eval_condition(&self, condition_lambda: &QLambda) -> Result<bool, QLException> {
        match condition_lambda.call(&[]) {
            Ok(result) => match result.value() {
                DataValue::Bool(b) => Ok(b),
                _ => Err(self.error_reporter.report(
                    error_codes::WHILE_CONDITION_BOOL_REQUIRED,
                    error_codes::error_msg(error_codes::WHILE_CONDITION_BOOL_REQUIRED),
                )),
            },
            Err(err) => Err(wrap_throwable(
                err,
                &*self.error_reporter,
                error_codes::WHILE_CONDITION_ERROR,
                error_codes::error_msg(error_codes::WHILE_CONDITION_ERROR),
                &[],
            )),
        }
    }
}

impl QLInstruction for WhileInstruction {
    fn execute(
        &self,
        q_context: &mut dyn QContext,
        ql_options: &QLOptions,
    ) -> Result<QResult, QLException> {
        let mut while_scope_context = child_fresh_scope_context(q_context, HashMap::new());
        let condition_lambda =
            Rc::clone(&self.condition).to_lambda(&mut while_scope_context, ql_options, false);
        let body_lambda =
            Rc::clone(&self.body).to_lambda(&mut while_scope_context, ql_options, true);
        // whileBody:
        while self.eval_condition(&condition_lambda)? {
            match body_lambda.call(&[]) {
                Ok(body_result) => match body_result {
                    QResult::Return(_) => return Ok(body_result),
                    QResult::Break => break,
                    _ => {}
                },
                Err(err) => {
                    // Java hardcodes this code/message (not in QLErrorCodes).
                    return Err(wrap_throwable(
                        err,
                        &*self.error_reporter,
                        "WHILE_BODY_EXECUTE_ERROR",
                        "while body execute error",
                        &[],
                    ))
                }
            }
        }
        Ok(QResult::NEXT_INSTRUCTION)
    }

    fn stack_input(&self) -> i32 {
        0
    }

    fn stack_output(&self) -> i32 {
        0
    }

    fn println(&self, index: usize, depth: usize, debug: &mut dyn FnMut(String)) {
        PrintlnUtils::println_by_cur_depth(depth as i32, &format!("{index}: While"), debug);
        PrintlnUtils::println_by_cur_depth(depth as i32 + 1, "Condition", debug);
        self.condition.println(depth + 2, debug);
        PrintlnUtils::println_by_cur_depth(depth as i32 + 1, "Body", debug);
        self.body.println(depth + 2, debug);
    }

    fn error_reporter(&self) -> &Rc<dyn ErrorReporter> {
        &self.error_reporter
    }
}

/// Operation: try and catch throw element
/// Input: 0
/// Output: 1
///
/// Mirrors Java `TryCatchInstruction`. Catch entries are keyed by
/// [`ClassRef`] (Java `Class<?>`); matching follows Java
/// `Class.isAssignableFrom` for the built-in exception hierarchy plus
/// `java.lang.Object` (matches everything) — see Stage-3a notes.
pub struct TryCatchInstruction {
    error_reporter: Rc<dyn ErrorReporter>,
    body: Rc<dyn QLambdaDefinition>,
    exception_table: Vec<(ClassRef, Rc<dyn QLambdaDefinition>)>,
    /// nullable
    final_body: Option<Rc<dyn QLambdaDefinition>>,
}

impl TryCatchInstruction {
    pub fn new(
        error_reporter: Rc<dyn ErrorReporter>,
        body: Rc<dyn QLambdaDefinition>,
        exception_table: Vec<(ClassRef, Rc<dyn QLambdaDefinition>)>,
        final_body: Option<Rc<dyn QLambdaDefinition>>,
    ) -> Self {
        TryCatchInstruction {
            error_reporter,
            body,
            exception_table,
            final_body,
        }
    }

    pub fn body(&self) -> &Rc<dyn QLambdaDefinition> {
        &self.body
    }

    pub fn exception_table(&self) -> &[(ClassRef, Rc<dyn QLambdaDefinition>)] {
        &self.exception_table
    }

    pub fn final_body(&self) -> Option<&Rc<dyn QLambdaDefinition>> {
        self.final_body.as_ref()
    }

    /// Java `shouldExitTryCatch`.
    fn should_exit_try_catch(result: &QResult) -> bool {
        matches!(
            result,
            QResult::Return(_) | QResult::Break | QResult::Continue
        )
    }

    /// Java `getExceptionHandler(Class)`:
    /// `entry.getKey().isAssignableFrom(catchObjClass)`.
    fn get_exception_handler(&self, catch_obj: Option<&DataValue>) -> Option<&Rc<dyn QLambdaDefinition>> {
        let catch_type = match catch_obj {
            // Java substitutes `new Object()` for a null catch object.
            None => "java.lang.Object",
            Some(value) => value.data_type_name(),
        };
        self.exception_table
            .iter()
            .find(|(clz, _)| class_assignable_from(clz, catch_type))
            .map(|(_, handler)| handler)
    }

    /// Java `callExceptionHandler`.
    fn call_exception_handler(
        &self,
        catch_obj: Option<&DataValue>,
        q_context: &mut dyn QContext,
        ql_options: &QLOptions,
    ) -> Result<Option<QResult>, QLException> {
        let exception_handler = self.get_exception_handler(catch_obj);
        let Some(handler) = exception_handler else {
            return Ok(None);
        };
        let catch_handler_lambda = Rc::clone(handler).to_lambda(q_context, ql_options, true);
        let arg = catch_obj.cloned().unwrap_or(DataValue::Null);
        match catch_handler_lambda.call(std::slice::from_ref(&arg)) {
            Ok(result) => Ok(Some(result)),
            Err(err) => Err(wrap_throwable(
                err,
                &*self.error_reporter,
                error_codes::EXECUTE_CATCH_HANDLER_ERROR,
                error_codes::error_msg(error_codes::EXECUTE_CATCH_HANDLER_ERROR),
                &[],
            )),
        }
    }

    /// Java `tryCatchResult`.
    fn try_catch_result(
        &self,
        q_context: &mut dyn QContext,
        ql_options: &QLOptions,
    ) -> Result<QResult, QLException> {
        let body_lambda = Rc::clone(&self.body).to_lambda(q_context, ql_options, true);
        match body_lambda.call(&[]) {
            Ok(result) => Ok(result),
            Err(err) => {
                let handled =
                    self.call_exception_handler(err.catch_obj(), q_context, ql_options)?;
                match handled {
                    Some(result) => Ok(result),
                    // Java: QLRuntimeException with no matching handler →
                    // rethrow as-is; other Throwables → EXECUTE_TRY_BLOCK_ERROR.
                    None => Err(err),
                }
            }
        }
    }

    /// Java `callFinal`.
    fn call_final(
        &self,
        final_body: &Rc<dyn QLambdaDefinition>,
        q_context: &mut dyn QContext,
        ql_options: &QLOptions,
    ) -> Result<(), QLException> {
        let final_lambda = Rc::clone(final_body).to_lambda(q_context, ql_options, true);
        final_lambda.call(&[]).map(|_| ()).map_err(|err| {
            wrap_throwable(
                err,
                &*self.error_reporter,
                error_codes::EXECUTE_FINAL_BLOCK_ERROR,
                error_codes::error_msg(error_codes::EXECUTE_FINAL_BLOCK_ERROR),
                &[],
            )
        })
    }
}

impl QLInstruction for TryCatchInstruction {
    fn execute(
        &self,
        q_context: &mut dyn QContext,
        ql_options: &QLOptions,
    ) -> Result<QResult, QLException> {
        let try_catch_result = self.try_catch_result(q_context, ql_options)?;
        let result_value = try_catch_result.value();
        q_context.push(QValue::Data(result_value).to_immutable());

        if let Some(final_body) = &self.final_body {
            self.call_final(final_body, q_context, ql_options)?;
        }
        if Self::should_exit_try_catch(&try_catch_result) {
            return Ok(try_catch_result);
        }
        Ok(QResult::NEXT_INSTRUCTION)
    }

    fn stack_input(&self) -> i32 {
        0
    }

    fn stack_output(&self) -> i32 {
        1
    }

    fn println(&self, index: usize, depth: usize, debug: &mut dyn FnMut(String)) {
        PrintlnUtils::println_by_cur_depth(depth as i32, &format!("{index}: TryCatch"), debug);
        PrintlnUtils::println_by_cur_depth(depth as i32 + 1, "Body", debug);
        self.body.println(depth + 2, debug);
        for (clz, handler) in &self.exception_table {
            PrintlnUtils::println_by_cur_depth(depth as i32 + 1, clz.simple_name(), debug);
            handler.println(depth + 2, debug);
        }
        if let Some(final_body) = &self.final_body {
            PrintlnUtils::println_by_cur_depth(depth as i32 + 1, "Finally", debug);
            final_body.println(depth + 2, debug);
        }
    }

    fn error_reporter(&self) -> &Rc<dyn ErrorReporter> {
        &self.error_reporter
    }
}

/// Java `Class.isAssignableFrom` for the built-in type/exception hierarchy
/// used by catch matching. `java.lang.Object` is assignable from anything;
/// the exception supertypes match their subclasses; otherwise exact name.
fn class_assignable_from(entry: &ClassRef, catch_type: &str) -> bool {
    let entry_name = entry.java_name();
    if entry_name == catch_type || entry_name == "java.lang.Object" {
        return true;
    }
    let catch_is_exception = catch_type.contains("Exception") || catch_type.contains("Throwable");
    match entry_name {
        "java.lang.Throwable" => catch_is_exception || catch_type.contains("Error"),
        "java.lang.Exception" => catch_is_exception,
        "java.lang.RuntimeException" => {
            catch_type.contains("RuntimeException")
                || catch_type == "com.alibaba.qlexpress4.exception.QLRuntimeException"
        }
        _ => false,
    }
}
