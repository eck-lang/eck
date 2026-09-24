use super::*;
use crate::containers::map::MapType;

/// Builds the complete built-in registry used by map-value tests.
fn registry() -> Registry {
    crate::default_registry().expect("the built-in registry should initialize")
}

/// Creates an integer runtime value.
fn integer(registry: &Registry, number: i64) -> Value {
    Value::new(registry.type_by_name("int64").unwrap(), number)
}

/// Reads an integer runtime payload.
fn integer_payload(value: &Value) -> i64 {
    *value
        .downcast_ref::<i64>()
        .expect("the fixture should contain an integer")
}

/// Verifies exact-key insertion, replacement, and lookup behavior.
#[test]
fn inserts_replaces_and_gets_values() {
    let registry = registry();
    let mut map = MapValue::new();

    assert!(
        map.insert(integer(&registry, 1), integer(&registry, 10), &registry)
            .unwrap()
            .is_none()
    );
    let replaced = map
        .insert(integer(&registry, 1), integer(&registry, 20), &registry)
        .unwrap()
        .unwrap();

    assert_eq!(integer_payload(&replaced), 10);
    assert_eq!(
        map.get(&integer(&registry, 1), &registry)
            .unwrap()
            .map(integer_payload),
        Some(20)
    );
    assert!(
        map.get(&integer(&registry, 2), &registry)
            .unwrap()
            .is_none()
    );
    assert_eq!(map.entries().len(), 1);
}

/// Verifies cloning produces independent storage with shared value ownership.
#[test]
fn clone_preserves_value_copy_on_write_ownership() {
    let registry = registry();
    let string_type = registry.type_by_name("string").unwrap();
    let mut source = MapValue::new();
    source
        .insert(
            integer(&registry, 1),
            Value::new(string_type, String::from("first")),
            &registry,
        )
        .unwrap();
    let mut clone = source.clone();

    assert!(!source.entries()[0].value.is_uniquely_owned());
    clone
        .insert(
            integer(&registry, 1),
            Value::new(string_type, String::from("second")),
            &registry,
        )
        .unwrap();

    assert_eq!(
        source.entries()[0].value.downcast_ref::<String>().unwrap(),
        "first"
    );
    assert_eq!(
        clone.entries()[0].value.downcast_ref::<String>().unwrap(),
        "second"
    );
}

/// Verifies mutable extraction follows the outer value's copy-on-write boundary.
#[test]
fn mutable_extraction_requires_unique_map_ownership() {
    let mut value = Value::new_map(MapType::dynamic(), MapValue::new());
    let clone = value.clone();

    assert!(MapValue::from_value_mut(&mut value).is_err());
    drop(clone);
    assert!(MapValue::from_value_mut(&mut value).is_ok());
}

/// Verifies map identity and payload mismatches produce stable errors.
#[test]
fn malformed_map_values_are_rejected() {
    let registry = registry();
    let scalar = integer(&registry, 1);
    let map_payload_with_scalar_identity =
        Value::new(registry.type_by_name("int64").unwrap(), MapValue::new());
    let malformed = Value::new_map(MapType::dynamic(), 1_i64);

    assert!(MapValue::from_value(&scalar).is_err());
    assert!(MapValue::from_value(&map_payload_with_scalar_identity).is_err());
    assert!(matches!(
        MapValue::from_value(&malformed),
        Err(CoreError::InvalidValueRepresentation(name)) if name == "map"
    ));
}
