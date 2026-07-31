//! for 循环指令,对应 Java `com.alibaba.qlexpress4.runtime.instruction.ForInstruction`。
//! 职责:经典 for 循环执行体。
//! 本文件由 `flow.rs` 拆分而来(SPEC §5.5 一类一文件),仅移动代码与补充中文注释,行为完全一致。

use crate::exception::QLException;
use crate::exception::error_codes;
use crate::exception::error_reporter::ErrorReporter;
use crate::ql_options::QLOptions;
use crate::runtime::delegate_qcontext::DelegateQContext;
use crate::runtime::instruction::QLInstruction;
use crate::runtime::q_result::QResult;
use crate::runtime::qcontext::QContext;
use crate::runtime::qlambda::QLambda;
use crate::runtime::qlambda_definition::QLambdaDefinition;
use crate::runtime::scope::QScope;
use crate::runtime::util::throw_utils::wrap_throwable;
use crate::runtime::value::DataValue;
use crate::utils::println_utils::PrintlnUtils;
use std::collections::HashMap;
use std::rc::Rc;

/// for 循环指令。对应 Java: com.alibaba.qlexpress4.runtime.instruction.ForInstruction(职责:经典 for 循环执行体)
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
    /// 构造实例。对应 Java 源码 `com/alibaba/qlexpress4/runtime/instruction/ForInstruction.java:43` 的 `ForInstruction::<init>`。
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

    /// 对应 Java 方法 `forInit`。
    pub fn for_init(&self) -> Option<&Rc<dyn QLambdaDefinition>> {
        self.for_init.as_ref()
    }

    /// 对应 Java 方法 `condition`。
    pub fn condition(&self) -> Option<&Rc<dyn QLambdaDefinition>> {
        self.condition.as_ref()
    }

    /// 对应 Java 方法 `conditionErrorReporter`。
    pub fn condition_error_reporter(&self) -> &Rc<dyn ErrorReporter> {
        &self.condition_error_reporter
    }

    /// 对应 Java 方法 `forUpdate`。
    pub fn for_update(&self) -> Option<&Rc<dyn QLambdaDefinition>> {
        self.for_update.as_ref()
    }

    /// 对应 Java 方法 `forScopeMaxStackSize`。
    pub fn for_scope_max_stack_size(&self) -> usize {
        self.for_scope_max_stack_size
    }

    /// 对应 Java 方法 `forBody`。
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
    /// 向下转型支持(供 api/parsecache Exporter 的 Java `instanceof` 分派)。
    fn as_any(&self) -> Option<&dyn std::any::Any> {
        Some(self)
    }

    fn execute(
        &self,
        q_context: &mut dyn QContext,
        ql_options: &QLOptions,
    ) -> Result<QResult, QLException> {
        let mut for_scope_context = if self.need_for_scope() {
            child_fresh_scope_context(
                q_context,
                HashMap::with_capacity(1),
                self.for_scope_max_stack_size,
            )
        } else {
            delegate_current(q_context)
        };
        if let Some(for_init) = &self.for_init {
            let init_lambda =
                Rc::clone(for_init).to_lambda(&mut for_scope_context, ql_options, false);
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
        let body_lambda =
            Rc::clone(&self.for_body).to_lambda(&mut for_scope_context, ql_options, true);

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
                    ));
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

    fn compiled_instruction_count(&self) -> usize {
        [
            self.for_init.as_ref(),
            self.condition.as_ref(),
            self.for_update.as_ref(),
            Some(&self.for_body),
        ]
        .into_iter()
        .flatten()
        .fold(1usize, |total, definition| {
            total.saturating_add(definition.compiled_instruction_count())
        })
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

/// Helper: build a delegate context whose current scope is a fresh-stack
/// block scope over the current scope (Java `new DelegateQContext(qContext,
/// new QvmBlockScope(qContext, symbols, maxStackSize, ExceptionTable.EMPTY))`).
fn child_fresh_scope_context(
    q_context: &mut dyn QContext,
    symbols: crate::runtime::scope::SymbolTable,
    max_stack_size: usize,
) -> DelegateQContext {
    let scope = QScope::block_fresh_stack(&q_context.current_scope(), symbols, max_stack_size);
    DelegateQContext::new(Rc::clone(q_context.q_runtime()), scope)
}

/// Helper: delegate context sharing the current scope (Java
/// `new DelegateQContext(qContext, qContext.getCurrentScope())`).
fn delegate_current(q_context: &mut dyn QContext) -> DelegateQContext {
    DelegateQContext::new(Rc::clone(q_context.q_runtime()), q_context.current_scope())
}
