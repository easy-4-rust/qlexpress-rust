//! Bounded-value stack, mirroring Java `runtime/FixedSizeStack`.
//!
//! Java uses a fixed-capacity stack for the QVM operand stack so that
//! overruns are caught at run time (rather than corrupting the heap).
//! The current Rust VM uses an unbounded `Vec<DataValue>` for the
//! operand stack; this struct provides an opt-in bounded variant for
//! future hardening and for tests that want to exercise stack-overflow
//! behaviour explicitly.
//!
//! Usage:
//! ```ignore
//! let mut stack = FixedSizeStack::with_capacity(16);
//! stack.push(DataValue::Int(1))?;   // Ok(1)
//! stack.push(DataValue::Int(2))?;   // Ok(2)
//! assert_eq!(stack.pop(), Some(DataValue::Int(2)));
//! ```

use crate::exception::{QLException, QLExceptionKind};
use crate::runtime::value::DataValue;

/// A bounded stack with explicit overflow / underflow detection.
#[derive(Clone, Debug)]
pub struct FixedSizeStack {
    data: Vec<DataValue>,
    capacity: usize,
}

impl FixedSizeStack {
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            data: Vec::with_capacity(capacity),
            capacity,
        }
    }

    pub fn capacity(&self) -> usize {
        self.capacity
    }

    pub fn len(&self) -> usize {
        self.data.len()
    }

    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }

    pub fn is_full(&self) -> bool {
        self.data.len() >= self.capacity
    }

    /// Push a value; returns an `Runtime` exception on overflow.
    pub fn push(&mut self, value: DataValue) -> Result<(), QLException> {
        if self.is_full() {
            return Err(QLException::for_test(
                QLExceptionKind::Runtime,
                format!(
                    "FixedSizeStack overflow (capacity = {})",
                    self.capacity
                ),
                crate::exception::error_codes::STACK_OVERFLOW,
            ));
        }
        self.data.push(value);
        Ok(())
    }

    /// Pop the top value; returns `None` when the stack is empty.
    pub fn pop(&mut self) -> Option<DataValue> {
        self.data.pop()
    }

    /// Peek at the top value without popping.
    pub fn peek(&self) -> Option<&DataValue> {
        self.data.last()
    }

    /// Read the value at depth `n` from the top (`n = 0` is the top).
    pub fn peek_at(&self, depth: usize) -> Option<&DataValue> {
        if depth >= self.data.len() {
            return None;
        }
        Some(&self.data[self.data.len() - 1 - depth])
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::exception::error_codes;

    #[test]
    fn push_pop_basic() {
        let mut s = FixedSizeStack::with_capacity(4);
        assert!(s.push(DataValue::Int(1)).is_ok());
        assert!(s.push(DataValue::Int(2)).is_ok());
        assert_eq!(s.peek(), Some(&DataValue::Int(2)));
        assert_eq!(s.pop(), Some(DataValue::Int(2)));
        assert_eq!(s.pop(), Some(DataValue::Int(1)));
        assert_eq!(s.pop(), None);
    }

    #[test]
    fn overflow_returns_error() {
        let mut s = FixedSizeStack::with_capacity(2);
        assert!(s.push(DataValue::Int(1)).is_ok());
        assert!(s.push(DataValue::Int(2)).is_ok());
        let err = s.push(DataValue::Int(3)).unwrap_err();
        assert!(matches!(err.kind(), QLExceptionKind::Runtime));
        assert_eq!(err.error_code(), error_codes::STACK_OVERFLOW);
    }

    #[test]
    fn peek_at_walks_from_top() {
        let mut s = FixedSizeStack::with_capacity(4);
        s.push(DataValue::Int(10)).unwrap();
        s.push(DataValue::Int(20)).unwrap();
        s.push(DataValue::Int(30)).unwrap();
        assert_eq!(s.peek_at(0), Some(&DataValue::Int(30)));
        assert_eq!(s.peek_at(1), Some(&DataValue::Int(20)));
        assert_eq!(s.peek_at(2), Some(&DataValue::Int(10)));
        assert_eq!(s.peek_at(3), None);
    }
}