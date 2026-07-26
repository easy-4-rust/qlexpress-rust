//! Bytecode exception table, mirroring Java `runtime/ExceptionTable`.
//!
//! Each entry records: "if a `catch_type` is thrown while the program
//! counter is in `[start_pc, end_pc)`, jump to `handler_pc`". This is
//! the same shape as the JVM exception table attribute.
//!
//! The current Rust implementation inlines the equivalent fields in
//! the `TryCatchInstruction` struct directly; this file exposes the
//! canonical struct so that future refactors (and the `FixedSizeStack`
//! story) can share a uniform representation.

use crate::runtime::value::DataValue;

/// One exception handler entry, mirroring
/// `com.alibaba.qlexpress4.runtime.ExceptionTable.ExceptionTableEntry`.
///
/// - `start_pc`: inclusive lower bound of the program-counter range.
/// - `end_pc`: exclusive upper bound.
/// - `handler_pc`: jump target on match.
/// - `catch_type`: type-name filter (Java FQN, e.g. `java.lang.Exception`);
///   `None` matches any exception (`finally`-style handlers).
#[derive(Clone, Debug)]
pub struct ExceptionTableEntry {
    pub start_pc: usize,
    pub end_pc: usize,
    pub handler_pc: usize,
    pub catch_type: Option<String>,
}

impl ExceptionTableEntry {
    pub fn covers(&self, pc: usize) -> bool {
        pc >= self.start_pc && pc < self.end_pc
    }
}

/// The full exception table attached to a `try/catch` instruction.
#[derive(Clone, Debug, Default)]
pub struct ExceptionTable {
    entries: Vec<ExceptionTableEntry>,
}

impl ExceptionTable {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_entry(entry: ExceptionTableEntry) -> Self {
        let mut t = Self::new();
        t.entries.push(entry);
        t
    }

    pub fn push(&mut self, entry: ExceptionTableEntry) {
        self.entries.push(entry);
    }

    pub fn entries(&self) -> &[ExceptionTableEntry] {
        &self.entries
    }

    /// Locate the first handler matching `pc` whose `catch_type` matches
    /// `exception.data_type_name()`. When `catch_type` is `None`, the
    /// handler matches any exception.
    pub fn lookup(&self, pc: usize, exception: &DataValue) -> Option<usize> {
        let exc_type = exception.data_type_name();
        self.entries
            .iter()
            .find(|e| e.covers(pc) && e.catch_type.as_deref().is_none_or(|t| t == exc_type))
            .map(|e| e.handler_pc)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn entry_covers_pc_in_range() {
        let e = ExceptionTableEntry {
            start_pc: 1,
            end_pc: 5,
            handler_pc: 10,
            catch_type: None,
        };
        assert!(e.covers(1));
        assert!(e.covers(4));
        assert!(!e.covers(0));
        assert!(!e.covers(5));
    }

    #[test]
    fn lookup_handles_any_when_catch_type_is_none() {
        let t = ExceptionTable::with_entry(ExceptionTableEntry {
            start_pc: 0,
            end_pc: 100,
            handler_pc: 42,
            catch_type: None,
        });
        // Any DataValue matches because catch_type is None.
        assert_eq!(t.lookup(10, &DataValue::Null), Some(42));
        assert_eq!(t.lookup(10, &DataValue::Int(1)), Some(42));
    }

    #[test]
    fn lookup_filters_by_catch_type() {
        // Use DataValue::Int as a stand-in for the exception; the
        // filter compares against `exception.data_type_name()`, which
        // returns `java.lang.Integer` for an Int payload. The handler
        // matches when its `catch_type` equals that string.
        let t = ExceptionTable::with_entry(ExceptionTableEntry {
            start_pc: 0,
            end_pc: 100,
            handler_pc: 42,
            catch_type: Some("java.lang.Integer".into()),
        });
        assert_eq!(t.lookup(10, &DataValue::Int(1)), Some(42));

        let t2 = ExceptionTable::with_entry(ExceptionTableEntry {
            start_pc: 0,
            end_pc: 100,
            handler_pc: 42,
            catch_type: Some("java.lang.RuntimeException".into()),
        });
        // Integer payload does not match a RuntimeException handler.
        assert_eq!(t2.lookup(10, &DataValue::Int(1)), None);
    }
}
