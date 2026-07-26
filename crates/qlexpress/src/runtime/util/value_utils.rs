//! Value helpers, mirroring Java `ValueUtils`.

use crate::exception::error_reporter::ErrorReporter;
use crate::exception::QLException;
use crate::runtime::data::convert::to_i64;
use crate::runtime::value::{DataValue, QValue};

/// Java `ValueUtils.toImmutable(Value)`.
pub fn to_immutable(origin: &QValue) -> QValue {
    origin.to_immutable()
}

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
