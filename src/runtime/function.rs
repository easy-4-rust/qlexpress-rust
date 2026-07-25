//! Script-callable functions, mirroring Java
//! `com.alibaba.qlexpress4.runtime.function` (minimal surface for Stage 3a;
//! `custom_function.rs`/`extension_function.rs` land in Stage 5).

use std::rc::Rc;

use crate::exception::QLException;
use crate::runtime::parameters::Parameters;
use crate::runtime::qcontext::QContext;
use crate::runtime::qlambda::QLambda;
use crate::runtime::value::DataValue;

/// A function callable from scripts, mirroring Java `CustomFunction`.
pub trait CustomFunction {
    /// Java `call(QContext, Parameters)`; returns the function result
    /// (Java `Object`).
    fn call(
        &self,
        q_context: &mut dyn QContext,
        parameters: &Parameters,
    ) -> Result<DataValue, QLException>;

    /// Downcast hook for [`LazyArgCustomFunction`] (Java uses
    /// `customFunction instanceof LazyArgCustomFunction` in
    /// `QvmInstructionVisitor.visitCallFunction`).
    fn as_lazy_arg(&self) -> Option<&dyn LazyArgCustomFunction> {
        None
    }
}

/// A custom function whose chosen arguments are compiled into lambdas
/// (lazy evaluation), mirroring Java `LazyArgCustomFunction`.
pub trait LazyArgCustomFunction: CustomFunction {
    /// Java `isLazyArg(int)`: whether the `index`-th argument should be
    /// compiled as a lambda instead of being evaluated eagerly.
    fn is_lazy_arg(&self, index: usize) -> bool;
}

/// Adapts a [`QLambda`] into a [`CustomFunction`], mirroring Java
/// `QLambdaFunction` (used by `DefineFunctionInstruction`).
pub struct QLambdaFunction {
    q_lambda: Rc<QLambda>,
}

impl QLambdaFunction {
    pub fn new(q_lambda: Rc<QLambda>) -> Self {
        QLambdaFunction { q_lambda }
    }
}

impl CustomFunction for QLambdaFunction {
    fn call(
        &self,
        _q_context: &mut dyn QContext,
        parameters: &Parameters,
    ) -> Result<DataValue, QLException> {
        // Java: collect parameters.get(i).get() then qLambda.call(...).getResult().get()
        let params_arr = parameters.values();
        Ok(self.q_lambda.call(&params_arr)?.value())
    }
}
