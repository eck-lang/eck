use super::*;
use crate::semantic::{
    ArrayElementMode, ArrayType, ConfigurationOverride, RuntimeConfiguration, SemanticType, Value,
    ValueType,
};

/// A payload used to exercise array formatting without depending on the array crate.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct FakeArrayPayload(&'static str);

/// Formats the fake array payload while checking that Core passes array identity through.
fn format_fake_array(
    _: &Registry,
    value: &Value,
    array_type: ArrayType,
    _: &RuntimeConfiguration,
) -> Result<String, CoreError> {
    let payload = value
        .downcast_ref::<FakeArrayPayload>()
        .ok_or_else(|| CoreError::InvalidValueRepresentation("fake array".into()))?;
    if value.semantic_type() != SemanticType::Array(array_type) {
        return Err(CoreError::InvalidValueRepresentation(
            "array identity changed".into(),
        ));
    }
    Ok(payload.0.to_string())
}

/// Keeps an integer configuration value unchanged for registry-level tests.
fn normalize_integer(value: ConfigurationValue) -> Result<ConfigurationValue, CoreError> {
    match value {
        ConfigurationValue::Integer(value) => Ok(ConfigurationValue::Integer(value)),
        _ => Err(CoreError::InvalidConfigurationValue(
            "expected an integer".into(),
        )),
    }
}

/// Accepts an optional integer configuration leaf for object-level `None` tests.
fn normalize_optional_integer(value: ConfigurationValue) -> Result<ConfigurationValue, CoreError> {
    match value {
        ConfigurationValue::Integer(value) => Ok(ConfigurationValue::Integer(value)),
        ConfigurationValue::None => Ok(ConfigurationValue::None),
        ConfigurationValue::Symbol(_) => Err(CoreError::InvalidConfigurationValue(
            "expected an integer or None".into(),
        )),
    }
}

/// Registers a minimal integer leaf configuration used by the tests in this module.
fn register_integer_configuration(registry: &mut Registry) {
    registry
        .register_configuration(ConfigurationDescriptor {
            path: "example.limit",
            none_object_path: None,
            default: ConfigurationValue::Integer(4),
            normalize: normalize_integer,
        })
        .unwrap();
}

/// Verifies default creation, normalization, and modal merging of one configuration leaf.
#[test]
fn creates_execution_local_defaults_and_applies_overrides() {
    let mut registry = Registry::new();
    register_integer_configuration(&mut registry);
    let mut first_execution = registry.default_runtime_configuration();
    let second_execution = registry.default_runtime_configuration();
    let normalized = registry
        .normalize_configuration_value("example.limit", ConfigurationValue::Integer(7))
        .unwrap();

    assert!(first_execution.uses_initial_values());
    assert!(second_execution.uses_initial_values());
    first_execution.apply(&ConfigurationOverride::new(vec![(
        "example.limit".into(),
        normalized,
    )]));

    assert!(!first_execution.uses_initial_values());
    assert!(second_execution.uses_initial_values());

    assert_eq!(
        first_execution.value("example.limit"),
        Some(&ConfigurationValue::Integer(7))
    );
    assert_eq!(
        second_execution.value("example.limit"),
        Some(&ConfigurationValue::Integer(4))
    );
}

/// Verifies deterministic errors for duplicate and unknown configuration paths.
#[test]
fn rejects_duplicate_and_unknown_configuration_paths() {
    let mut registry = Registry::new();
    register_integer_configuration(&mut registry);

    assert!(matches!(
        registry.register_configuration(ConfigurationDescriptor {
            path: "example.limit",
            none_object_path: None,
            default: ConfigurationValue::Integer(4),
            normalize: normalize_integer,
        }),
        Err(CoreError::DuplicateConfiguration(path)) if path == "example.limit"
    ));
    assert!(matches!(
        registry.normalize_configuration_value("example.unknown", ConfigurationValue::Integer(1)),
        Err(CoreError::UnknownConfiguration(path)) if path == "example.unknown"
    ));
}

/// Verifies that only explicitly registered object paths can use `None`.
#[test]
fn resolves_explicit_none_objects_without_adding_reset_behavior() {
    let mut registry = Registry::new();
    registry
        .register_configuration(ConfigurationDescriptor {
            path: "example.limit.enabled",
            none_object_path: Some("example"),
            default: ConfigurationValue::Integer(4),
            normalize: normalize_optional_integer,
        })
        .unwrap();

    assert_eq!(
        registry
            .normalize_none_configuration_value("example")
            .unwrap(),
        ("example.limit.enabled".into(), ConfigurationValue::None)
    );
    assert!(matches!(
        registry.normalize_none_configuration_value("other"),
        Err(CoreError::UnknownConfiguration(path)) if path == "other"
    ));
}

/// Verifies a configured type explicitly exposes its initial-result identity.
#[test]
fn reports_initial_result_transform_identity_for_registered_types() {
    let mut registry = Registry::new();
    let type_id =
        crate::semantic::registry::test_support::register_type(&mut registry, "configured");
    registry
        .register_type_configuration(
            type_id,
            TypeConfigurationDescriptor {
                transform_result: None,
                transform_owned_result: None,
                initial_result_transform_is_identity: true,
                format: None,
            },
        )
        .unwrap();

    assert!(
        registry
            .initial_result_transform_is_identity(type_id)
            .unwrap()
    );
}

/// Verifies arrays require exactly one registered formatter and dispatch through it.
#[test]
fn registers_and_dispatches_the_array_formatter() {
    let mut registry = Registry::new();
    let array_type = ArrayType {
        element: ValueType::plain(registry.allocate_type_id()),
        element_mode: ArrayElementMode::Exact,
    };
    let value = Value::new_array(array_type, FakeArrayPayload("[fake]"));

    assert!(matches!(
        registry.format_value(&value),
        Err(CoreError::MissingArrayFormatter)
    ));
    registry
        .register_array_formatter(format_fake_array)
        .unwrap();
    assert!(matches!(
        registry.register_array_formatter(format_fake_array),
        Err(CoreError::DuplicateArrayFormatter)
    ));
    assert_eq!(registry.format_value(&value).unwrap(), "[fake]");
}

/// Verifies array result transforms preserve the array identity and opaque payload.
#[test]
fn configured_result_transforms_leave_arrays_unchanged() {
    let mut registry = Registry::new();
    let element_type =
        crate::semantic::registry::test_support::register_type(&mut registry, "element");
    let array_type = ArrayType {
        element: ValueType::plain(element_type),
        element_mode: ArrayElementMode::AdaptiveInt,
    };
    let value = Value::new_array(array_type, FakeArrayPayload("payload"));
    let configuration = registry.default_runtime_configuration();

    let transformed = registry
        .transform_configured_result(&value, &configuration)
        .unwrap();
    let transformed_owned = registry
        .transform_owned_configured_result(value.clone(), &configuration)
        .unwrap();

    assert_eq!(transformed.semantic_type(), SemanticType::Array(array_type));
    assert_eq!(
        transformed_owned.semantic_type(),
        SemanticType::Array(array_type)
    );
    assert_eq!(
        transformed.downcast_ref::<FakeArrayPayload>(),
        Some(&FakeArrayPayload("payload"))
    );
    assert_eq!(
        transformed_owned.downcast_ref::<FakeArrayPayload>(),
        Some(&FakeArrayPayload("payload"))
    );
}
