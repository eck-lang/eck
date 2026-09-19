use super::*;

use crate::semantic::{CoreError, Value};

/// Verifies extraction of null payloads and rejection of other representations.
#[test]
fn extracts_null_values_and_rejects_other_representations() {
    let null = Value::new(
        crate::primitives::null::test_type_id(),
        crate::primitives::null::value::Null,
    );
    let integer = Value::new(crate::primitives::null::test_type_id(), 1_i64);

    assert!(get(&null).is_ok());
    assert!(matches!(
        get(&integer),
        Err(CoreError::InvalidValueRepresentation(name)) if name == "null"
    ));
}
