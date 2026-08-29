use language_core::{CoreError, Value};

use super::value::get;

/// Formats a boolean runtime value in its canonical representation.
pub(crate) fn format(value: &Value) -> Result<String, CoreError> {
    Ok(get(value)?.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formats_booleans_and_rejects_other_representations() {
        let boolean = Value::new(crate::test_type_id(), false);
        let integer = Value::new(crate::test_type_id(), 0_i64);

        assert_eq!(format(&boolean).unwrap(), "false");
        assert!(matches!(
            format(&integer),
            Err(CoreError::InvalidValueRepresentation(name)) if name == "bool"
        ));
    }
}
