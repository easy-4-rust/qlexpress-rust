//! Lexer/parser package mirroring Java `com.alibaba.qlexpress4.aparser`.
//!
//! Stage 0 delivered the enums/structs referenced by `InitOptions`;
//! Stage 1 added the token model ([`token`]), the scanner ([`qlexer`]) and
//! the [`parser_operator_manager`] contract. Stage 2 adds the syntax tree
//! ([`syntax_tree_factory`]), the recursive-descent parser ([`qlparser`]), the
//! compile-time visitors ([`check_visitor`], [`out_var_names_visitor`], [`out_var_attrs_visitor`],
//! [`scope_stack_visitor`]), the import resolver ([`import_manager`]),
//! macro/scope helpers and the compile cache.

pub mod check_visitor;
pub mod compile_cache;
pub mod compile_time_function;
pub mod exist_stack;
pub mod generator_scope;
pub mod import_manager;
pub mod interpolation_mode;
pub mod macro_define;
pub mod operator_factory;
pub mod out_function_visitor;
pub mod out_var_attrs_visitor;
pub mod out_var_names_visitor;
pub mod parse_tree;
pub mod parser_operator_manager;
pub mod qlexer;
pub mod qlparser;
pub mod qlparser_base_visitor;
pub mod qvm_instruction_visitor;
pub mod rule_context;
pub mod scope_stack_visitor;
pub mod syntax_tree_factory;
pub mod terminal_node;
pub mod token;
pub mod trace_expression_visitor;

pub use check_visitor::CheckVisitor;
pub use compile_cache::{CompileCache, QCompileCache, ScriptCompileCache};
pub use compile_time_function::{CodeGenerator, CompileTimeFunction};
pub use exist_stack::{ExistStack, ExistVarStack};
pub use generator_scope::GeneratorScope;
pub use import_manager::{ImportManager, ImportScope, LoadPartQualifiedResult, QLImport};
pub use interpolation_mode::InterpolationMode;
pub use macro_define::MacroDefine;
pub use operator_factory::{OperatorFactory, OperatorManager};
pub use out_function_visitor::OutFunctionVisitor;
pub use out_var_attrs_visitor::OutVarAttrsVisitor;
pub use out_var_names_visitor::OutVarNamesVisitor;
pub use parser_operator_manager::{OpType, ParserOperatorManager};
pub use qlexer::tokenize;
pub use qlparser::{build_tree, QLParser};
pub use qlparser_base_visitor::Visitor;
pub use qvm_instruction_visitor::{
    compile_script, CompileTimeFunctions, Context, InstructionMacroDefine, InstructionScope,
    QvmInstructionVisitor, SharedInstruction, UserDefineFunctions,
};
pub use rule_context::{ChildRef, HasChildren};
pub use scope_stack_visitor::{ScopeStack, ScopedVisitor};
pub use syntax_tree_factory::Node;
pub use terminal_node::TerminalNode;
pub use token::Token;
pub use trace_expression_visitor::TraceExpressionVisitor;
