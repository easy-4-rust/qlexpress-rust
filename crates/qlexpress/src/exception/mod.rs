//! Exception system mirroring Java `com.alibaba.qlexpress4.exception`.
//!
//! Unlike Java's checked `RuntimeException` hierarchy, Rust uses a single
//! error type [`QLException`] with a [`QLExceptionKind`] discriminant so the
//! whole engine can use `Result<T, QLException>` (SPEC §3.4). Each Java
//! exception class still has a corresponding file/type in this module.

pub mod default_err_reporter;
pub mod error_reporter;
pub mod ex_message;
pub mod ex_message_util;
/// `exception_factory` 子模块。
pub mod exception_factory;
pub mod exception_type;
pub use crate::lsp;
pub mod pure_err_reporter;
pub mod ql_error_codes;
pub mod ql_exception;
pub mod ql_exception_kind;
pub use ql_error_codes as error_codes;
pub mod ql_runtime_exception;
/// `ql_syntax_exception` 子模块。
pub mod ql_syntax_exception;
/// `ql_timeout_exception` 子模块。
pub mod ql_timeout_exception;
/// `user_define_exception` 子模块。
pub mod user_define_exception;

pub use default_err_reporter::DefaultErrReporter;
pub use error_reporter::ErrorReporter;
pub use ex_message::ExMessage;
pub use ex_message_util::ExMessageUtil;
pub use exception_type::ExceptionType;
pub use pure_err_reporter::PureErrReporter;
pub use ql_exception::QLException;
pub use ql_exception_kind::QLExceptionKind;
pub use ql_runtime_exception::QLRuntimeException;
pub use ql_syntax_exception::QLSyntaxException;
pub use ql_timeout_exception::QLTimeoutException;
pub use user_define_exception::UserDefineException;
