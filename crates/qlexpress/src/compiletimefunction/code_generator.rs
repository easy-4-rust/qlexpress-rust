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

/// `CodeGenerator` 接口的 Rust 实现，保留对应对象的领域职责与公开契约。
/// 对应或承接 Java 源文件：`com/alibaba/qlexpress4/aparser/compiletimefunction/CodeGenerator.java`；具体对象路径见 `docs/对象级对照表.md`。
/// Java `CodeGenerator`: the tool `QvmInstructionVisitor` passes to a
/// [`CompileTimeFunction`](super::CompileTimeFunction)
/// so it can emit instructions during compilation.
/// 对应 Java: com.alibaba.qlexpress4.aparser.compiletimefunction.CodeGenerator。
pub trait CodeGenerator {
    /// 添加或注册 instruction。
    /// 参数：`instruction`；返回：无。
    /// 对应或承接 Java 源文件：`com/alibaba/qlexpress4/aparser/compiletimefunction/CodeGenerator.java`，方法 `addInstruction`。
    /// Java `addInstruction(QLInstruction)`.
    fn add_instruction(&mut self, instruction: Instruction);

    /// 添加或注册 instructions by tree。
    /// 参数：`tree`；返回：无。
    /// 对应或承接 Java 源文件：`com/alibaba/qlexpress4/aparser/compiletimefunction/CodeGenerator.java`，方法 `addInstructionsByTree`。
    /// Java `addInstructionsByTree(ParseTree)`: compile a syntax subtree
    /// with the current visitor.
    fn add_instructions_by_tree(&mut self, tree: &Node);

    /// 校验或报告 parse err。
    /// 参数：`err_code`、`err_reason`；返回：`QLSyntaxException`。
    /// 对应或承接 Java 源文件：`com/alibaba/qlexpress4/aparser/compiletimefunction/CodeGenerator.java`，方法 `reportParseErr`。
    /// Java `reportParseErr(String, String)`; records the syntax error on
    /// the owning visitor (Java throws it) and returns it.
    fn report_parse_err(&mut self, err_code: &str, err_reason: &str) -> QLSyntaxException;

    /// 构建 lambda definition。
    /// 参数：`expression`、`params`；返回：`Rc<dyn QLambdaDefinition>`。
    /// 对应或承接 Java 源文件：`com/alibaba/qlexpress4/aparser/compiletimefunction/CodeGenerator.java`，方法 `generateLambdaDefinition`。
    /// Java `generateLambdaDefinition(ExpressionContext, List<Param>)`:
    /// compile an expression body into a lambda definition.
    fn generate_lambda_definition(
        &mut self,
        expression: &Node,
        params: Vec<Param>,
    ) -> Rc<dyn QLambdaDefinition>;

    /// 处理 error reporter 对应的接口职责。
    /// 无显式参数；返回：`Rc<dyn ErrorReporter>`。
    /// 对应或承接 Java 源文件：`com/alibaba/qlexpress4/aparser/compiletimefunction/CodeGenerator.java`，方法 `errorReporter`。
    /// Java `getErrorReporter()`: reporter bound to the function name
    /// token.
    fn error_reporter(&self) -> Rc<dyn ErrorReporter>;

    /// 处理 new reporter with token 对应的接口职责。
    /// 参数：`token`；返回：`Rc<dyn ErrorReporter>`。
    /// 对应或承接 Java 源文件：`com/alibaba/qlexpress4/aparser/compiletimefunction/CodeGenerator.java`，方法 `newReporterWithToken`。
    /// Java `newReporterWithToken(Token)`.
    fn new_reporter_with_token(&self, token: &Token) -> Rc<dyn ErrorReporter>;
}
