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

pub use super::exception_table_entry::ExceptionTableEntry;

impl ExceptionTableEntry {
    /// 判断程序计数器是否落在该异常处理区间内。
    /// 对应 Java: `ExceptionTable` 查找覆盖当前指令的 catch 条目。
    pub fn covers(&self, pc: usize) -> bool {
        pc >= self.start_pc && pc < self.end_pc
    }
}

/// 按声明顺序保存并查找 try/catch 异常处理分支。
/// 对应或承接 Java 源文件：`com/alibaba/qlexpress4/runtime/ExceptionTable.java`；具体对象路径见 `docs/对象级对照表.md`。
/// The full exception table attached to a `try/catch` instruction.
#[derive(Clone, Debug, Default)]
pub struct ExceptionTable {
    entries: Vec<ExceptionTableEntry>,
}

impl ExceptionTable {
    /// 构造实例。对应 Java 源码 `com/alibaba/qlexpress4/runtime/ExceptionTable.java:12` 的 `ExceptionTable::<init>`。
    pub fn new() -> Self {
        Self::default()
    }

    /// 追加一条异常处理区间并返回表本身，便于构建器式初始化。
    /// 对应 Java: `ExceptionTable#addExceptionTableEntry`。
    pub fn with_entry(entry: ExceptionTableEntry) -> Self {
        let mut t = Self::new();
        t.entries.push(entry);
        t
    }

    /// 追加一条异常处理区间。
    /// 对应 Java: `ExceptionTable#addExceptionTableEntry`。
    pub fn push(&mut self, entry: ExceptionTableEntry) {
        self.entries.push(entry);
    }

    /// 返回按编译顺序保存的异常处理区间。
    /// 对应 Java: `ExceptionTable` 的异常表条目集合。
    pub fn entries(&self) -> &[ExceptionTableEntry] {
        &self.entries
    }

    /// 按异常类型查找首个可匹配的 catch 处理器。
    /// 参数：`pc`、`exception`；返回：`Option<usize>`。
    /// 对应或承接 Java 源文件：`com/alibaba/qlexpress4/runtime/ExceptionTable.java`，方法 `lookup`；Rust 侧按所有权与 `Result` 语义适配。
    /// Locate the first handler matching `pc` whose `catch_type` matches
    /// `exception.data_type_name()`. When `catch_type` is `None`, the
    /// handler matches any exception.
    /// 对应 Java: com.alibaba.qlexpress4.runtime.ExceptionTable#lookup。
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
