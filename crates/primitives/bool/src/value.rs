use language_core::{CoreError, Value};

/// Extracts the boolean payload from a runtime value.
pub(crate) fn get(value: &Value) -> Result<bool, CoreError> {
    value
        .downcast_ref::<bool>()
        .copied()
        .ok_or_else(|| CoreError::InvalidValueRepresentation("bool".into()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extracts_boolean_values_and_rejects_other_representations() {
        let boolean = Value::new(crate::test_type_id(), true);
        let integer = Value::new(crate::test_type_id(), 1_i64);

        assert!(get(&boolean).unwrap());
        assert!(matches!(
            get(&integer),
            Err(CoreError::InvalidValueRepresentation(name)) if name == "bool"
        ));
    }
}
