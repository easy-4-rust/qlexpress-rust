#![doc = include_str!("../README.md")]
#![warn(missing_docs)]
//!
//! qlexpress — full semantic migration of Alibaba QLExpress4 to Rust.
//!
//! `lib.rs` is a thin index: only `pub mod` declarations plus facade
//! re-exports. No implementation lives here (see SPEC §2).

// QLException 对齐 Java 单一异常类(携带完整诊断信息),Err 体积大是
// 架构性选择;全库统一 `Result<_, QLException>`,此处豁免该 lint。
#![allow(clippy::result_large_err)]

pub mod annotation;
pub mod aparser;
pub mod api;
pub mod arithmetic;
pub mod assign;
pub mod base;
pub mod bit;
pub mod check_options;
pub mod class_supplier;
pub mod collection;
pub mod compare;
pub mod compiletimefunction;
pub mod context;
pub mod convert;
pub mod data;
pub mod default_class_supplier;
pub mod enums;
pub mod exception;
pub mod express4_runner;
pub mod function;
pub mod init_options;
pub mod instruction;
pub mod lambda;
pub mod logic;
pub mod lsp;
pub mod member;
pub mod number;
pub(crate) mod observability;
pub mod operator;
pub mod parsecache;
pub mod proxy;
pub mod ql_options;
pub mod ql_precedences;
pub mod ql_result;
pub mod runtime;
pub mod scope;
pub mod security;
pub mod string;
pub mod trace;
pub mod unary;
pub mod util;
pub mod utils;

// ---- Facade re-exports (SPEC §2):只 re-export 不定义 ----

pub use check_options::CheckOptions;
pub use class_supplier::ClassSupplier;
pub use default_class_supplier::DefaultClassSupplier;
/// 引擎门面及顶层契约。对应 Java `com.alibaba.qlexpress4` 包顶层类。
pub use express4_runner::Express4Runner;
pub use init_options::InitOptions;
pub use ql_options::{QLOptions, QLOptionsBuilder};
pub use ql_result::QLResult;
/// 为宿主类型生成 `NativeType` 与 `NativeObject` 实现的派生宏。
pub use qlexpress_derive::QLExpressType;

/// 异常体系。对应 Java `com.alibaba.qlexpress4.exception` 公开类。
pub use exception::{
    ErrorReporter, PureErrReporter, QLException, QLExceptionKind, QLSyntaxException,
};

/// 值与上下文。对应 Java `runtime.Value` / `runtime.context.*`。
pub use runtime::context::{ExpressContext, MapExpressContext, QLAliasContext};
pub use runtime::value::{DataValue, NativeObject, QValue, Value};

pub use annotation::{QLFunctionMethod, QLFunctionProvider};
/// 宿主扩展契约:自定义函数、变参函数、批量注册结果、自定义操作符。
/// 对应 Java `runtime.function.CustomFunction` / `api.*` /
/// `runtime.operator.CustomBinaryOperator`。
pub use api::{BatchAddFunctionResult, QLFunctionalVarargs};
pub use runtime::function::CustomFunction;
pub use runtime::operator::custom_binary_operator::CustomBinaryOperator;

/// 安全策略。对应 Java `security.QLSecurityStrategy`。
pub use security::{
    CacheStats, CancellationToken, Capability, CapabilityPolicy, CompileCachePolicy, NativeMember,
    QLSecurityStrategy, ResourceLimits, SandboxProfile,
};

/// 语法树/编译入口支撑(SPEC §3.6 `parse_to_syntax_tree` 的返回模型)。
pub use aparser::{
    build_tree, CheckVisitor, ChildRef, CompileCache, GeneratorScope, HasChildren, ImportManager,
    MacroDefine, Node, OutFunctionVisitor, OutVarAttrsVisitor, OutVarNamesVisitor, QCompileCache,
    QLParser, TerminalNode, Token, Visitor,
};
