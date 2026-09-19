use crate::semantic::{CoreError, Registry, TypeDescriptor, TypeId, Value, ValueType};

use super::*;

/// Formats the test payload as a decimal magnitude.
fn format_number(value: &Value) -> Result<String, CoreError> {
    let number = value
        .downcast_ref::<i64>()
        .ok_or_else(|| CoreError::InvalidValueRepresentation("test number".into()))?;
    Ok(number.to_string())
}

/// Parses any decimal magnitude used by the wide test type.
fn parse_wide(raw_text: &str, type_id: TypeId) -> Result<Value, CoreError> {
    let number = raw_text
        .parse::<i64>()
        .map_err(|error| CoreError::Runtime(error.to_string()))?;
    Ok(Value::new(type_id, number))
}

/// Parses a single-digit magnitude used by the narrow test type.
fn parse_narrow(raw_text: &str, type_id: TypeId) -> Result<Value, CoreError> {
    let number = raw_text
        .parse::<i64>()
        .map_err(|error| CoreError::Runtime(error.to_string()))?;
    if !(0..=9).contains(&number) {
        return Err(CoreError::Runtime(format!("`{raw_text}` is out of range")));
    }
    Ok(Value::new(type_id, number))
}

/// Registers two integer-shaped representations that differ in range.
fn registry() -> (Registry, TypeId, TypeId) {
    let mut registry = Registry::new();
    let wide = registry.allocate_type_id();
    let narrow = registry.allocate_type_id();
    for (id, name, parser) in [
        (wide, "wide", parse_wide as crate::semantic::LiteralParser),
        (narrow, "narrow", parse_narrow),
    ] {
        registry
            .register_type(TypeDescriptor {
                id,
                name,
                is_integer: true,
                parse_numeric_literal: Some(parser),
                parse_string_literal: None,
                parse_regex_literal: None,
                parse_boolean_literal: None,
                parse_null_literal: None,
                format: format_number,
            })
            .expect("the test type registers");
    }
    (registry, wide, narrow)
}

/// Verifies an element already in the declared representation crosses unchanged.
#[test]
fn an_exact_element_crosses_without_conversion() {
    let (registry, wide, _) = registry();
    let configuration = registry.default_runtime_configuration();
    let value = Value::new(wide, 7_i64);

    let stored = apply_element_contract(&registry, &configuration, value, ValueType::plain(wide))
        .expect("the value already carries the declared representation");

    assert_eq!(stored.type_id(), wide);
    assert_eq!(*stored.downcast_ref::<i64>().unwrap(), 7);
}

/// Verifies an element is normalized into the declared representation.
#[test]
fn a_promoted_element_is_normalized() {
    let (registry, wide, narrow) = registry();
    let configuration = registry.default_runtime_configuration();
    let value = Value::new(wide, 7_i64);

    let stored = apply_element_contract(&registry, &configuration, value, ValueType::plain(narrow))
        .expect("the narrow representation holds the value");

    assert_eq!(stored.type_id(), narrow);
    assert_eq!(*stored.downcast_ref::<i64>().unwrap(), 7);
}

/// Verifies a value the declared representation cannot hold is rejected.
#[test]
fn an_unrepresentable_element_is_rejected() {
    let (registry, wide, narrow) = registry();
    let configuration = registry.default_runtime_configuration();
    let value = Value::new(wide, 42_i64);

    let error =
        match apply_element_contract(&registry, &configuration, value, ValueType::plain(narrow)) {
            Ok(_) => panic!("the narrow representation cannot hold the value"),
            Err(error) => error,
        };

    assert!(matches!(
        error,
        CoreError::ArrayElementNotRepresentable { ref value, ref element }
            if value == "42" && element == "narrow"
    ));
    assert_eq!(
        error.to_string(),
        "array element `42` cannot be represented as `narrow`"
    );
}
