//! Insertion-ordered hash map storage for runtime values.

use std::collections::{HashMap, hash_map::Entry};

use crate::semantic::{CoreError, Registry, Value};

use super::MapKey;

/// Public runtime payload for an ECK map.
#[derive(Clone)]
pub struct MapValue {
    /// Resolves exact scalar keys to their insertion-order entry positions.
    indices: HashMap<MapKey, usize>,
    /// Owns original keys and current values in deterministic insertion order.
    entries: Vec<MapEntry>,
}

/// One original key and current value retained for ordered formatting.
#[derive(Clone)]
pub(super) struct MapEntry {
    pub(super) key: Value,
    pub(super) value: Value,
}

impl MapValue {
    /// Creates an empty dynamic map.
    pub fn new() -> Self {
        Self {
            indices: HashMap::new(),
            entries: Vec::new(),
        }
    }

    /// Inserts one key/value pair and returns the value it replaced, if any.
    ///
    /// Replacing an existing key preserves its original key object and insertion
    /// position. Invalid keys leave the map unchanged.
    pub fn insert(
        &mut self,
        key: Value,
        value: Value,
        registry: &Registry,
    ) -> Result<Option<Value>, CoreError> {
        let map_key = MapKey::from_value(&key, registry)?;
        Ok(self.insert_prepared(map_key, key, value))
    }

    /// Inserts a key that has already crossed the runtime key boundary.
    pub(super) fn insert_prepared(
        &mut self,
        map_key: MapKey,
        key: Value,
        value: Value,
    ) -> Option<Value> {
        match self.indices.entry(map_key) {
            Entry::Occupied(entry) => {
                let index = *entry.get();
                Some(std::mem::replace(&mut self.entries[index].value, value))
            }
            Entry::Vacant(entry) => {
                let index = self.entries.len();
                self.entries.push(MapEntry { key, value });
                entry.insert(index);
                None
            }
        }
    }

    /// Returns the value stored for one exact scalar key.
    pub fn get(&self, key: &Value, registry: &Registry) -> Result<Option<&Value>, CoreError> {
        let map_key = MapKey::from_value(key, registry)?;
        Ok(self
            .indices
            .get(&map_key)
            .map(|index| &self.entries[*index].value))
    }

    /// Borrows a map payload after checking both semantic identity and payload type.
    pub fn from_value(value: &Value) -> Result<&Self, CoreError> {
        if value.map_type().is_none() {
            return Err(invalid_map_value());
        }
        value.downcast_ref::<Self>().ok_or_else(invalid_map_value)
    }

    /// Mutably borrows an exclusively owned map payload after validation.
    pub fn from_value_mut(value: &mut Value) -> Result<&mut Self, CoreError> {
        if value.map_type().is_none() {
            return Err(invalid_map_value());
        }
        value.downcast_mut::<Self>().ok_or_else(invalid_map_value)
    }

    /// Borrows all entries in deterministic insertion order.
    pub(super) fn entries(&self) -> &[MapEntry] {
        &self.entries
    }
}

impl Default for MapValue {
    /// Creates an empty dynamic map.
    fn default() -> Self {
        Self::new()
    }
}

/// Returns the stable error for a non-map identity or malformed map payload.
pub(crate) fn invalid_map_value() -> CoreError {
    CoreError::InvalidValueRepresentation("map".into())
}

#[cfg(test)]
#[path = "value.tests.rs"]
mod tests;
