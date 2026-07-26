//! Compile-time function mechanism, mirroring Java
//! `com.alibaba.qlexpress4.aparser.compiletimefunction`.
//!
//! A [`CompileTimeFunction`] is invoked by `QvmInstructionVisitor` while
//! compiling a function call whose name is registered as a compile-time
//! function; it emits instructions directly through the [`CodeGenerator`]
//! callback instead of a runtime `CallFunctionInstruction`.

pub mod code_generator;
#[path = "compile_time_function.rs"]
pub mod contract;

pub use code_generator::CodeGenerator;
pub use contract::CompileTimeFunction;
