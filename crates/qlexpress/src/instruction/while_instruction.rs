//! while 循环指令,对应 Java `com.alibaba.qlexpress4.runtime.instruction.WhileInstruction`。
//! 职责:while 循环执行体。
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

/// while 循环指令。对应 Java: com.alibaba.qlexpress4.runtime.instruction.WhileInstruction(职责:while 循环执行体)
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
    /// 构造指令,对应 Java 构造器 `WhileInstruction`。
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

    /// 对应 Java 方法 `condition`。
    pub fn condition(&self) -> &Rc<dyn QLambdaDefinition> {
        &self.condition
    }

    /// 对应 Java 方法 `body`。
    pub fn body(&self) -> &Rc<dyn QLambdaDefinition> {
        &self.body
    }

    /// 对应 Java 方法 `whileScopeMaxStackSize`。
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
    /// 向下转型支持(供 api/parsecache Exporter 的 Java `instanceof` 分派)。
    fn as_any(&self) -> Option<&dyn std::any::Any> {
        Some(self)
    }

    fn execute(
        &self,
        q_context: &mut dyn QContext,
        ql_options: &QLOptions,
    ) -> Result<QResult, QLException> {
        let mut while_scope_context =
            child_fresh_scope_context(q_context, HashMap::new(), self.while_scope_max_stack_size);
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
                    ));
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

    fn compiled_instruction_count(&self) -> usize {
        1usize
            .saturating_add(self.condition.compiled_instruction_count())
            .saturating_add(self.body.compiled_instruction_count())
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
