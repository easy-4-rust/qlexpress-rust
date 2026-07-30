//! Insertion-ordered map backing [`DataValue::Map`], mirroring the role of
//! Java's `LinkedHashMap` (SPEC §3.1). Dependency-free: linear scan over a
//! `Vec` of entries, using QLExpress value equality for keys.

use crate::runtime::value::DataValue;

#[derive(Clone, Debug, Default, PartialEq)]
/// 按插入顺序保存 QLExpress Map 键值对，并使用脚本值相等语义查键。
/// 对应 Java `LinkedHashMap<Object, Object>` 的运行时职责。
pub struct IndexMap {
    entries: Vec<(DataValue, DataValue)>,
}

impl IndexMap {
    /// 创建空的有序 Map。对应 Java: `new LinkedHashMap<>()`。
    pub fn new() -> Self {
        IndexMap::default()
    }

    /// 按给定顺序构造 Map；重复键保留首次位置并覆盖其值。
    /// 对应 Java: 依次调用 `LinkedHashMap#put`。
    pub fn from_entries(entries: Vec<(DataValue, DataValue)>) -> Self {
        let mut map = IndexMap::new();
        for (k, v) in entries {
            map.insert(k, v);
        }
        map
    }

    /// 处理 get 对应的领域职责。
    /// 参数：`key`；返回：`Option<&DataValue>`。
    /// Rust 原生适配；承接当前文件既有 rustdoc 标注的 Java 职责；Rust 侧按所有权与 `Result` 语义适配。
    /// Java `Map.get`.
    /// 对应 Java: 无（Rust 原生适配）。
    pub fn get(&self, key: &DataValue) -> Option<&DataValue> {
        self.entries.iter().find(|(k, _)| k == key).map(|(_, v)| v)
    }

    /// 判断 key 条件。
    /// 参数：`key`；返回：`bool`。
    /// Rust 原生适配；承接当前文件既有 rustdoc 标注的 Java 职责；Rust 侧按所有权与 `Result` 语义适配。
    /// Java `Map.containsKey`.
    /// 对应 Java: 无（Rust 原生适配）。
    pub fn contains_key(&self, key: &DataValue) -> bool {
        self.get(key).is_some()
    }

    /// 处理 insert 对应的领域职责。
    /// 参数：`key`、`value`；返回：`Option<DataValue>`。
    /// Rust 原生适配；承接当前文件既有 rustdoc 标注的 Java 职责；Rust 侧按所有权与 `Result` 语义适配。
    /// Java `Map.put`: replaces the value in place when the key exists,
    /// otherwise appends (preserving insertion order). Returns the old value.
    /// 对应 Java: 无（Rust 原生适配）。
    pub fn insert(&mut self, key: DataValue, value: DataValue) -> Option<DataValue> {
        for (k, v) in &mut self.entries {
            if *k == key {
                return Some(std::mem::replace(v, value));
            }
        }
        self.entries.push((key, value));
        None
    }

    /// 处理 remove 对应的领域职责。
    /// 参数：`key`；返回：`Option<DataValue>`。
    /// Rust 原生适配；承接当前文件既有 rustdoc 标注的 Java 职责；Rust 侧按所有权与 `Result` 语义适配。
    /// Java `Map.remove`, keeping the order of remaining entries.
    /// 对应 Java: 无（Rust 原生适配）。
    pub fn remove(&mut self, key: &DataValue) -> Option<DataValue> {
        let index = self.entries.iter().position(|(k, _)| k == key)?;
        Some(self.entries.remove(index).1)
    }

    /// 处理 clear 对应的领域职责。
    /// 无显式参数；返回：无。
    /// Rust 原生适配；承接当前文件既有 rustdoc 标注的 Java 职责；Rust 侧按所有权与 `Result` 语义适配。
    /// Java `Map.clear`.
    /// 对应 Java: 无（Rust 原生适配）。
    pub fn clear(&mut self) {
        self.entries.clear();
    }

    /// 返回键值对数量。对应 Java: `Map#size`。
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// 判断 Map 是否为空。对应 Java: `Map#isEmpty`。
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// 处理 entries 对应的领域职责。
    /// 无显式参数；返回：`&[(DataValue, DataValue)]`。
    /// 对应或承接 Java 源文件：`com/alibaba/qlexpress4/runtime/QRuntime.java`，方法 `entries`；Rust 侧按所有权与 `Result` 语义适配。
    /// Entries in insertion order.
    pub fn entries(&self) -> &[(DataValue, DataValue)] {
        &self.entries
    }

    /// 按插入顺序迭代键。对应 Java: `LinkedHashMap#keySet`。
    pub fn keys(&self) -> impl Iterator<Item = &DataValue> {
        self.entries.iter().map(|(k, _)| k)
    }

    /// 按插入顺序迭代值。对应 Java: `LinkedHashMap#values`。
    pub fn values(&self) -> impl Iterator<Item = &DataValue> {
        self.entries.iter().map(|(_, v)| v)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keeps_insertion_order_and_replaces_in_place() {
        let mut map = IndexMap::new();
        map.insert(DataValue::Str("b".into()), DataValue::Int(2));
        map.insert(DataValue::Str("a".into()), DataValue::Int(1));
        map.insert(DataValue::Str("b".into()), DataValue::Int(3));
        let keys: Vec<_> = map.keys().cloned().collect();
        assert_eq!(
            keys,
            vec![DataValue::Str("b".into()), DataValue::Str("a".into())]
        );
        assert_eq!(
            map.get(&DataValue::Str("b".into())),
            Some(&DataValue::Int(3))
        );
        assert_eq!(map.len(), 2);
    }

    #[test]
    fn numeric_keys_match_across_types() {
        let mut map = IndexMap::new();
        map.insert(DataValue::Int(1), DataValue::Str("one".into()));
        // 1L == 1 under QLExpress numeric equality.
        assert_eq!(
            map.get(&DataValue::Long(1)),
            Some(&DataValue::Str("one".into()))
        );
    }

    #[test]
    fn remove_keeps_order() {
        let mut map = IndexMap::from_entries(vec![
            (DataValue::Int(1), DataValue::Int(10)),
            (DataValue::Int(2), DataValue::Int(20)),
            (DataValue::Int(3), DataValue::Int(30)),
        ]);
        assert_eq!(map.remove(&DataValue::Int(2)), Some(DataValue::Int(20)));
        let keys: Vec<_> = map.keys().cloned().collect();
        assert_eq!(keys, vec![DataValue::Int(1), DataValue::Int(3)]);
    }
}
