//! Compile-time function contract, mirroring Java
//! `compiletimefunction.CompileTimeFunction`.

use crate::aparser::operator_factory::OperatorFactory;
use crate::aparser::syntax_tree_factory::Node;

use super::code_generator::CodeGenerator;

/// Java `CompileTimeFunction`: creates the instructions for a function
/// call at compile time.
pub trait CompileTimeFunction {
    /// Java `createFunctionInstruction(String, List<ExpressionContext>,
    /// OperatorFactory, CodeGenerator)`.
    ///
    /// * `function_name` — the called function name.
    /// * `arguments` — the argument syntax trees (Java
    ///   `ExpressionContext`s; each is a [`Node::Expression`]).
    /// * `operator_factory` — compile-time operator lookup.
    /// * `code_generator` — emission callback into the current visitor.
    fn create_function_instruction(
        &self,
        function_name: &str,
        arguments: &[&Node],
        operator_factory: &dyn OperatorFactory,
        code_generator: &mut dyn CodeGenerator,
    );
}
