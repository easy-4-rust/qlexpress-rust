//! 脚本可调用函数,对应 Java `com.alibaba.qlexpress4.runtime.function` 包。
//!
//! 一类一文件(SPEC §5.5):本文件仅做 mod 声明与 re-export,不含实现。
//! (Stage 3a 的 `function.rs` 骨架已按 Java 类边界拆入 `function/` 目录。)

pub mod custom_function;
pub mod extension_function;
pub mod filter_extension_function;
pub mod lazy_arg_custom_function;
pub mod map_extension_function;
pub mod qlambda_function;
pub mod qmethod_function;

pub use custom_function::CustomFunction;
pub use extension_function::{as_native_method, ExtensionFunction};
pub use filter_extension_function::FilterExtensionFunction;
pub use lazy_arg_custom_function::LazyArgCustomFunction;
pub use map_extension_function::MapExtensionFunction;
pub use qlambda_function::QLambdaFunction;
pub use qmethod_function::QMethodFunction;
