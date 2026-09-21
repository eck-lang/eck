use crate::semantic::{
    ArrayType, CoreError, Registry, ScalarRepresentation, SemanticType, Value, ValueType,
};

use super::*;

/// Allocates a scalar type ID for array payload fixtures.
fn value_type() -> crate::semantic::TypeId {
    Registry::new().allocate_type_id()
}

/// Creates a scalar integer payload fixture.
fn value(number: i64) -> Value {
    Value::new(value_type(), number)
}

/// Creates the array identity used by public payload tests.
fn array_type() -> ArrayType {
    ArrayType::static_element(
        SemanticType::Scalar(ValueType::plain(value_type())),
        ScalarRepresentation::Exact,
    )
}

/// Reads an integer fixture from an array element.
fn integer(value: &Value) -> i64 {
    *value
        .downcast_ref::<i64>()
        .expect("the fixture stores an integer")
}

/// Verifies the public array API and complete identity round-trip.
#[test]
fn public_api_preserves_array_identity_and_order() {
    let array_type = array_type();
    let mut array = ArrayValue::new(vec![value(1), value(2)]);
    array.unshift(value(0));
    array.elements_mut()[1] = value(9);
    let wrapped = Value::new_array(array_type.clone(), array);

    assert_eq!(
        wrapped.semantic_type(),
        SemanticType::array(array_type.clone())
    );
    let borrowed = ArrayValue::from_value(&wrapped).unwrap();
    assert_eq!(borrowed.length(), 3);
    assert_eq!(
        borrowed.elements().iter().map(integer).collect::<Vec<_>>(),
        [0, 9, 2]
    );
}

/// Verifies mutable extraction rejects shared payloads until copy-on-write occurs.
#[test]
fn mutable_extraction_rejects_shared_payloads() {
    let array_type = array_type();
    let mut value = Value::new_array(array_type.clone(), ArrayValue::new(vec![value(1)]));
    let clone = value.clone();

    assert!(ArrayValue::from_value_mut(&mut value).is_err());
    drop(clone);
    assert!(ArrayValue::from_value_mut(&mut value).is_ok());
}

/// Verifies scalar identities and mismatched payloads produce stable errors.
#[test]
fn malformed_array_values_are_rejected() {
    let scalar = Value::new(value_type(), 1_i64);
    assert!(matches!(
        ArrayValue::from_value(&scalar),
        Err(CoreError::InvalidValueRepresentation(name)) if name == "array"
    ));

    let array_payload_with_scalar_identity = Value::new(value_type(), ArrayValue::new(Vec::new()));
    assert!(matches!(
        ArrayValue::from_value(&array_payload_with_scalar_identity),
        Err(CoreError::InvalidValueRepresentation(name)) if name == "array"
    ));

    let malformed = Value::new_array(array_type(), 1_i64);
    assert!(matches!(
        ArrayValue::from_value(&malformed),
        Err(CoreError::InvalidValueRepresentation(name)) if name == "array"
    ));
}

/// Verifies clones copy live elements into independent compact storage.
#[test]
fn clone_is_independent_and_compact() {
    let array_type = array_type();
    let mut source = ArrayValue::new(Vec::new());
    for number in 0..8 {
        source.push(value(number));
    }
    source.unshift(value(-1));
    let mut clone = source.clone();
    clone.pop();

    let source_value = Value::new_array(array_type.clone(), source);
    assert_eq!(ArrayValue::from_value(&source_value).unwrap().length(), 9);
    assert_eq!(
        clone.elements().iter().map(integer).collect::<Vec<_>>(),
        [-1, 0, 1, 2, 3, 4, 5, 6]
    );
}
