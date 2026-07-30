//! 块作用域,对应 Java `com.alibaba.qlexpress4.runtime.scope.QvmBlockScope`。
//! 职责:持有块级符号表与函数表。
//! 本文件由 `scope/mod.rs` 拆分而来(SPEC §5.5 一类一文件),仅移动代码与补充中文注释,行为完全一致。

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use crate::runtime::scope::{SharedFunctionTable, SymbolTable};

/// 块作用域数据。对应 Java: com.alibaba.qlexpress4.runtime.scope.QvmBlockScope
/// Block scope data, mirroring Java `QvmBlockScope`.
pub struct QvmBlockScope {
    symbol_table: SymbolTable,
    function_table: SharedFunctionTable,
}

impl QvmBlockScope {
    /// 构造块作用域。对应 Java 构造器 `QvmBlockScope(... symbolTable ...)`。
    pub fn new(symbol_table: SymbolTable) -> Self {
        QvmBlockScope {
            symbol_table,
            function_table: Rc::new(RefCell::new(HashMap::new())),
        }
    }

    /// 获取符号表。对应 Java 方法 `getSymbolTable`。
    pub fn symbol_table(&self) -> &SymbolTable {
        &self.symbol_table
    }

    /// 获取函数表。对应 Java 方法 `getFunctionTable`。
    pub fn function_table(&self) -> &SharedFunctionTable {
        &self.function_table
    }

    /// 可变符号表(Java 直接字段访问写穿的 Rust 对应),供 `QScope` 的
    /// define_local_symbol 等变更路径使用。
    pub(crate) fn symbol_table_mut(&mut self) -> &mut SymbolTable {
        &mut self.symbol_table
    }
}
