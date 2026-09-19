use std::str::FromStr;

use crate::semantic::{ConfigurationOverride, Extension, Registry};
use rust_decimal::Decimal;

use super::*;
use crate::DecimalExtension;

/// Builds a registry containing the decimal extension and its configuration schema.
fn decimal_registry() -> Registry {
    let mut registry = Registry::new();
    DecimalExtension.register(&mut registry).unwrap();
    registry
}

/// Builds a decimal runtime value using the registered decimal type identifier.
fn decimal_value(registry: &Registry, source: &str) -> Value {
    Value::new(
        registry.type_by_name("decimal").unwrap(),
        Decimal::from_str(source).unwrap(),
    )
}

/// Verifies maximum arithmetic defaults and exact literal/result preservation.
#[test]
fn default_precision_rounds_results_but_not_literals() {
    let registry = decimal_registry();
    let literal = decimal_value(&registry, "123.456789");
    let configuration = registry.default_runtime_configuration();

    assert_eq!(get(&literal).unwrap().to_string(), "123.456789");
    assert_eq!(
        get(&transform_result(&literal, &configuration).unwrap())
            .unwrap()
            .to_string(),
        "123.456789"
    );
    assert_eq!(
        get(
            &transform_result(&decimal_value(&registry, "1000001.123456"), &configuration).unwrap()
        )
        .unwrap()
        .to_string(),
        "1000001.123456"
    );
}

/// Verifies that `Max` and overly large integer precisions normalize to decimal capacity.
#[test]
fn caps_precision_to_the_decimal_capacity() {
    let registry = decimal_registry();

    assert_eq!(
        registry
            .normalize_configuration_value(PRECISION_PATH, ConfigurationValue::Symbol("Max".into()))
            .unwrap(),
        ConfigurationValue::Integer(MAXIMUM_PRECISION)
    );
    assert_eq!(
        registry
            .normalize_configuration_value(PRECISION_PATH, ConfigurationValue::Integer(29))
            .unwrap(),
        ConfigurationValue::Integer(MAXIMUM_PRECISION)
    );
    assert!(
        registry
            .normalize_configuration_value(PRECISION_PATH, ConfigurationValue::Integer(0))
            .is_err()
    );
    assert_eq!(
        registry
            .normalize_configuration_value(SCALE_PATH, ConfigurationValue::Symbol("Max".into()))
            .unwrap(),
        ConfigurationValue::Integer(MAXIMUM_PRECISION)
    );
}

/// Verifies that precision limits total significant digits independently of scale.
#[test]
fn applies_significant_digit_precision_independently_of_scale() {
    let registry = decimal_registry();
    let mut configuration = registry.default_runtime_configuration();
    configuration.apply(&ConfigurationOverride::new(vec![
        (PRECISION_PATH.into(), ConfigurationValue::Integer(4)),
        (
            SCALE_PATH.into(),
            ConfigurationValue::Integer(MAXIMUM_PRECISION),
        ),
    ]));

    assert_eq!(
        get(&transform_result(&decimal_value(&registry, "123.456789"), &configuration).unwrap())
            .unwrap()
            .to_string(),
        "123.4"
    );
}

/// Verifies midpoint behaviour for both positive and negative decimal results.
#[test]
fn applies_the_active_decimal_place_rounding_strategy() {
    let registry = decimal_registry();
    let mut configuration = registry.default_runtime_configuration();
    configuration.apply(&ConfigurationOverride::new(vec![
        (
            PRECISION_PATH.into(),
            ConfigurationValue::Integer(MAXIMUM_PRECISION),
        ),
        (SCALE_PATH.into(), ConfigurationValue::Integer(2)),
        (
            ROUNDING_PATH.into(),
            ConfigurationValue::Symbol("HalfUp".into()),
        ),
    ]));

    assert_eq!(
        get(&transform_result(&decimal_value(&registry, "1.235"), &configuration).unwrap())
            .unwrap()
            .to_string(),
        "1.24"
    );
    assert_eq!(
        get(&transform_result(&decimal_value(&registry, "-1.235"), &configuration).unwrap())
            .unwrap()
            .to_string(),
        "-1.24"
    );

    configuration.apply(&ConfigurationOverride::new(vec![(
        SCALE_PATH.into(),
        ConfigurationValue::Integer(0),
    )]));
    assert_eq!(
        get(&transform_result(&decimal_value(&registry, "12.6"), &configuration).unwrap())
            .unwrap()
            .to_string(),
        "13"
    );
}

/// Verifies that unrestricted formatting returns the stored representation unchanged.
#[test]
fn none_formatting_does_not_limit_decimal_output() {
    let registry = decimal_registry();
    let mut configuration = registry.default_runtime_configuration();
    configuration.apply(&ConfigurationOverride::new(vec![(
        FORMAT_SCALE_PATH.into(),
        ConfigurationValue::None,
    )]));

    assert_eq!(
        format(&decimal_value(&registry, "123.456789"), &configuration).unwrap(),
        "123.456789"
    );
}

/// Verifies scale-limited display rounding and truncation without mutating the source value.
#[test]
fn scale_formatting_is_non_destructive_and_can_truncate_toward_zero() {
    let registry = decimal_registry();
    let value = decimal_value(&registry, "123.456789");
    let mut configuration = registry.default_runtime_configuration();
    configuration.apply(&ConfigurationOverride::new(vec![
        (FORMAT_SCALE_PATH.into(), ConfigurationValue::Integer(4)),
        (
            FORMAT_ROUNDING_PATH.into(),
            ConfigurationValue::Symbol("HalfEven".into()),
        ),
    ]));

    assert_eq!(format(&value, &configuration).unwrap(), "123.4568");
    assert_eq!(get(&value).unwrap().to_string(), "123.456789");

    configuration.apply(&ConfigurationOverride::new(vec![(
        FORMAT_ROUNDING_PATH.into(),
        ConfigurationValue::Symbol("Truncate".into()),
    )]));
    assert_eq!(format(&value, &configuration).unwrap(), "123.4567");
    assert_eq!(
        format(&decimal_value(&registry, "-123.456789"), &configuration).unwrap(),
        "-123.4567"
    );
}

/// Verifies that independent execution configurations do not share modal overrides.
#[test]
fn execution_configurations_are_isolated() {
    let registry = decimal_registry();
    let mut configured_execution = registry.default_runtime_configuration();
    let unconfigured_execution = registry.default_runtime_configuration();
    configured_execution.apply(&ConfigurationOverride::new(vec![(
        SCALE_PATH.into(),
        ConfigurationValue::Integer(2),
    )]));
    let value = decimal_value(&registry, "12.3456");

    assert_eq!(
        get(&transform_result(&value, &configured_execution).unwrap())
            .unwrap()
            .to_string(),
        "12.34"
    );
    assert_eq!(
        get(&transform_result(&value, &unconfigured_execution).unwrap())
            .unwrap()
            .to_string(),
        "12.3456"
    );
}
