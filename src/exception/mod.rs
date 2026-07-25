//! Exception system mirroring Java `com.alibaba.qlexpress4.exception`.
//!
//! Unlike Java's checked `RuntimeException` hierarchy, Rust uses a single
//! error type [`QLException`] with a [`QLExceptionKind`] discriminant so the
//! whole engine can use `Result<T, QLException>` (SPEC §3.4). Each Java
//! exception class still has a corresponding file/type in this module.

pub mod default_err_reporter;
pub mod error_codes;
pub mod error_reporter;
pub mod ex_message_util;
pub mod exception_factory;
pub mod lsp;
pub mod pure_err_reporter;
pub mod ql_exception;
pub mod ql_runtime_exception;
pub mod ql_syntax_exception;
pub mod ql_timeout_exception;
pub mod user_define_exception;

pub use default_err_reporter::DefaultErrReporter;
pub use error_reporter::ErrorReporter;
pub use ex_message_util::{ExMessage, ExMessageUtil};
pub use pure_err_reporter::PureErrReporter;
pub use ql_exception::{QLException, QLExceptionKind};
pub use ql_runtime_exception::QLRuntimeException;
pub use ql_syntax_exception::QLSyntaxException;
pub use ql_timeout_exception::QLTimeoutException;
pub use user_define_exception::{ExceptionType, UserDefineException};
