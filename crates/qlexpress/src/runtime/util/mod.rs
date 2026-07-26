//! VM helper utilities, mirroring Java `com.alibaba.qlexpress4.runtime.util`.

pub mod method_invoke_utils;
pub mod throw_utils;
pub mod value_utils;

pub use method_invoke_utils::{find_method_and_invoke, invoke_i_method, invoke_native_method};
pub use throw_utils::{report_user_defined_exception, wrap_throwable};
pub use value_utils::{assert_number, java_index, to_immutable};
