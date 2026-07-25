//! Insertion-ordered string-keyed map (Java `LinkedHashMap` equivalent),
//! dependency-free per SPEC §3.2.

use crate::runtime::value::DataValue;

/// An insertion-ordered map from [`DataValue`] keys to [`DataValue`] values.
///
/// Mirrors the subset of `java.util.LinkedHashMap` the engine needs:
/// insertion-order iteration, `get`/`put`/`remove`/`containsKey`.
#[derive(Clone, Debug, Default)]
pub struct IndexMap {
    entries: Vec<(DataValue, DataValue)>,
}

impl IndexMap {
    pub fn new() -> Self {
        IndexMap { entries: Vec::new() }
    }

    pub fn with_capacity(capacity: usize) -> Self {
        IndexMap {
            entries: Vec::with_capacity(capacity),
        }
    }

    /// Java `LinkedHashMap` copy-constructor.
    pub fn from_entries(entries: Vec<(DataValue, DataValue)>) -> Self {
        let mut map = IndexMap::with_capacity(entries.len());
        for (k, v) in entries {
            map.insert(k, v);
        }
        map
    }

    /// Java `get`.
    pub fn get(&self, key: &DataValue) -> Option<&DataValue> {
        self.entries
            .iter()
            .find(|(k, _)| k == key)
            .map(|(_, v)| v)
    }

    /// Java `put`: replaces the value when the key exists (keeping its
    /// position), appends otherwise. Returns the previous value.
    pub fn insert(&mut self, key: DataValue, value: DataValue) -> Option<DataValue> {
        match self.entries.iter_mut().find(|(k, _)| *k == key) {
            Some(slot) => Some(std::mem::replace(&mut slot.1, value)),
            None => {
                self.entries.push((key, value));
                None
            }
        }
    }

    /// Java `containsKey`.
    pub fn contains_key(&self, key: &DataValue) -> bool {
        self.entries.iter().any(|(k, _)| k == key)
    }

    /// Java `remove`.
    pub fn remove(&mut self, key: &DataValue) -> Option<DataValue> {
        self.entries
            .iter()
            .position(|(k, _)| k == key)
            .map(|pos| self.entries.remove(pos).1)
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    pub fn clear(&mut self) {
        self.entries.clear();
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

    pub fn iter(&self) -> impl Iterator<Item = (&DataValue, &DataValue)> {
        self.entries.iter().map(|(k, v)| (k, v))
    }
}

impl PartialEq for IndexMap {
    fn eq(&self, other: &Self) -> bool {
        self.entries == other.entries
    }
}

impl FromIterator<(DataValue, DataValue)> for IndexMap {
    fn from_iter<T: IntoIterator<Item = (DataValue, DataValue)>>(iter: T) -> Self {
        let mut map = IndexMap::new();
        for (k, v) in iter {
            map.insert(k, v);
        }
        map
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn insertion_order_and_replace_semantics() {
        let mut map = IndexMap::new();
        assert!(map.insert(DataValue::Str("b".into()), DataValue::Int(1)).is_none());
        map.insert(DataValue::Str("a".into()), DataValue::Int(2));
        // Replacing keeps original position (LinkedHashMap semantics).
        assert_eq!(
            map.insert(DataValue::Str("b".into()), DataValue::Int(3)),
            Some(DataValue::Int(1))
        );
        let keys: Vec<_> = map.keys().cloned().collect();
        assert_eq!(
            keys,
            vec![DataValue::Str("b".into()), DataValue::Str("a".into())]
        );
        assert_eq!(map.get(&DataValue::Str("b".into())), Some(&DataValue::Int(3)));
        assert_eq!(map.len(), 2);
        assert!(!map.is_empty());
        assert!(map.contains_key(&DataValue::Str("a".into())));
        assert_eq!(map.remove(&DataValue::Str("a".into())), Some(DataValue::Int(2)));
        assert!(!map.contains_key(&DataValue::Str("a".into())));
        map.clear();
        assert!(map.is_empty());
    }

    #[test]
    fn numeric_keys_match_across_types() {
        let mut map = IndexMap::new();
        map.insert(DataValue::Int(1), DataValue::Str("one".into()));
        // 1L and 1.0 hit the same slot (QLExpress numeric equality).
        assert_eq!(
            map.get(&DataValue::Long(1)),
            Some(&DataValue::Str("one".into()))
        );
        assert!(map.contains_key(&DataValue::Double(1.0)));
    }
}
