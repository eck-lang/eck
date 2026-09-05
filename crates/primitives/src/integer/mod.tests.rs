use language_core::{BinaryOperator, ComparisonOperator, CoreError, Extension, Registry, Value};
use num_bigint::BigInt;

use super::*;

const SIGNED_INTEGER_TYPES: [&str; 6] = ["int8", "int16", "int32", "int64", "int128", "bigint"];
const BINARY_OPERATORS: [BinaryOperator; 6] = [
    BinaryOperator::Addition,
    BinaryOperator::Subtraction,
    BinaryOperator::Multiplication,
    BinaryOperator::Division,
    BinaryOperator::Remainder,
    BinaryOperator::Power,
];
const COMPARISON_OPERATORS: [ComparisonOperator; 6] = [
    ComparisonOperator::Equal,
    ComparisonOperator::NotEqual,
    ComparisonOperator::Less,
    ComparisonOperator::LessOrEqual,
    ComparisonOperator::Greater,
    ComparisonOperator::GreaterOrEqual,
];

/// Creates a correctly represented signed fixed-width integer value by type name.
fn signed_integer_value(registry: &Registry, type_name: &str, value: i128) -> Value {
    let type_id = registry.type_by_name(type_name).unwrap();
    match type_name {
        "int8" => Value::new(type_id, i8::try_from(value).unwrap()),
        "int16" => Value::new(type_id, i16::try_from(value).unwrap()),
        "int32" => Value::new(type_id, i32::try_from(value).unwrap()),
        "int64" => Value::new(type_id, i64::try_from(value).unwrap()),
        "int128" => Value::new(type_id, value),
        "bigint" => Value::new(type_id, BigInt::from(value)),
        _ => panic!("unsupported signed integer test type `{type_name}`"),
    }
}

/// Extracts any signed fixed-width integer payload into its exact `i128` value.
fn signed_integer_payload(value: &Value) -> i128 {
    if let Some(integer) = value.downcast_ref::<i8>() {
        return i128::from(*integer);
    }
    if let Some(integer) = value.downcast_ref::<i16>() {
        return i128::from(*integer);
    }
    if let Some(integer) = value.downcast_ref::<i32>() {
        return i128::from(*integer);
    }
    if let Some(integer) = value.downcast_ref::<i64>() {
        return i128::from(*integer);
    }
    if let Some(integer) = value.downcast_ref::<BigInt>() {
        return i128::try_from(integer).unwrap();
    }
    *value.downcast_ref::<i128>().unwrap()
}

/// Calculates the expected result for the small operands used by the pair matrix.
fn expected_operation_value(operator: BinaryOperator, left: i128, right: i128) -> i128 {
    match operator {
        BinaryOperator::Addition => left + right,
        BinaryOperator::Subtraction => left - right,
        BinaryOperator::Multiplication => left * right,
        BinaryOperator::Division => left / right,
        BinaryOperator::Remainder => left % right,
        BinaryOperator::Power => left.pow(u32::try_from(right).unwrap()),
    }
}

/// Calculates the expected comparison result for the pair matrix.
fn expected_comparison_value(operator: ComparisonOperator, left: i128, right: i128) -> bool {
    match operator {
        ComparisonOperator::Equal => left == right,
        ComparisonOperator::NotEqual => left != right,
        ComparisonOperator::Less => left < right,
        ComparisonOperator::LessOrEqual => left <= right,
        ComparisonOperator::Greater => left > right,
        ComparisonOperator::GreaterOrEqual => left >= right,
    }
}

/// Verifies every ordered unequal-width pair and operator promotes to the wider type.
#[test]
fn promotes_every_ordered_signed_integer_pair_for_arithmetic() {
    let mut registry = Registry::new();
    crate::register_all(&mut registry).unwrap();

    for narrower_index in 0..SIGNED_INTEGER_TYPES.len() {
        for wider_index in (narrower_index + 1)..SIGNED_INTEGER_TYPES.len() {
            let narrower_name = SIGNED_INTEGER_TYPES[narrower_index];
            let wider_name = SIGNED_INTEGER_TYPES[wider_index];
            let wider_id = registry.type_by_name(wider_name).unwrap();

            for (left_name, right_name, left_value, right_value) in [
                (narrower_name, wider_name, 2_i128, 3_i128),
                (wider_name, narrower_name, 3_i128, 2_i128),
            ] {
                let left = signed_integer_value(&registry, left_name, left_value);
                let right = signed_integer_value(&registry, right_name, right_value);

                for operator in BINARY_OPERATORS {
                    let operator_id = registry
                        .resolve_binary_operator(operator, left.type_id(), right.type_id())
                        .unwrap();
                    let descriptor = registry.operator(operator_id).unwrap();
                    let result = (descriptor.execute)(&left, &right).unwrap();

                    assert_eq!(descriptor.result_type, wider_id);
                    assert_eq!(result.type_id(), wider_id);
                    assert_eq!(
                        signed_integer_payload(&result),
                        expected_operation_value(operator, left_value, right_value)
                    );
                }
            }
        }
    }
}

/// Verifies every mixed comparison keeps exact values and original operand order.
#[test]
fn compares_every_ordered_signed_integer_pair_exactly() {
    let mut registry = Registry::new();
    crate::register_all(&mut registry).unwrap();

    for narrower_index in 0..SIGNED_INTEGER_TYPES.len() {
        for wider_index in (narrower_index + 1)..SIGNED_INTEGER_TYPES.len() {
            let narrower_name = SIGNED_INTEGER_TYPES[narrower_index];
            let wider_name = SIGNED_INTEGER_TYPES[wider_index];

            for (left_name, right_name, left_value, right_value) in [
                (narrower_name, wider_name, -128_i128, 32_767_i128),
                (wider_name, narrower_name, 32_767_i128, -128_i128),
            ] {
                let left = signed_integer_value(&registry, left_name, left_value);
                let right = signed_integer_value(&registry, right_name, right_value);

                for operator in COMPARISON_OPERATORS {
                    let comparison_id = registry
                        .resolve_comparison(operator, left.type_id(), right.type_id())
                        .unwrap();
                    let descriptor = registry.comparison(comparison_id).unwrap();
                    assert_eq!(
                        (descriptor.execute)(&left, &right).unwrap(),
                        expected_comparison_value(operator, left_value, right_value)
                    );
                }
            }
        }
    }
}

/// Verifies mixed arithmetic retains fixed-width overflow and zero-divisor errors.
#[test]
fn mixed_arithmetic_preserves_runtime_errors() {
    let mut registry = Registry::new();
    crate::register_all(&mut registry).unwrap();
    let integer8_id = registry.type_by_name("int8").unwrap();

    for (wider_name, maximum) in [
        ("int16", i128::from(i16::MAX)),
        ("int32", i128::from(i32::MAX)),
        ("int64", i128::from(i64::MAX)),
        ("int128", i128::MAX),
    ] {
        let maximum = signed_integer_value(&registry, wider_name, maximum);
        let one = Value::new(integer8_id, 1_i8);
        let addition = registry
            .resolve_binary_operator(BinaryOperator::Addition, maximum.type_id(), integer8_id)
            .unwrap();
        assert!(matches!(
            (registry.operator(addition).unwrap().execute)(&maximum, &one),
            Err(CoreError::Runtime(message)) if message.contains("overflow")
        ));

        let zero = Value::new(integer8_id, 0_i8);
        let division = registry
            .resolve_binary_operator(BinaryOperator::Division, maximum.type_id(), integer8_id)
            .unwrap();
        assert!(matches!(
            (registry.operator(division).unwrap().execute)(&maximum, &zero),
            Err(CoreError::DivisionByZero)
        ));
    }
}

/// Verifies mixed executors reject payloads outside the registered integer representations.
#[test]
fn mixed_operations_and_comparisons_reject_invalid_representations() {
    let mut registry = Registry::new();
    crate::register_all(&mut registry).unwrap();
    let integer8_id = registry.type_by_name("int8").unwrap();
    let integer16_id = registry.type_by_name("int16").unwrap();
    let invalid = Value::new(integer8_id, false);
    let wide = Value::new(integer16_id, 1_i16);

    let addition = registry
        .resolve_binary_operator(BinaryOperator::Addition, integer8_id, integer16_id)
        .unwrap();
    assert!(matches!(
        (registry.operator(addition).unwrap().execute)(&invalid, &wide),
        Err(CoreError::InvalidValueRepresentation(_))
    ));

    let equality = registry
        .resolve_comparison(ComparisonOperator::Equal, integer8_id, integer16_id)
        .unwrap();
    assert!(matches!(
        (registry.comparison(equality).unwrap().execute)(&invalid, &wide),
        Err(CoreError::InvalidValueRepresentation(_))
    ));
}

/// Verifies named mixed comparison declarations activate in any type registration order.
#[test]
fn mixed_comparison_declarations_are_registration_order_independent() {
    let mut registry = Registry::new();
    integer16::comparisons::register_promotions(&mut registry).unwrap();
    integer32::comparisons::register_promotions(&mut registry).unwrap();
    integer64::comparisons::register_promotions(&mut registry).unwrap();
    integer128::comparisons::register_promotions(&mut registry).unwrap();
    bigint::comparisons::register_promotions(&mut registry).unwrap();

    Integer128Extension::new().register(&mut registry).unwrap();
    BigintExtension::new().register(&mut registry).unwrap();
    Integer8Extension::new().register(&mut registry).unwrap();
    Integer64Extension::new().register(&mut registry).unwrap();
    Integer16Extension::new().register(&mut registry).unwrap();
    Integer32Extension::new().register(&mut registry).unwrap();

    for narrower_index in 0..SIGNED_INTEGER_TYPES.len() {
        for wider_index in (narrower_index + 1)..SIGNED_INTEGER_TYPES.len() {
            let narrower_id = registry
                .type_by_name(SIGNED_INTEGER_TYPES[narrower_index])
                .unwrap();
            let wider_id = registry
                .type_by_name(SIGNED_INTEGER_TYPES[wider_index])
                .unwrap();
            for operator in COMPARISON_OPERATORS {
                assert!(
                    registry
                        .resolve_comparison(operator, narrower_id, wider_id)
                        .is_ok()
                );
                assert!(
                    registry
                        .resolve_comparison(operator, wider_id, narrower_id)
                        .is_ok()
                );
            }
        }
    }
}

/// Verifies promotion setup reports duplicate signatures instead of silently replacing them.
#[test]
fn signed_promotion_registration_rejects_a_second_registration() {
    let mut registry = Registry::new();
    IntegerExtension::new().register(&mut registry).unwrap();
    Integer8Extension::new().register(&mut registry).unwrap();
    Integer16Extension::new().register(&mut registry).unwrap();
    Integer32Extension::new().register(&mut registry).unwrap();
    Integer128Extension::new().register(&mut registry).unwrap();
    BigintExtension::new().register(&mut registry).unwrap();
    register_signed_promotions(&mut registry).unwrap();

    assert!(matches!(
        register_signed_promotions(&mut registry),
        Err(CoreError::DuplicateOperator { .. })
    ));
}

/// Verifies promotion excludes unsigned integer types while `bigint` promotes every signed width.
#[test]
fn leaves_unsigned_integers_outside_signed_promotion() {
    let mut registry = Registry::new();
    crate::register_all(&mut registry).unwrap();

    for signed_name in SIGNED_INTEGER_TYPES {
        let signed_id = registry.type_by_name(signed_name).unwrap();
        for excluded_name in ["uint8", "uint64"] {
            let excluded_id = registry.type_by_name(excluded_name).unwrap();
            for operator in BINARY_OPERATORS {
                assert!(
                    registry
                        .resolve_binary_operator(operator, signed_id, excluded_id)
                        .is_err()
                );
                assert!(
                    registry
                        .resolve_binary_operator(operator, excluded_id, signed_id)
                        .is_err()
                );
            }
        }
    }

    let bigint_id = registry.type_by_name("bigint").unwrap();
    for signed_name in ["int8", "int16", "int32", "int64", "int128"] {
        let signed_id = registry.type_by_name(signed_name).unwrap();
        for operator in BINARY_OPERATORS {
            for (left_id, right_id) in [(signed_id, bigint_id), (bigint_id, signed_id)] {
                let operator_id = registry
                    .resolve_binary_operator(operator, left_id, right_id)
                    .unwrap();
                assert_eq!(
                    registry.operator(operator_id).unwrap().result_type,
                    bigint_id
                );
            }
        }
    }

    assert_eq!(
        registry.default_integer().unwrap(),
        registry.type_by_name("int64").unwrap()
    );
    assert_eq!(registry.type_by_name("int"), registry.type_by_name("int64"));
}
