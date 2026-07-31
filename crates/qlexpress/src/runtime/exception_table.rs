//! 作用域异常处理位置表，对应 Java `runtime/ExceptionTable`。
//!
//! Java 对象保存按声明顺序排列的“异常类型 → 相对处理位置”以及可选的
//! `finally` 位置。早期 Rust 迁移还在本对象中加入了 PC 区间表扩展；该扩展
//! API 继续保留，但不替代 Java 的原始字段与查找语义。

use crate::runtime::class_ref::ClassRef;
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
/// 对应 Java：`com.alibaba.qlexpress4.runtime.ExceptionTable`。
#[derive(Clone, Debug, Default)]
pub struct ExceptionTable {
    /// 按声明顺序保存的异常类型与相对处理位置。
    handler_pos_map: Vec<(ClassRef, i32)>,
    /// 可选的 finally 相对位置。
    final_pos: Option<i32>,
    /// Rust 扩展：按 PC 区间保存处理器位置。
    entries: Vec<ExceptionTableEntry>,
}

impl ExceptionTable {
    /// 构造空异常表。
    ///
    /// 对应 Java 常量 `ExceptionTable.EMPTY`。
    pub fn new() -> Self {
        Self::default()
    }

    /// 使用异常处理位置与 finally 位置构造异常表。
    ///
    /// 对应 Java：
    /// `ExceptionTable(List<Map.Entry<Class<?>, Integer>>, Integer)`。
    ///
    /// # 参数
    /// - `handler_pos_map`：按 catch 声明顺序排列的异常类型和相对位置。
    /// - `final_pos`：可空的 finally 相对位置。
    pub fn from_handler_positions(
        handler_pos_map: Vec<(ClassRef, i32)>,
        final_pos: Option<i32>,
    ) -> Self {
        Self {
            handler_pos_map,
            final_pos,
            entries: Vec::new(),
        }
    }

    /// 返回第一个能够处理给定抛出值的相对位置。
    ///
    /// 对应 Java：`ExceptionTable#getRelativePos(Object)`。
    ///
    /// `null` 与 Java 一致由第一个 catch 处理；非空值按类型相同或
    /// `java.lang.Object` catch-all 匹配。需要宿主继承层次时使用
    /// [`Self::get_relative_pos_with`] 提供注册表的可赋值判断。
    ///
    /// # 参数
    /// - `throw_obj`：脚本或宿主抛出的值。
    ///
    /// # 返回值
    /// 返回首个匹配的相对位置；没有匹配项时返回 `None`。
    pub fn get_relative_pos(&self, throw_obj: &DataValue) -> Option<i32> {
        self.get_relative_pos_with(throw_obj, |handler_type, thrown_type| {
            handler_type == thrown_type || handler_type.is_java_object()
        })
    }

    /// 使用宿主提供的类型可赋值判断查找异常处理位置。
    ///
    /// 该入口承接 Java `Class.isAssignableFrom`；显式注册继承关系的 Rust
    /// 宿主可传入注册表判断，从而保留自定义异常父子类型语义。
    ///
    /// # 参数
    /// - `throw_obj`：脚本或宿主抛出的值。
    /// - `is_assignable_from`：判断 catch 类型能否接收实际抛出类型。
    ///
    /// # 返回值
    /// 返回首个匹配的相对位置；没有匹配项时返回 `None`。
    /// 对应 Java：`ExceptionTable#getRelativePos(Object)` 与 `Class#isAssignableFrom(Class)`。
    pub fn get_relative_pos_with(
        &self,
        throw_obj: &DataValue,
        is_assignable_from: impl Fn(&ClassRef, &ClassRef) -> bool,
    ) -> Option<i32> {
        if throw_obj.is_null() {
            return self.handler_pos_map.first().map(|(_, position)| *position);
        }
        let thrown_type = match throw_obj {
            DataValue::Object(object) => ClassRef::from_name(object.borrow().native_type_name()),
            _ => ClassRef::from_name(throw_obj.data_type_name()),
        };
        self.handler_pos_map
            .iter()
            .find(|(handler_type, _)| is_assignable_from(handler_type, &thrown_type))
            .map(|(_, position)| *position)
    }

    /// 返回可选的 finally 相对位置。
    ///
    /// 对应 Java：`ExceptionTable#getFinalPos()`。
    ///
    /// # 返回值
    /// 有 finally 块时返回其位置，否则返回 `None`。
    pub fn get_final_pos(&self) -> Option<i32> {
        self.final_pos
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

    /// `SOURCE_PARITY`：Java 对 `null` 直接选择第一项，并保留可空的
    /// `finalPos`。
    #[test]
    fn java_relative_and_final_positions_preserve_declaration_order() {
        let table = ExceptionTable::from_handler_positions(
            vec![
                (ClassRef::from_name("java.lang.IllegalArgumentException"), 4),
                (ClassRef::from_name("java.lang.Object"), 9),
            ],
            Some(12),
        );

        assert_eq!(table.get_relative_pos(&DataValue::Null), Some(4));
        assert_eq!(table.get_relative_pos(&DataValue::Int(1)), Some(9));
        assert_eq!(table.get_final_pos(), Some(12));
        assert_eq!(ExceptionTable::new().get_final_pos(), None);
    }

    /// `RUST_OBLIGATION`：显式注册的宿主继承关系能够参与 Java
    /// `Class.isAssignableFrom` 等价判断。
    #[test]
    fn registered_assignability_can_select_parent_handler() {
        let table = ExceptionTable::from_handler_positions(
            vec![(ClassRef::from_name("java.lang.Number"), 7)],
            None,
        );

        assert_eq!(
            table.get_relative_pos_with(&DataValue::Int(1), |handler, thrown| {
                handler.java_name() == "java.lang.Number"
                    && thrown.java_name() == "java.lang.Integer"
            }),
            Some(7)
        );
    }

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
