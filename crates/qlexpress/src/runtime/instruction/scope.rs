//! Scope/variable instructions, mirroring Java `LoadInstruction`,
//! `LoadLambdaInstruction`, `DefineLocalInstruction`,
//! `DefineFunctionInstruction`, `NewScopeInstruction`,
//! `CloseScopeInstruction`.

use std::rc::Rc;

use crate::exception::error_codes;
use crate::exception::error_reporter::ErrorReporter;
use crate::exception::QLException;
use crate::ql_options::QLOptions;
use crate::ql_result::QResult;
use crate::runtime::data::convert::obj_type_convertor::{ObjTypeConvertor, TargetType};
use crate::runtime::function::QLambdaFunction;
use crate::runtime::instruction::{with_trace, QLInstruction};
use crate::runtime::qcontext::QContext;
use crate::runtime::qlambda::QLambdaDefinition;
use crate::runtime::value::{DataValue, QValue};
use crate::utils::println_utils::PrintlnUtils;

/// Operation: load variable from local to global scope, create when not exist
/// Input: 0
/// Output: 1 left value of local variable
///
/// Mirrors Java `LoadInstruction`.
pub struct LoadInstruction {
    error_reporter: Rc<dyn ErrorReporter>,
    name: String,
    trace_key: Option<i32>,
}

impl LoadInstruction {
    pub fn new(error_reporter: Rc<dyn ErrorReporter>, name: impl Into<String>, trace_key: Option<i32>) -> Self {
        LoadInstruction {
            error_reporter,
            name: name.into(),
            trace_key,
        }
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn trace_key(&self) -> Option<i32> {
        self.trace_key
    }
}

impl QLInstruction for LoadInstruction {
    fn execute(
        &self,
        q_context: &mut dyn QContext,
        _ql_options: &QLOptions,
    ) -> Result<QResult, QLException> {
        let symbol_value = q_context
            .get_symbol(&self.name)
            .expect("global scope always creates symbols");
        let evaluated = symbol_value.borrow().get();
        q_context.push(QValue::Left(symbol_value));

        // trace
        with_trace(q_context, self.trace_key, |trace| {
            trace.value_evaluated(evaluated);
        });

        Ok(QResult::NEXT_INSTRUCTION)
    }

    fn stack_input(&self) -> i32 {
        0
    }

    fn stack_output(&self) -> i32 {
        1
    }

    fn println(&self, index: usize, depth: usize, debug: &mut dyn FnMut(String)) {
        PrintlnUtils::println_by_cur_depth(
            depth as i32,
            &format!("{}: Load {}", index, self.name),
            debug,
        );
    }

    fn error_reporter(&self) -> &Rc<dyn ErrorReporter> {
        &self.error_reporter
    }
}

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
    pub fn new(
        error_reporter: Rc<dyn ErrorReporter>,
        lambda_definition: Rc<dyn QLambdaDefinition>,
    ) -> Self {
        LoadLambdaInstruction {
            error_reporter,
            lambda_definition,
        }
    }

    pub fn lambda_definition(&self) -> &Rc<dyn QLambdaDefinition> {
        &self.lambda_definition
    }
}

impl QLInstruction for LoadLambdaInstruction {
    fn execute(
        &self,
        q_context: &mut dyn QContext,
        ql_options: &QLOptions,
    ) -> Result<QResult, QLException> {
        let lambda_instance = Rc::clone(&self.lambda_definition).to_lambda(q_context, ql_options, true);
        q_context.push(QValue::Data(DataValue::Lambda(lambda_instance)));
        Ok(QResult::NEXT_INSTRUCTION)
    }

    fn stack_input(&self) -> i32 {
        0
    }

    fn stack_output(&self) -> i32 {
        1
    }

    fn println(&self, index: usize, depth: usize, debug: &mut dyn FnMut(String)) {
        PrintlnUtils::println_by_cur_depth(depth as i32, &format!("{index}: LoadLambda"), debug);
        self.lambda_definition.println(depth + 1, debug);
    }

    fn error_reporter(&self) -> &Rc<dyn ErrorReporter> {
        &self.error_reporter
    }
}

/// Operation: define a symbol in local scope
/// Input: 1 symbol init value
/// Output: 0
///
/// Mirrors Java `DefineLocalInstruction`.
pub struct DefineLocalInstruction {
    error_reporter: Rc<dyn ErrorReporter>,
    variable_name: String,
    define_clz: Option<TargetType>,
}

impl DefineLocalInstruction {
    pub fn new(
        error_reporter: Rc<dyn ErrorReporter>,
        variable_name: impl Into<String>,
        define_clz: Option<TargetType>,
    ) -> Self {
        DefineLocalInstruction {
            error_reporter,
            variable_name: variable_name.into(),
            define_clz,
        }
    }

    pub fn variable_name(&self) -> &str {
        &self.variable_name
    }

    pub fn define_clz(&self) -> Option<TargetType> {
        self.define_clz
    }
}

impl QLInstruction for DefineLocalInstruction {
    fn execute(
        &self,
        q_context: &mut dyn QContext,
        _ql_options: &QLOptions,
    ) -> Result<QResult, QLException> {
        let init_value = q_context.pop().get();
        let ql_convert_result = ObjTypeConvertor::cast_opt(&init_value, self.define_clz);
        if !ql_convert_result.is_convertible() {
            // Java reportFormat(INCOMPATIBLE_ASSIGNMENT_TYPE, msg,
            //   defineClz.getName(), initValue class name)
            return Err(self.error_reporter.report_format(
                error_codes::INCOMPATIBLE_ASSIGNMENT_TYPE,
                error_codes::error_msg(error_codes::INCOMPATIBLE_ASSIGNMENT_TYPE),
                &[
                    self.define_clz
                        .map(TargetType::java_name)
                        .unwrap_or("java.lang.Object")
                        .to_string(),
                    if init_value.is_null() {
                        "null".to_string()
                    } else {
                        init_value.data_type_name().to_string()
                    },
                ],
            ));
        }
        q_context.define_local_symbol(
            &self.variable_name,
            self.define_clz,
            ql_convert_result.into_converted(),
        );
        Ok(QResult::NEXT_INSTRUCTION)
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
            &format!("{}: DefineLocal {}", index, self.variable_name),
            debug,
        );
    }

    fn error_reporter(&self) -> &Rc<dyn ErrorReporter> {
        &self.error_reporter
    }
}

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

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn lambda_definition(&self) -> &Rc<dyn QLambdaDefinition> {
        &self.lambda_definition
    }
}

impl QLInstruction for DefineFunctionInstruction {
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

/// Operation: new scope
/// Input: 0
/// Output: 0
///
/// Mirrors Java `NewScopeInstruction`.
pub struct NewScopeInstruction {
    error_reporter: Rc<dyn ErrorReporter>,
    scope_name: String,
}

impl NewScopeInstruction {
    pub fn new(error_reporter: Rc<dyn ErrorReporter>, scope_name: impl Into<String>) -> Self {
        NewScopeInstruction {
            error_reporter,
            scope_name: scope_name.into(),
        }
    }

    pub fn scope_name(&self) -> &str {
        &self.scope_name
    }
}

impl QLInstruction for NewScopeInstruction {
    fn execute(
        &self,
        q_context: &mut dyn QContext,
        _ql_options: &QLOptions,
    ) -> Result<QResult, QLException> {
        q_context.new_scope();
        Ok(QResult::NEXT_INSTRUCTION)
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
            &format!("{}: NewScope {}", index, self.scope_name),
            debug,
        );
    }

    fn error_reporter(&self) -> &Rc<dyn ErrorReporter> {
        &self.error_reporter
    }
}

/// Operation: close scope
/// Input: 0
/// Output: 0
///
/// Mirrors Java `CloseScopeInstruction`.
pub struct CloseScopeInstruction {
    error_reporter: Rc<dyn ErrorReporter>,
    scope_name: String,
}

impl CloseScopeInstruction {
    pub fn new(error_reporter: Rc<dyn ErrorReporter>, scope_name: impl Into<String>) -> Self {
        CloseScopeInstruction {
            error_reporter,
            scope_name: scope_name.into(),
        }
    }

    pub fn scope_name(&self) -> &str {
        &self.scope_name
    }
}

impl QLInstruction for CloseScopeInstruction {
    fn execute(
        &self,
        q_context: &mut dyn QContext,
        _ql_options: &QLOptions,
    ) -> Result<QResult, QLException> {
        q_context.close_scope();
        Ok(QResult::NEXT_INSTRUCTION)
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
            &format!("{}: CloseScope {}", index, self.scope_name),
            debug,
        );
    }

    fn error_reporter(&self) -> &Rc<dyn ErrorReporter> {
        &self.error_reporter
    }
}
