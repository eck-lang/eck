use super::*;
use crate::containers::map::MapType;

/// Creates a scalar value for one registered primitive name.
fn value<T: Send + Sync + 'static>(registry: &Registry, name: &str, payload: T) -> Value {
    Value::new(registry.type_by_name(name).unwrap(), payload)
}

/// Verifies rendering follows insertion order after replacement.
#[test]
fn formats_entries_in_deterministic_insertion_order() {
    let registry = crate::default_registry().unwrap();
    let mut map = MapValue::new();
    map.insert(
        value(&registry, "string", String::from("first")),
        value(&registry, "int64", 1_i64),
        &registry,
    )
    .unwrap();
    map.insert(
        value(&registry, "string", String::from("second")),
        value(&registry, "int64", 2_i64),
        &registry,
    )
    .unwrap();
    map.insert(
        value(&registry, "string", String::from("first")),
        value(&registry, "int64", 3_i64),
        &registry,
    )
    .unwrap();
    let value = Value::new_map(MapType::dynamic(), map);
    let configuration = registry.default_runtime_configuration();

    assert_eq!(
        format_value(&registry, &value, &configuration).unwrap(),
        "{\n    \"first\": 3,\n    \"second\": 2\n}"
    );
}

/// Verifies nested maps recurse through the same deterministic formatter.
#[test]
fn formats_nested_maps() {
    let registry = crate::default_registry().unwrap();
    let mut inner = MapValue::new();
    inner
        .insert(
            value(&registry, "bool", true),
            value(&registry, "int64", 1_i64),
            &registry,
        )
        .unwrap();
    let mut outer = MapValue::new();
    outer
        .insert(
            value(&registry, "string", String::from("nested")),
            Value::new_map(MapType::dynamic(), inner),
            &registry,
        )
        .unwrap();
    let value = Value::new_map(MapType::dynamic(), outer);

    assert_eq!(
        format_value(&registry, &value, &registry.default_runtime_configuration()).unwrap(),
        "{\n    \"nested\": {\n        true: 1\n    }\n}"
    );
}

/// Verifies map strings use escaped quoted source syntax.
#[test]
fn quotes_and_escapes_string_entries() {
    let registry = crate::default_registry().unwrap();
    let mut map = MapValue::new();
    map.insert(
        value(&registry, "string", String::from("line\n\"quoted\"")),
        value(&registry, "string", String::from("path\\value")),
        &registry,
    )
    .unwrap();
    let value = Value::new_map(MapType::dynamic(), map);

    assert_eq!(
        format_value(&registry, &value, &registry.default_runtime_configuration()).unwrap(),
        "{\n    \"line\\n\\\"quoted\\\"\": \"path\\\\value\"\n}"
    );
}
