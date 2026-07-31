//! 函数定义指令,对应 Java `com.alibaba.qlexpress4.runtime.instruction.DefineFunctionInstruction`。
//! 职责:在当前作用域定义函数。
//! 本文件由 `scope.rs` 拆分而来(SPEC §5.5 一类一文件),仅移动代码与补充中文注释,行为完全一致。

use crate::exception::QLException;
use crate::exception::error_reporter::ErrorReporter;
use crate::ql_options::QLOptions;
use crate::runtime::function::QLambdaFunction;
use crate::runtime::instruction::QLInstruction;
use crate::runtime::q_result::QResult;
use crate::runtime::qcontext::QContext;
use crate::runtime::qlambda_definition::QLambdaDefinition;
use crate::utils::println_utils::PrintlnUtils;
use std::rc::Rc;

/// 函数定义指令。对应 Java: com.alibaba.qlexpress4.runtime.instruction.DefineFunctionInstruction(职责:在当前作用域定义函数)
/// Operation: define function
/// Input: 0
/// Output: 0
///
/// Mirrors Java `DefineFunctionInstruction`.
pub struct DefineFunctionInstruction {
    error_reporter: Rc<dyn ErrorReporter>,
    name: String,
    lambda_definition: Rc<dyn QLambdaDefinition>,
}

impl DefineFunctionInstruction {
    /// 构造指令,对应 Java 构造器 `DefineFunctionInstruction`。
    pub fn new(
        error_reporter: Rc<dyn ErrorReporter>,
        name: impl Into<String>,
        lambda_definition: Rc<dyn QLambdaDefinition>,
    ) -> Self {
        DefineFunctionInstruction {
            error_reporter,
            name: name.into(),
            lambda_definition,
        }
    }

    /// 对应 Java 方法 `name`。
    pub fn name(&self) -> &str {
        &self.name
    }

    /// 对应 Java 方法 `lambdaDefinition`。
    pub fn lambda_definition(&self) -> &Rc<dyn QLambdaDefinition> {
        &self.lambda_definition
    }
}

impl QLInstruction for DefineFunctionInstruction {
    /// 向下转型支持(供 api/parsecache Exporter 的 Java `instanceof` 分派)。
    fn as_any(&self) -> Option<&dyn std::any::Any> {
        Some(self)
    }

    fn execute(
        &self,
        q_context: &mut dyn QContext,
        ql_options: &QLOptions,
    ) -> Result<QResult, QLException> {
        // Java: lambda captures the defining scope, so a function can call
        // itself recursively through the scope's own function table.
        let lambda = Rc::clone(&self.lambda_definition).to_lambda(q_context, ql_options, true);
        q_context.define_function(&self.name, Rc::new(QLambdaFunction::new(lambda)));
        Ok(QResult::NEXT_INSTRUCTION)
    }

    fn stack_input(&self) -> i32 {
        0
    }

    fn stack_output(&self) -> i32 {
        0
    }

    fn compiled_instruction_count(&self) -> usize {
        1usize.saturating_add(self.lambda_definition.compiled_instruction_count())
    }

    fn println(&self, index: usize, depth: usize, debug: &mut dyn FnMut(String)) {
        PrintlnUtils::println_by_cur_depth(
            depth as i32,
            &format!("{}: DefineFunction {}", index, self.name),
            debug,
        );
        self.lambda_definition.println(depth + 1, debug);
    }

    fn error_reporter(&self) -> &Rc<dyn ErrorReporter> {
        &self.error_reporter
    }
}
