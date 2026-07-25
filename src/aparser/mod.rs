//! Lexer/parser package mirroring Java `com.alibaba.qlexpress4.aparser`.
//!
//! Stage 0 delivered the enums/structs referenced by `InitOptions`;
//! Stage 1 added the token model ([`token`]), the scanner ([`qlexer`]) and
//! the [`parser_operator_manager`] contract. Stage 2 adds the syntax tree
//! ([`syntax_tree`]), the recursive-descent parser ([`qlparser`]), the
//! compile-time visitors ([`check_visitor`], [`out_var_visitors`],
//! [`scope_stack_visitor`]), the import resolver ([`import_manager`]),
//! macro/scope helpers and the compile cache.

pub mod check_visitor;
pub mod compile_cache;
pub mod compile_time_function;
pub mod generator_scope;
pub mod import_manager;
pub mod interpolation_mode;
pub mod macro_define;
pub mod operator_factory;
pub mod out_var_visitors;
pub mod parser_operator_manager;
pub mod qlexer;
pub mod qlparser;
pub mod qvm_instruction_visitor;
pub mod scope_stack_visitor;
pub mod syntax_tree;
pub mod token;

pub use check_visitor::CheckVisitor;
pub use compile_cache::{CompileCache, QCompileCache, ScriptCompileCache};
pub use compile_time_function::{CodeGenerator, CompileTimeFunction};
pub use generator_scope::GeneratorScope;
pub use import_manager::{ImportManager, ImportScope, LoadPartQualifiedResult, QLImport};
pub use interpolation_mode::InterpolationMode;
pub use macro_define::MacroDefine;
pub use operator_factory::{OperatorFactory, OperatorManager};
pub use out_var_visitors::{OutFunctionVisitor, OutVarAttrsVisitor, OutVarNamesVisitor};
pub use parser_operator_manager::{OpType, ParserOperatorManager};
pub use qlexer::tokenize;
pub use qlparser::{build_tree, QLParser};
pub use qvm_instruction_visitor::{
    compile_script, CompileTimeFunctions, Context, InstructionMacroDefine, InstructionScope,
    QvmInstructionVisitor, SharedInstruction, UserDefineFunctions,
};
pub use scope_stack_visitor::{ExistStack, ExistVarStack, ScopeStack, ScopedVisitor};
pub use syntax_tree::{ChildRef, HasChildren, Node, TerminalNode, Visitor};
pub use token::Token;
