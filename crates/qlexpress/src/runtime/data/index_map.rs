//! Insertion-ordered map backing [`DataValue::Map`], mirroring the role of
//! Java's `LinkedHashMap` (SPEC §3.1). Dependency-free: linear scan over a
//! `Vec` of entries, using QLExpress value equality for keys.

use crate::runtime::value::DataValue;

#[derive(Clone, Debug, Default, PartialEq)]
pub struct IndexMap {
    entries: Vec<(DataValue, DataValue)>,
}

impl IndexMap {
    pub fn new() -> Self {
        IndexMap::default()
    }

    pub fn from_entries(entries: Vec<(DataValue, DataValue)>) -> Self {
        let mut map = IndexMap::new();
        for (k, v) in entries {
            map.insert(k, v);
        }
        map
    }

    /// Java `Map.get`.
    pub fn get(&self, key: &DataValue) -> Option<&DataValue> {
        self.entries.iter().find(|(k, _)| k == key).map(|(_, v)| v)
    }

    /// Java `Map.containsKey`.
    pub fn contains_key(&self, key: &DataValue) -> bool {
        self.get(key).is_some()
    }

    /// Java `Map.put`: replaces the value in place when the key exists,
    /// otherwise appends (preserving insertion order). Returns the old value.
    pub fn insert(&mut self, key: DataValue, value: DataValue) -> Option<DataValue> {
        for (k, v) in &mut self.entries {
            if *k == key {
                return Some(std::mem::replace(v, value));
            }
        }
        self.entries.push((key, value));
        None
    }

    /// Java `Map.remove`, keeping the order of remaining entries.
    pub fn remove(&mut self, key: &DataValue) -> Option<DataValue> {
        let index = self.entries.iter().position(|(k, _)| k == key)?;
        Some(self.entries.remove(index).1)
    }

    /// Java `Map.clear`.
    pub fn clear(&mut self) {
        self.entries.clear();
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Entries in insertion order.
    pub fn entries(&self) -> &[(DataValue, DataValue)] {
        &self.entries
    }

    pub fn keys(&self) -> impl Iterator<Item = &DataValue> {
        self.entries.iter().map(|(k, _)| k)
    }

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
