use super::*;

use language_core::{CoreError, Value};

/// Verifies canonical formatting and invalid runtime representation handling.
#[test]
fn formats_null_and_rejects_other_representations() {
    let null = Value::new(crate::null::test_type_id(), crate::null::value::Null);
    let integer = Value::new(crate::null::test_type_id(), 0_i64);

    assert_eq!(format(&null).unwrap(), "null");
    assert!(matches!(
        format(&integer),
        Err(CoreError::InvalidValueRepresentation(name)) if name == "null"
    ));
}
