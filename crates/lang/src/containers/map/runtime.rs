//! Runtime execution of map literals, reads, and mutable writes.

use crate::RuntimeError;
use crate::containers::map::{MapKey, MapType, MapValue};
use crate::ir::{LocalVariableSlot, TypedExpression};
use crate::runtime::Runtime;
use crate::semantic::Value;

impl Runtime<'_> {
    /// Builds one dynamic map literal in source entry order.
    pub(crate) fn build_map_value(
        &mut self,
        entries: &[(TypedExpression, TypedExpression)],
    ) -> Result<Option<Value>, RuntimeError> {
        let mut map = MapValue::new();
        for (key_expression, value_expression) in entries {
            let key = self
                .eval(key_expression)?
                .ok_or_else(|| RuntimeError::Message("map key returned no value".into()))?;
            let map_key = MapKey::from_value(&key, self.registry)?;
            let value = self
                .eval(value_expression)?
                .ok_or_else(|| RuntimeError::Message("map value returned no value".into()))?;
            map.insert_prepared(map_key, key, value);
        }
        Ok(Some(Value::new_map(MapType::dynamic(), map)))
    }

    /// Reads one map key and returns the compiler-prepared missing value when absent.
    pub(crate) fn read_map_value(
        &mut self,
        map: &TypedExpression,
        key: &TypedExpression,
        missing: &Value,
    ) -> Result<Option<Value>, RuntimeError> {
        let map = self
            .eval(map)?
            .ok_or_else(|| RuntimeError::Message("map expression returned no value".into()))?;
        let key = self
            .eval(key)?
            .ok_or_else(|| RuntimeError::Message("map key returned no value".into()))?;
        let value = MapValue::from_value(&map)?
            .get(&key, self.registry)?
            .cloned()
            .unwrap_or_else(|| missing.clone());
        Ok(Some(value))
    }

    /// Inserts or replaces one entry in a mutable map binding.
    ///
    /// The key is validated and the stored expression is evaluated before the
    /// binding is touched, so either failure leaves the map unchanged.
    pub(crate) fn write_map_value(
        &mut self,
        slot: LocalVariableSlot,
        key: &TypedExpression,
        value: &TypedExpression,
    ) -> Result<(), RuntimeError> {
        let key = self
            .eval(key)?
            .ok_or_else(|| RuntimeError::Message("map key returned no value".into()))?;
        let map_key = MapKey::from_value(&key, self.registry)?;
        let value = self
            .eval(value)?
            .ok_or_else(|| RuntimeError::Message("map value returned no value".into()))?;
        let map = self.local_values[slot.0]
            .as_mut()
            .ok_or_else(|| RuntimeError::Message("map binding is not initialized".into()))?;
        copy_if_shared(map)?;
        MapValue::from_value_mut(map)?.insert_prepared(map_key, key, value);
        Ok(())
    }
}

/// Gives a shared map payload its own structural copy before mutation.
fn copy_if_shared(value: &mut Value) -> Result<(), RuntimeError> {
    let map_type = value
        .map_type()
        .ok_or_else(|| RuntimeError::Message("value is not a map".into()))?;
    if !value.is_uniquely_owned() {
        let copied = MapValue::from_value(value)?.clone();
        *value = Value::new_map(map_type, copied);
    }
    Ok(())
}

#[cfg(test)]
#[path = "runtime.tests.rs"]
mod tests;
