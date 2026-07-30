//! Assignable map entry, mirroring Java `MapItemValue`.

use std::cell::RefCell;
use std::rc::Rc;

use crate::runtime::class_ref::ClassRef;
use crate::runtime::data::index_map::IndexMap;
use crate::runtime::left_value::LeftValue;
use crate::runtime::value::{DataValue, Value};

/// 指向共享 Map 指定键并支持读取、插入和覆盖的左值。
/// 对应或承接 Java 源文件：`com/alibaba/qlexpress4/runtime/data/MapItemValue.java`；具体对象路径见 `docs/对象级对照表.md`。
/// Mirrors Java `MapItemValue`: an l-value view of `map[key]`.
/// 对应 Java: com.alibaba.qlexpress4.runtime.data.MapItemValue。
pub struct MapItemValue {
    symbol_name: Option<String>,
    map: Rc<RefCell<IndexMap>>,
    key: DataValue,
}

impl MapItemValue {
    /// 创建对象实例。
    /// 参数：`map`、`key`；返回：`Self`。
    /// 对应或承接 Java 源文件：`com/alibaba/qlexpress4/runtime/data/MapItemValue.java`，构造器 `<init>`；Rust 侧按所有权与 `Result` 语义适配。
    /// Java `MapItemValue(Map map, Object key)`.
    /// 对应 Java: com.alibaba.qlexpress4.runtime.data.MapItemValue#new。
    pub fn new(map: Rc<RefCell<IndexMap>>, key: DataValue) -> Self {
        MapItemValue {
            symbol_name: None,
            map,
            key,
        }
    }

    /// 附加 symbol name 配置并返回新值。
    /// 参数：`symbol_name`、`map`、`key`；返回：`Self`。
    /// 对应或承接 Java 源文件：`com/alibaba/qlexpress4/runtime/data/MapItemValue.java`，方法 `withSymbolName`；Rust 侧按所有权与 `Result` 语义适配。
    /// Java `MapItemValue(String symbolName, Map map, Object key)`.
    /// 对应 Java: com.alibaba.qlexpress4.runtime.data.MapItemValue#withSymbolName。
    pub fn with_symbol_name(
        symbol_name: impl Into<String>,
        map: Rc<RefCell<IndexMap>>,
        key: DataValue,
    ) -> Self {
        MapItemValue {
            symbol_name: Some(symbol_name.into()),
            map,
            key,
        }
    }
}

impl Value for MapItemValue {
    /// Java `map.get(key)`.
    fn get(&self) -> DataValue {
        self.map
            .borrow()
            .get(&self.key)
            .cloned()
            .unwrap_or(DataValue::Null)
    }

    fn type_name(&self) -> &'static str {
        self.get().data_type_name()
    }
}

impl LeftValue for MapItemValue {
    /// Java returns `null` (no declared type).
    fn defined_type(&self) -> Option<ClassRef> {
        None
    }

    /// Java `map.put(key, newValue)`.
    fn set_inner(&mut self, new_value: DataValue) {
        self.map.borrow_mut().insert(self.key.clone(), new_value);
    }

    fn symbol_name(&self) -> Option<&str> {
        self.symbol_name.as_deref()
    }
}

impl std::fmt::Debug for MapItemValue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MapItemValue")
            .field("symbol_name", &self.symbol_name)
            .field("key", &self.key)
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::exception::pure_err_reporter::PureErrReporter;

    #[test]
    fn get_missing_key_yields_null_like_java_map_get() {
        let map = Rc::new(RefCell::new(IndexMap::new()));
        let item = MapItemValue::new(Rc::clone(&map), DataValue::Str("k".into()));
        assert_eq!(item.get(), DataValue::Null);
    }

    #[test]
    fn set_puts_into_shared_map() {
        let map = Rc::new(RefCell::new(IndexMap::new()));
        let mut item =
            MapItemValue::with_symbol_name("m.k", Rc::clone(&map), DataValue::Str("k".into()));
        item.set(DataValue::Int(42), &PureErrReporter::INSTANCE)
            .unwrap();
        assert_eq!(
            map.borrow().get(&DataValue::Str("k".into())),
            Some(&DataValue::Int(42))
        );
        assert_eq!(item.symbol_name(), Some("m.k"));
        assert_eq!(item.defined_type(), None);
    }
}
