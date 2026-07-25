//! VM helper utilities, mirroring Java `com.alibaba.qlexpress4.runtime.util`.

pub mod throw_utils;
pub mod value_utils;

pub use throw_utils::{report_user_defined_exception, wrap_throwable};
pub use value_utils::{assert_number, java_index, to_immutable};
