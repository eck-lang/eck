use num_bigint::BigInt;
use rust_decimal::Decimal;

use super::*;
use crate::containers::array::{ArrayType, ArrayValue};
use crate::containers::map::{MapType, MapValue};

/// Builds the complete built-in registry used by map-key tests.
fn registry() -> Registry {
    crate::default_registry().expect("the built-in registry should initialize")
}

/// Builds a scalar value for one registered primitive name.
fn value<T: Send + Sync + 'static>(registry: &Registry, name: &str, payload: T) -> Value {
    Value::new(
        registry
            .type_by_name(name)
            .expect("the primitive should be registered"),
        payload,
    )
}

/// Verifies compiler-facing support covers every hashable built-in scalar.
#[test]
fn reports_supported_scalar_types_and_qualified_identities() {
    let registry = registry();
    for name in SUPPORTED_TYPE_NAMES {
        let value_type = ValueType::plain(registry.type_by_name(name).unwrap());
        assert!(MapKey::supports_type(&registry, value_type), "{name}");
    }

    let millimeter = registry.subtype_by_suffix("mm").unwrap();
    let qualified = ValueType::qualified(registry.type_by_name("int64").unwrap(), millimeter);
    assert!(MapKey::supports_type(&registry, qualified));
    assert!(!MapKey::supports_type(
        &registry,
        ValueType::plain(registry.type_by_name("null").unwrap())
    ));
}

/// Verifies equality includes the complete value type and concrete payload width.
#[test]
fn preserves_strict_typed_identity() {
    let registry = registry();
    let int8 = MapKey::from_value(&value(&registry, "int8", 7_i8), &registry).unwrap();
    let int64 = MapKey::from_value(&value(&registry, "int64", 7_i64), &registry).unwrap();
    let millimeter = registry.subtype_by_suffix("mm").unwrap();
    let qualified_value = value(&registry, "int64", 7_i64).with_subtype(Some(millimeter));
    let qualified = MapKey::from_value(&qualified_value, &registry).unwrap();

    assert_ne!(int8, int64);
    assert_ne!(int64, qualified);
}

/// Verifies float keys retain infinities, merge signed zero, and reject NaN.
#[test]
fn handles_floating_point_edge_cases() {
    let registry = registry();
    let positive_zero = MapKey::from_value(&value(&registry, "float", 0.0_f32), &registry).unwrap();
    let negative_zero =
        MapKey::from_value(&value(&registry, "float", -0.0_f32), &registry).unwrap();
    let infinity = MapKey::from_value(&value(&registry, "double", f64::INFINITY), &registry);
    let negative_infinity =
        MapKey::from_value(&value(&registry, "double", f64::NEG_INFINITY), &registry);
    let nan = MapKey::from_value(&value(&registry, "double", f64::NAN), &registry);

    assert_eq!(positive_zero, negative_zero);
    assert_ne!(infinity.unwrap(), negative_infinity.unwrap());
    assert!(matches!(nan, Err(CoreError::Runtime(message)) if message.contains("NaN")));
}

/// Verifies decimal and arbitrary-precision payloads use native equality and hashing.
#[test]
fn uses_native_decimal_and_bigint_identity() {
    let registry = registry();
    let decimal_one =
        MapKey::from_value(&value(&registry, "decimal", Decimal::new(10, 1)), &registry).unwrap();
    let decimal_one_with_scale = MapKey::from_value(
        &value(&registry, "decimal", Decimal::new(100, 2)),
        &registry,
    )
    .unwrap();
    let bigint = BigInt::parse_bytes(b"999999999999999999999999", 10).unwrap();
    let bigint_key =
        MapKey::from_value(&value(&registry, "bigint", bigint.clone()), &registry).unwrap();

    assert_eq!(decimal_one, decimal_one_with_scale);
    assert_eq!(
        bigint_key,
        MapKey::from_value(&value(&registry, "bigint", bigint), &registry).unwrap()
    );
}

/// Verifies unsupported containers, null, and unknown payloads fail cleanly.
#[test]
fn rejects_unsupported_keys() {
    let registry = registry();
    let array = Value::new_array(ArrayType::dynamic(), ArrayValue::new(Vec::new()));
    let map = Value::new_map(MapType::dynamic(), MapValue::new());
    let null = registry.parse_null("null", None).unwrap();
    let unsupported = value(&registry, "int64", 'x');

    assert!(MapKey::from_value(&array, &registry).is_err());
    assert!(MapKey::from_value(&map, &registry).is_err());
    assert!(MapKey::from_value(&null, &registry).is_err());
    assert!(MapKey::from_value(&unsupported, &registry).is_err());
}
