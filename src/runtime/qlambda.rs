//! Script lambdas, mirroring Java `QLambda`, `QLambdaInner`, `QLambdaEmpty`,
//! `QLambdaDefinition`, `QLambdaDefinitionInner`, `QLambdaDefinitionEmpty`
//! and `QLambdaTrace`.

use std::collections::HashMap;
use std::fmt;
use std::rc::Rc;

use crate::exception::error_codes;
use crate::exception::error_reporter::ErrorReporter;
use crate::exception::pure_err_reporter::PureErrReporter;
use crate::exception::QLException;
use crate::ql_options::QLOptions;
use crate::ql_result::QResult;
use crate::runtime::data::convert::obj_type_convertor::{ObjTypeConvertor, TargetType};
use crate::runtime::data::lambda::QLambdaMethod;
use crate::runtime::data::AssignableDataValue;
use crate::runtime::delegate_qcontext::DelegateQContext;
use crate::runtime::function::CustomFunction;
use crate::runtime::instruction::Instruction;
use crate::runtime::left_value::LeftValue;
use crate::runtime::qcontext::QContext;
use crate::runtime::qvm_runtime::run_instructions;
use crate::runtime::scope::{Scope, SymbolTable};
use crate::runtime::trace::QTraces;
use crate::runtime::value::DataValue;
use crate::utils::println_utils::PrintlnUtils;

/// Lambda parameter declaration, mirroring Java
/// `QLambdaDefinitionInner.Param`. `clazz == None` mirrors a `null`
/// `Class<?>` (untyped parameter, Java `Object`).
#[derive(Clone, Debug)]
pub struct Param {
    name: String,
    clazz: Option<TargetType>,
}

impl Param {
    pub fn new(name: impl Into<String>, clazz: Option<TargetType>) -> Self {
        Param {
            name: name.into(),
            clazz,
        }
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn clazz(&self) -> Option<TargetType> {
        self.clazz
    }
}

/// A lambda definition (compile-time), mirroring Java `QLambdaDefinition`.
pub trait QLambdaDefinition {
    /// Java `toLambda(QContext, QLOptions, boolean newEnv)`.
    fn to_lambda(
        self: Rc<Self>,
        q_context: &mut dyn QContext,
        ql_options: &QLOptions,
        new_env: bool,
    ) -> Rc<QLambda>;

    /// Java `println(int depth, Consumer<String> debug)`.
    fn println(&self, depth: usize, debug: &mut dyn FnMut(String));

    /// Java `getName()`.
    fn name(&self) -> &str;
}

/// Lambda defined by an instruction sequence, mirroring Java
/// `QLambdaDefinitionInner`.
pub struct QLambdaDefinitionInner {
    /// Function name.
    name: String,
    instructions: Vec<Instruction>,
    params_type: Vec<Param>,
    max_stack_size: usize,
}

impl QLambdaDefinitionInner {
    pub fn new(
        name: impl Into<String>,
        instructions: Vec<Instruction>,
        params_type: Vec<Param>,
        max_stack_size: usize,
    ) -> Self {
        QLambdaDefinitionInner {
            name: name.into(),
            instructions,
            params_type,
            max_stack_size,
        }
    }

    pub fn instructions(&self) -> &[Instruction] {
        &self.instructions
    }

    pub fn params_type(&self) -> &[Param] {
        &self.params_type
    }

    pub fn max_stack_size(&self) -> usize {
        self.max_stack_size
    }
}

impl QLambdaDefinition for QLambdaDefinitionInner {
    /// Java `toLambda`: captures the *current scope* of `qContext` in a new
    /// `DelegateQContext` (this is what makes closures and recursive
    /// self-references work — the function table of the defining scope is
    /// reachable from the lambda body).
    fn to_lambda(
        self: Rc<Self>,
        q_context: &mut dyn QContext,
        ql_options: &QLOptions,
        new_env: bool,
    ) -> Rc<QLambda> {
        Rc::new(QLambda::Inner(QLambdaInner::new(
            self,
            DelegateQContext::new(
                Rc::clone(q_context.q_runtime()),
                q_context.current_scope(),
            ),
            ql_options.clone(),
            new_env,
        )))
    }

    fn println(&self, depth: usize, debug: &mut dyn FnMut(String)) {
        for (i, instruction) in self.instructions.iter().enumerate() {
            instruction.println(i, depth, debug);
        }
    }

    fn name(&self) -> &str {
        &self.name
    }
}

/// Java `QLambdaDefinitionEmpty.INSTANCE`.
pub struct QLambdaDefinitionEmpty;

impl QLambdaDefinitionEmpty {
    /// Java `QLambdaDefinitionEmpty.INSTANCE`.
    pub const INSTANCE: QLambdaDefinitionEmpty = QLambdaDefinitionEmpty;
}

impl QLambdaDefinition for QLambdaDefinitionEmpty {
    fn to_lambda(
        self: Rc<Self>,
        _q_context: &mut dyn QContext,
        _ql_options: &QLOptions,
        _new_env: bool,
    ) -> Rc<QLambda> {
        Rc::new(QLambda::Empty)
    }

    fn println(&self, depth: usize, debug: &mut dyn FnMut(String)) {
        PrintlnUtils::println_by_cur_depth(depth as i32, self.name(), debug);
    }

    fn name(&self) -> &str {
        "EmptyLambdaDefinition"
    }
}

/// A callable script lambda value, mirroring the Java `QLambda` interface
/// and its implementations (`QLambdaEmpty`, `QLambdaInner`, and the
/// method-reference lambda `QLambdaMethod`).
pub enum QLambda {
    /// Java `QLambdaEmpty.INSTANCE`: calling it yields
    /// `QResult.NEXT_INSTRUCTION`.
    Empty,
    /// Java `QLambdaInner`: instruction-sequence lambda.
    Inner(QLambdaInner),
    /// Java `data/lambda/QLambdaMethod`: object method as a lambda.
    Method(QLambdaMethod),
}

impl QLambda {
    /// Java `QLambda.call(Object... params)`.
    pub fn call(&self, params: &[DataValue]) -> Result<QResult, QLException> {
        match self {
            QLambda::Empty => Ok(QResult::NEXT_INSTRUCTION),
            QLambda::Inner(inner) => inner.call(params),
            QLambda::Method(method) => method.call(params),
        }
    }

    /// Java `QLambda.getFunctionDefined(Object... params)`.
    pub fn function_defined(
        &self,
        params: &[DataValue],
    ) -> Result<HashMap<String, Rc<dyn CustomFunction>>, QLException> {
        match self {
            QLambda::Inner(inner) => inner.function_defined(params),
            _ => Ok(HashMap::new()),
        }
    }
}

impl fmt::Debug for QLambda {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            QLambda::Empty => write!(f, "QLambdaEmpty"),
            QLambda::Inner(inner) => f
                .debug_struct("QLambdaInner")
                .field("name", &inner.definition.name())
                .field("params", &inner.definition.params_type())
                .field("new_env", &inner.new_env)
                .finish(),
            QLambda::Method(method) => write!(f, "QLambdaMethod({})", method.method_name()),
        }
    }
}

/// Instruction-sequence lambda, mirroring Java `QLambdaInner`.
pub struct QLambdaInner {
    lambda_definition: Rc<QLambdaDefinitionInner>,
    q_context: DelegateQContext,
    ql_options: QLOptions,
    new_env: bool,
}

impl QLambdaInner {
    pub fn new(
        lambda_definition: Rc<QLambdaDefinitionInner>,
        q_context: DelegateQContext,
        ql_options: QLOptions,
        new_env: bool,
    ) -> Self {
        QLambdaInner {
            lambda_definition,
            q_context,
            ql_options,
            new_env,
        }
    }

    pub fn definition(&self) -> &Rc<QLambdaDefinitionInner> {
        &self.lambda_definition
    }

    /// Java `call(Object... params)`.
    pub fn call(&self, params: &[DataValue]) -> Result<QResult, QLException> {
        let mut runtime = if self.new_env {
            self.inherit_scope(params)?
        } else {
            // DelegateQContext is a pair of Rc handles; cloning shares the
            // same runtime and scope chain (Java mutates its own wrapper).
            DelegateQContext::new(
                Rc::clone(self.q_context.q_runtime()),
                self.q_context.current_scope(),
            )
        };
        self.call_inner(&mut runtime)
    }

    /// Java `getFunctionDefined(Object... params)`.
    pub fn function_defined(
        &self,
        params: &[DataValue],
    ) -> Result<HashMap<String, Rc<dyn CustomFunction>>, QLException> {
        let mut new_runtime = if self.new_env {
            self.inherit_scope(params)?
        } else {
            DelegateQContext::new(
                Rc::clone(self.q_context.q_runtime()),
                self.q_context.current_scope(),
            )
        };
        self.call_inner(&mut new_runtime)?;
        Ok(new_runtime.function_table())
    }

    /// Java `callInner`: the fetch-execute loop (see
    /// [`run_instructions`]).
    fn call_inner(&self, runtime: &mut dyn QContext) -> Result<QResult, QLException> {
        run_instructions(runtime, &self.lambda_definition.instructions, &self.ql_options)
    }

    /// Java `inheritScope`: bind parameters (converted to their declared
    /// types, `null` for missing ones) in a fresh block scope whose parent
    /// is the captured definition scope.
    fn inherit_scope(&self, params: &[DataValue]) -> Result<DelegateQContext, QLException> {
        let params_definition = &self.lambda_definition.params_type;
        let mut init_symbol_table: SymbolTable = HashMap::with_capacity(params.len());
        for (i, param_definition) in params_definition.iter().enumerate().take(params.len()) {
            let origin_param_i = &params[i];
            let target_cls = param_definition.clazz;
            let ql_convert_result = ObjTypeConvertor::cast_opt(origin_param_i, target_cls);
            if !ql_convert_result.is_convertible() {
                // Java: UserDefineException(INVALID_ARGUMENT, ...)
                let message = format!(
                    "invalid argument at index {} (start from 0), required type {}, but {} provided",
                    i,
                    target_cls
                        .map(TargetType::java_name)
                        .unwrap_or("java.lang.Object"),
                    if origin_param_i.is_null() {
                        "null".to_string()
                    } else {
                        origin_param_i.data_type_name().to_string()
                    }
                );
                return Err(PureErrReporter::INSTANCE.report(error_codes::INVALID_ARGUMENT, &message));
            }
            let slot: Rc<std::cell::RefCell<dyn LeftValue>> = match target_cls {
                Some(clz) => Rc::new(std::cell::RefCell::new(AssignableDataValue::with_type(
                    param_definition.name(),
                    ql_convert_result.into_converted(),
                    clz,
                ))),
                None => Rc::new(std::cell::RefCell::new(AssignableDataValue::new(
                    param_definition.name(),
                    ql_convert_result.into_converted(),
                ))),
            };
            init_symbol_table.insert(param_definition.name().to_string(), slot);
        }
        // null for rest params
        for param_definition in params_definition.iter().skip(params.len()) {
            let slot: Rc<std::cell::RefCell<dyn LeftValue>> = match param_definition.clazz {
                Some(clz) => Rc::new(std::cell::RefCell::new(AssignableDataValue::with_type(
                    param_definition.name(),
                    DataValue::Null,
                    clz,
                ))),
                None => Rc::new(std::cell::RefCell::new(AssignableDataValue::new(
                    param_definition.name(),
                    DataValue::Null,
                ))),
            };
            init_symbol_table.insert(param_definition.name().to_string(), slot);
        }
        let new_scope = Scope::block_fresh_stack(
            &self.q_context.current_scope(),
            init_symbol_table,
        );
        Ok(DelegateQContext::new(
            Rc::clone(self.q_context.q_runtime()),
            new_scope,
        ))
    }
}

/// Lambda plus the traces captured when it was produced, mirroring Java
/// `QLambdaTrace`.
pub struct QLambdaTrace {
    q_lambda: Rc<QLambda>,
    traces: QTraces,
}

impl QLambdaTrace {
    pub fn new(q_lambda: Rc<QLambda>, traces: QTraces) -> Self {
        QLambdaTrace { q_lambda, traces }
    }

    /// Java `getqLambda()`.
    pub fn q_lambda(&self) -> &Rc<QLambda> {
        &self.q_lambda
    }

    /// Java `getTraces()`.
    pub fn traces(&self) -> &QTraces {
        &self.traces
    }
}
