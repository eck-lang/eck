use super::*;
use crate::ConfigurationOverride;

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
    first_execution.apply(&ConfigurationOverride::new(vec![(
        "example.limit".into(),
        normalized,
    )]));

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
