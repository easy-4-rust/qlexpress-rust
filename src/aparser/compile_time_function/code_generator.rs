//! Code-generation callback handed to compile-time functions, mirroring
//! Java `compiletimefunction.CodeGenerator`.

use std::rc::Rc;

use crate::aparser::syntax_tree_factory::Node;
use crate::aparser::token::Token;
use crate::exception::error_reporter::ErrorReporter;
use crate::exception::ql_syntax_exception::QLSyntaxException;
use crate::runtime::instruction::Instruction;
use crate::runtime::qlambda_definition::QLambdaDefinition;
use crate::runtime::qlambda_definition_inner::Param;

/// Java `CodeGenerator`: the tool `QvmInstructionVisitor` passes to a
/// [`CompileTimeFunction`](super::compile_time_function::CompileTimeFunction)
/// so it can emit instructions during compilation.
pub trait CodeGenerator {
    /// Java `addInstruction(QLInstruction)`.
    fn add_instruction(&mut self, instruction: Instruction);

    /// Java `addInstructionsByTree(ParseTree)`: compile a syntax subtree
    /// with the current visitor.
    fn add_instructions_by_tree(&mut self, tree: &Node);

    /// Java `reportParseErr(String, String)`; records the syntax error on
    /// the owning visitor (Java throws it) and returns it.
    fn report_parse_err(&mut self, err_code: &str, err_reason: &str) -> QLSyntaxException;

    /// Java `generateLambdaDefinition(ExpressionContext, List<Param>)`:
    /// compile an expression body into a lambda definition.
    fn generate_lambda_definition(
        &mut self,
        expression: &Node,
        params: Vec<Param>,
    ) -> Rc<dyn QLambdaDefinition>;

    /// Java `getErrorReporter()`: reporter bound to the function name
    /// token.
    fn error_reporter(&self) -> Rc<dyn ErrorReporter>;

    /// Java `newReporterWithToken(Token)`.
    fn new_reporter_with_token(&self, token: &Token) -> Rc<dyn ErrorReporter>;
}
