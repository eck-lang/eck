use super::*;

use crate::semantic::Registry;

/// Builds a fully registered registry for context-aware promotion tests.
fn registered_registry_with_configuration() -> (Registry, crate::semantic::RuntimeConfiguration) {
    let mut registry = Registry::new();
    crate::register_all(&mut registry).unwrap();
    let configuration = registry.default_runtime_configuration();
    (registry, configuration)
}

/// Verifies integer division, zero-divisor rejection, and checked overflow.
#[test]
fn divides_integers_and_rejects_zero_and_overflow() {
    let lhs = Value::new(
        crate::primitives::integer::integer64::test_type_id(),
        43_i64,
    );
    let rhs = Value::new(crate::primitives::integer::integer64::test_type_id(), 5_i64);
    let zero = Value::new(crate::primitives::integer::integer64::test_type_id(), 0_i64);
    let minimum = Value::new(
        crate::primitives::integer::integer64::test_type_id(),
        i64::MIN,
    );
    let negative_one = Value::new(
        crate::primitives::integer::integer64::test_type_id(),
        -1_i64,
    );

    let result = division_integer(&lhs, &rhs).unwrap();

    assert_eq!(*result.downcast_ref::<i64>().unwrap(), 8);
    assert!(matches!(
        division_integer(&lhs, &zero),
        Err(CoreError::DivisionByZero)
    ));
    assert!(matches!(
        division_integer(&minimum, &negative_one),
        Err(CoreError::Runtime(message)) if message.contains("overflow")
    ));
}

/// Verifies mixed division preserves order, output type, zero checks, and overflow.
#[test]
fn divides_promoted_integer32_operands_as_integer64() {
    let wider_id = crate::primitives::integer::integer64::test_type_id();
    let narrower_id = crate::primitives::integer::integer32::test_type_id();
    let wide = Value::new(wider_id, 43_i64);
    let five = Value::new(narrower_id, 5_i32);
    let zero = Value::new(narrower_id, 0_i32);
    let minimum = Value::new(wider_id, i64::MIN);
    let negative_one = Value::new(narrower_id, -1_i32);

    let wide_left = division_mixed_integer(&wide, &five).unwrap();
    let narrow_left = division_mixed_integer(&five, &wide).unwrap();
    assert_eq!(
        (
            wide_left.type_id(),
            *wide_left.downcast_ref::<i64>().unwrap()
        ),
        (wider_id, 8)
    );
    assert_eq!(
        (
            narrow_left.type_id(),
            *narrow_left.downcast_ref::<i64>().unwrap()
        ),
        (wider_id, 0)
    );
    assert!(matches!(
        division_mixed_integer(&wide, &zero),
        Err(CoreError::DivisionByZero)
    ));
    assert!(matches!(
        division_mixed_integer(&minimum, &negative_one),
        Err(CoreError::Runtime(message)) if message.contains("overflow")
    ));
    let invalid = Value::new(narrower_id, false);
    assert!(matches!(
        division_mixed_integer(&invalid, &wide),
        Err(CoreError::InvalidValueRepresentation(_))
    ));
}

/// Verifies context-aware division promotes the `MIN / -1` overflow to `int128`.
#[test]
fn promotes_overflowed_context_division_to_int128() {
    let (registry, configuration) = registered_registry_with_configuration();
    let context = ExecutionContext::new(&registry, &configuration);
    let integer64_id = registry.type_by_name("int64").unwrap();
    let integer128_id = registry.type_by_name("int128").unwrap();
    let operator = registry
        .resolve_binary_operator(BinaryOperator::Division, integer64_id, integer64_id)
        .unwrap();
    let descriptor = registry.operator(operator).unwrap();
    let execute = descriptor.context_execute.unwrap();

    let in_range = execute(
        &context,
        &Value::new(integer64_id, 84_i64),
        &Value::new(integer64_id, 2_i64),
    )
    .unwrap();
    assert_eq!(in_range.type_id(), integer64_id);
    assert_eq!(*in_range.downcast_ref::<i64>().unwrap(), 42);

    let promoted = execute(
        &context,
        &Value::new(integer64_id, i64::MIN),
        &Value::new(integer64_id, -1_i64),
    )
    .unwrap();
    assert_eq!(promoted.type_id(), integer128_id);
    assert_eq!(
        *promoted.downcast_ref::<i128>().unwrap(),
        9_223_372_036_854_775_808_i128
    );

    assert!(matches!(
        execute(
            &context,
            &Value::new(integer64_id, 1_i64),
            &Value::new(integer64_id, 0_i64),
        ),
        Err(CoreError::DivisionByZero)
    ));
}

/// Verifies context-aware mixed division promotes `int64` overflow to `int128`.
#[test]
fn promotes_overflowed_mixed_context_division_to_int128() {
    let (registry, configuration) = registered_registry_with_configuration();
    let context = ExecutionContext::new(&registry, &configuration);
    let integer64_id = registry.type_by_name("int64").unwrap();
    let integer32_id = registry.type_by_name("int32").unwrap();
    let integer128_id = registry.type_by_name("int128").unwrap();
    let operator = registry
        .resolve_binary_operator(BinaryOperator::Division, integer64_id, integer32_id)
        .unwrap();
    let descriptor = registry.operator(operator).unwrap();

    let promoted = descriptor.context_execute.unwrap()(
        &context,
        &Value::new(integer64_id, i64::MIN),
        &Value::new(integer32_id, -1_i32),
    )
    .unwrap();
    assert_eq!(promoted.type_id(), integer128_id);
    assert_eq!(
        *promoted.downcast_ref::<i128>().unwrap(),
        9_223_372_036_854_775_808_i128
    );
}
