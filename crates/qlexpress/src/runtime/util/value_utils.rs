//! Value helpers, mirroring Java `ValueUtils`.

use crate::exception::error_reporter::ErrorReporter;
use crate::exception::QLException;
use crate::runtime::data::convert::to_i64;
use crate::runtime::value::{DataValue, QValue};

/// 转换为 immutable。
/// 参数：`origin`；返回：`QValue`。
/// 对应或承接 Java 源文件：`com/alibaba/qlexpress4/runtime/util/ValueUtils.java`，方法 `toImmutable`；Rust 侧按所有权与 `Result` 语义适配。
/// Java `ValueUtils.toImmutable(Value)`.
pub fn to_immutable(origin: &QValue) -> QValue {
    origin.to_immutable()
}

/// 校验 number。
/// 参数：`obj`、`err_code`、`err_msg`、`error_reporter`；返回：`Result<i64, QLException>`。
/// 对应或承接 Java 源文件：`com/alibaba/qlexpress4/runtime/util/ValueUtils.java`，方法 `assertNumber`；Rust 侧按所有权与 `Result` 语义适配。
/// Java `ValueUtils.assertType(obj, Number.class, ...)` specialised to the
/// only usage in the VM (index/slice operands): returns the value as an
/// integer when it is a `Number`, else reports the given error.
pub fn assert_number(
    obj: &DataValue,
    err_code: &str,
    err_msg: &str,
    error_reporter: &dyn ErrorReporter,
) -> Result<i64, QLException> {
    if obj.is_number() {
        return Ok(to_i64(obj));
    }
    Err(error_reporter.report(err_code, err_msg))
}

/// 处理 java index 对应的领域职责。
/// 参数：`length`、`ql_index`；返回：`i64`。
/// 对应或承接 Java 源文件：`com/alibaba/qlexpress4/runtime/util/ValueUtils.java`，方法 `javaIndex`；Rust 侧按所有权与 `Result` 语义适配。
/// Java `ValueUtils.javaIndex`: negative QL indices count from the end.
pub fn java_index(length: i64, ql_index: i64) -> i64 {
    if ql_index < 0 {
        length + ql_index
    } else {
        ql_index
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn java_index_wraps_negatives() {
        assert_eq!(java_index(5, -1), 4);
        assert_eq!(java_index(5, 2), 2);
    }
}
