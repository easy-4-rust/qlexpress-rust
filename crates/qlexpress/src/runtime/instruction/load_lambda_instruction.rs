//! Lambda 加载指令,对应 Java `com.alibaba.qlexpress4.runtime.instruction.LoadLambdaInstruction`。
//! 职责:将 Lambda 定义压栈。
//! 本文件由 `scope.rs` 拆分而来(SPEC §5.5 一类一文件),仅移动代码与补充中文注释,行为完全一致。

use crate::exception::error_reporter::ErrorReporter;
use crate::exception::QLException;
use crate::ql_options::QLOptions;
use crate::runtime::instruction::QLInstruction;
use crate::runtime::q_result::QResult;
use crate::runtime::qcontext::QContext;
use crate::runtime::qlambda_definition::QLambdaDefinition;
use crate::runtime::value::{DataValue, QValue};
use crate::utils::println_utils::PrintlnUtils;
use std::rc::Rc;

/// Lambda 加载指令。对应 Java: com.alibaba.qlexpress4.runtime.instruction.LoadLambdaInstruction(职责:将 Lambda 定义压栈)
/// Operation: instantiate lambda definition on stack
/// Input: 0
/// Output: 1 lambda instance
///
/// Mirrors Java `LoadLambdaInstruction`.
pub struct LoadLambdaInstruction {
    error_reporter: Rc<dyn ErrorReporter>,
    lambda_definition: Rc<dyn QLambdaDefinition>,
}

impl LoadLambdaInstruction {
    /// 构造指令,对应 Java 构造器 `LoadLambdaInstruction`。
    pub fn new(
        error_reporter: Rc<dyn ErrorReporter>,
        lambda_definition: Rc<dyn QLambdaDefinition>,
    ) -> Self {
        LoadLambdaInstruction {
            error_reporter,
            lambda_definition,
        }
    }

    /// 对应 Java 方法 `lambdaDefinition`。
    pub fn lambda_definition(&self) -> &Rc<dyn QLambdaDefinition> {
        &self.lambda_definition
    }
}

impl QLInstruction for LoadLambdaInstruction {
    /// 向下转型支持(供 api/parsecache Exporter 的 Java `instanceof` 分派)。
    fn as_any(&self) -> Option<&dyn std::any::Any> {
        Some(self)
    }

    fn execute(
        &self,
        q_context: &mut dyn QContext,
        ql_options: &QLOptions,
    ) -> Result<QResult, QLException> {
        let lambda_instance =
            Rc::clone(&self.lambda_definition).to_lambda(q_context, ql_options, true);
        q_context.push(QValue::Data(DataValue::Lambda(lambda_instance)));
        Ok(QResult::NEXT_INSTRUCTION)
    }

    fn stack_input(&self) -> i32 {
        0
    }

    fn stack_output(&self) -> i32 {
        1
    }

    fn compiled_instruction_count(&self) -> usize {
        1usize.saturating_add(self.lambda_definition.compiled_instruction_count())
    }

    fn println(&self, index: usize, depth: usize, debug: &mut dyn FnMut(String)) {
        PrintlnUtils::println_by_cur_depth(depth as i32, &format!("{index}: LoadLambda"), debug);
        self.lambda_definition.println(depth + 1, debug);
    }

    fn error_reporter(&self) -> &Rc<dyn ErrorReporter> {
        &self.error_reporter
    }
}
