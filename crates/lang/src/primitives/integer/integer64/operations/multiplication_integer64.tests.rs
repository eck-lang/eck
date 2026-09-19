use super::*;

use crate::semantic::Registry;

/// Builds a fully registered registry for context-aware promotion tests.
fn registered_registry_with_configuration() -> (Registry, crate::semantic::RuntimeConfiguration) {
    let mut registry = Registry::new();
    crate::register_all(&mut registry).unwrap();
    let configuration = registry.default_runtime_configuration();
    (registry, configuration)
}

/// Verifies integer multiplication and checked overflow handling.
#[test]
fn multiplies_integers_and_rejects_overflow() {
    let lhs = Value::new(crate::primitives::integer::integer64::test_type_id(), 6_i64);
    let rhs = Value::new(crate::primitives::integer::integer64::test_type_id(), 7_i64);
    let maximum = Value::new(
        crate::primitives::integer::integer64::test_type_id(),
        i64::MAX,
    );
    let two = Value::new(crate::primitives::integer::integer64::test_type_id(), 2_i64);

    let result = multiplication_integer(&lhs, &rhs).unwrap();

    assert_eq!(*result.downcast_ref::<i64>().unwrap(), 42);
    assert!(matches!(
        multiplication_integer(&maximum, &two),
        Err(CoreError::Runtime(message)) if message.contains("overflow")
    ));
}

/// Verifies mixed multiplication promotes both orders and checks `int64` overflow.
#[test]
fn multiplies_promoted_integer32_operands_as_integer64() {
    let wider_id = crate::primitives::integer::integer64::test_type_id();
    let narrower_id = crate::primitives::integer::integer32::test_type_id();
    let maximum = Value::new(wider_id, i64::MAX);
    let one = Value::new(narrower_id, 1_i32);
    let two = Value::new(narrower_id, 2_i32);

    for result in [
        multiplication_mixed_integer(&maximum, &one).unwrap(),
        multiplication_mixed_integer(&one, &maximum).unwrap(),
    ] {
        assert_eq!(result.type_id(), wider_id);
        assert_eq!(*result.downcast_ref::<i64>().unwrap(), i64::MAX);
    }
    assert!(matches!(
        multiplication_mixed_integer(&maximum, &two),
        Err(CoreError::Runtime(message)) if message.contains("overflow")
    ));
    let invalid = Value::new(narrower_id, false);
    assert!(matches!(
        multiplication_mixed_integer(&invalid, &maximum),
        Err(CoreError::InvalidValueRepresentation(_))
    ));
}

/// Verifies context-aware multiplication promotes `int64` overflow to `int128`.
#[test]
fn promotes_overflowed_context_multiplication_to_int128() {
    let (registry, configuration) = registered_registry_with_configuration();
    let context = ExecutionContext::new(&registry, &configuration);
    let integer64_id = registry.type_by_name("int64").unwrap();
    let integer128_id = registry.type_by_name("int128").unwrap();
    let operator = registry
        .resolve_binary_operator(BinaryOperator::Multiplication, integer64_id, integer64_id)
        .unwrap();
    let descriptor = registry.operator(operator).unwrap();
    let execute = descriptor.context_execute.unwrap();

    let in_range = execute(
        &context,
        &Value::new(integer64_id, 3_i64),
        &Value::new(integer64_id, 14_i64),
    )
    .unwrap();
    assert_eq!(in_range.type_id(), integer64_id);
    assert_eq!(*in_range.downcast_ref::<i64>().unwrap(), 42);

    let promoted = execute(
        &context,
        &Value::new(integer64_id, 4_611_686_018_427_387_904_i64),
        &Value::new(integer64_id, 2_i64),
    )
    .unwrap();
    assert_eq!(promoted.type_id(), integer128_id);
    assert_eq!(
        *promoted.downcast_ref::<i128>().unwrap(),
        9_223_372_036_854_775_808_i128
    );
}

/// Verifies context-aware mixed multiplication promotes `int64` overflow to `int128`.
#[test]
fn promotes_overflowed_mixed_context_multiplication_to_int128() {
    let (registry, configuration) = registered_registry_with_configuration();
    let context = ExecutionContext::new(&registry, &configuration);
    let integer64_id = registry.type_by_name("int64").unwrap();
    let integer32_id = registry.type_by_name("int32").unwrap();
    let integer128_id = registry.type_by_name("int128").unwrap();
    let operator = registry
        .resolve_binary_operator(BinaryOperator::Multiplication, integer64_id, integer32_id)
        .unwrap();
    let descriptor = registry.operator(operator).unwrap();

    let promoted = descriptor.context_execute.unwrap()(
        &context,
        &Value::new(integer64_id, 4_611_686_018_427_387_904_i64),
        &Value::new(integer32_id, 2_i32),
    )
    .unwrap();
    assert_eq!(promoted.type_id(), integer128_id);
    assert_eq!(
        *promoted.downcast_ref::<i128>().unwrap(),
        9_223_372_036_854_775_808_i128
    );
}
