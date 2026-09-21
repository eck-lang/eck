use super::*;

use crate::semantic::{
    ArrayType, ConfigurationDescriptor, ConfigurationValue, CoreError, Registry,
    RuntimeConfiguration, ScalarRepresentation, SemanticType, TypeConfigurationDescriptor,
    TypeDescriptor, Value, ValueType,
};

/// Formats the integer payload used to prove recursive array dispatch.
fn format_integer(value: &Value) -> Result<String, CoreError> {
    Ok(value
        .downcast_ref::<i64>()
        .expect("the fixture stores an integer")
        .to_string())
}

/// Keeps a symbolic suffix configuration unchanged for the contextual formatter test.
fn normalize_symbol(value: ConfigurationValue) -> Result<ConfigurationValue, CoreError> {
    match value {
        ConfigurationValue::Symbol(_) => Ok(value),
        _ => Err(CoreError::InvalidConfigurationValue(
            "expected a symbol".into(),
        )),
    }
}

/// Formats an integer with the active execution-scoped suffix configuration.
fn format_configured_integer(
    value: &Value,
    configuration: &RuntimeConfiguration,
) -> Result<String, CoreError> {
    let integer = value
        .downcast_ref::<i64>()
        .expect("the fixture stores an integer");
    let suffix = match configuration.value("test.suffix") {
        Some(ConfigurationValue::Symbol(suffix)) => suffix,
        _ => "",
    };
    Ok(format!("{integer}{suffix}"))
}

/// Verifies the extension only installs array formatting and no scalar type.
#[test]
fn extension_registers_array_formatting_without_a_type_id() {
    let mut registry = Registry::new();
    let registered_type_count = registry.registered_type_count();
    ArrayExtension.register(&mut registry).unwrap();
    assert_eq!(registry.registered_type_count(), registered_type_count);

    let element_type = registry.allocate_type_id();
    registry
        .register_type(TypeDescriptor {
            id: element_type,
            name: "int",
            is_integer: true,
            parse_numeric_literal: None,
            parse_string_literal: None,
            parse_regex_literal: None,
            parse_boolean_literal: None,
            parse_null_literal: None,
            format: format_integer,
        })
        .unwrap();
    let array_type = ArrayType::static_element(
        SemanticType::Scalar(ValueType::plain(element_type)),
        ScalarRepresentation::Exact,
    );
    let value = Value::new_array(
        array_type.clone(),
        ArrayValue::new(vec![Value::new(element_type, 1_i64)]),
    );

    assert_eq!(
        value.semantic_type(),
        SemanticType::array(array_type.clone())
    );
    assert_eq!(registry.registered_type_count(), registered_type_count + 1);
    assert_eq!(registry.format_value(&value).unwrap(), "[1]");
}

/// Verifies recursive array formatting forwards the active scalar context.
#[test]
fn formatter_recurses_with_the_active_configuration() {
    let mut registry = Registry::new();
    let element_type = registry.allocate_type_id();
    registry
        .register_type(TypeDescriptor {
            id: element_type,
            name: "int",
            is_integer: true,
            parse_numeric_literal: None,
            parse_string_literal: None,
            parse_regex_literal: None,
            parse_boolean_literal: None,
            parse_null_literal: None,
            format: format_integer,
        })
        .unwrap();
    registry
        .register_configuration(ConfigurationDescriptor {
            path: "test.suffix",
            none_object_path: None,
            default: ConfigurationValue::Symbol("!".into()),
            normalize: normalize_symbol,
        })
        .unwrap();
    registry
        .register_type_configuration(
            element_type,
            TypeConfigurationDescriptor {
                transform_result: None,
                transform_owned_result: None,
                initial_result_transform_is_identity: true,
                format: Some(format_configured_integer),
            },
        )
        .unwrap();
    ArrayExtension.register(&mut registry).unwrap();

    let array_type = ArrayType::static_element(
        SemanticType::Scalar(ValueType::plain(element_type)),
        ScalarRepresentation::Exact,
    );
    let value = Value::new_array(
        array_type,
        ArrayValue::new(vec![Value::new(element_type, 1_i64)]),
    );
    let mut configuration = registry.default_runtime_configuration();

    assert_eq!(
        registry
            .format_value_with_configuration(&value, &configuration)
            .unwrap(),
        "[1!]"
    );
    let suffix = registry
        .normalize_configuration_value("test.suffix", ConfigurationValue::Symbol("?".into()))
        .unwrap();
    configuration.apply(&crate::semantic::ConfigurationOverride::new(vec![(
        "test.suffix".into(),
        suffix,
    )]));
    assert_eq!(
        registry
            .format_value_with_configuration(&value, &configuration)
            .unwrap(),
        "[1?]"
    );
}
