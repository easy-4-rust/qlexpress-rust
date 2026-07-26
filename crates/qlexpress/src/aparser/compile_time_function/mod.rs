//! Compile-time function mechanism, mirroring Java
//! `com.alibaba.qlexpress4.aparser.compiletimefunction`.
//!
//! A [`CompileTimeFunction`] is invoked by `QvmInstructionVisitor` while
//! compiling a function call whose name is registered as a compile-time
//! function; it emits instructions directly through the [`CodeGenerator`]
//! callback instead of a runtime `CallFunctionInstruction`.

pub mod code_generator;
pub mod compile_time_function;

pub use code_generator::CodeGenerator;
pub use compile_time_function::CompileTimeFunction;
