//! Compile-time function contract, mirroring Java
//! `compiletimefunction.CompileTimeFunction`.

use crate::aparser::operator_factory::OperatorFactory;
use crate::aparser::syntax_tree_factory::Node;

use super::code_generator::CodeGenerator;

/// `CompileTimeFunction` 接口的 Rust 实现，保留对应对象的领域职责与公开契约。
/// 对应或承接 Java 源文件：`com/alibaba/qlexpress4/aparser/compiletimefunction/CompileTimeFunction.java`；具体对象路径见 `docs/对象级对照表.md`。
/// Java `CompileTimeFunction`: creates the instructions for a function
/// call at compile time.
pub trait CompileTimeFunction {
    /// 构建 function instruction。
    /// 参数：`function_name`、`arguments`、`operator_factory`、`code_generator`；返回：无。
    /// 对应或承接 Java 源文件：`com/alibaba/qlexpress4/aparser/compiletimefunction/CompileTimeFunction.java`，方法 `createFunctionInstruction`。
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
