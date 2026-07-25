//! qlexpress_rust — full semantic migration of Alibaba QLExpress4 to Rust.
//!
//! `lib.rs` is a thin index: only `pub mod` declarations plus facade
//! re-exports. No implementation lives here (see SPEC §2).

pub mod aparser;
pub mod check_options;
pub mod class_supplier;
pub mod exception;
pub mod init_options;
pub mod operator;
pub mod ql_options;
pub mod ql_precedences;
pub mod ql_result;
pub mod runtime;
pub mod security;
pub mod utils;

// ---- Facade re-exports (SPEC §2) ----
// Express4Runner is delivered in Stage 5; re-exported here once implemented.
// `parse_to_syntax_tree` support (SPEC §3.6): the parser entry point and
// the syntax tree model.
pub use aparser::{
    build_tree, ChildRef, CheckVisitor, CompileCache, GeneratorScope, HasChildren, ImportManager,
    MacroDefine, Node, OutFunctionVisitor, OutVarAttrsVisitor, OutVarNamesVisitor, QCompileCache,
    QLParser, TerminalNode, Token, Visitor,
};
pub use check_options::CheckOptions;
pub use exception::{ErrorReporter, PureErrReporter, QLException, QLExceptionKind, QLSyntaxException};
pub use init_options::InitOptions;
pub use ql_options::{QLOptions, QLOptionsBuilder};
pub use ql_result::QLResult;
pub use runtime::value::{DataValue, NativeObject, Value};
