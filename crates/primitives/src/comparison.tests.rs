use super::*;
use language_core::{Registry, TypeDescriptor, Value};

/// Registers a minimal placeholder type that satisfies descriptor requirements.
fn register_test_type(registry: &mut Registry, name: &'static str) {
    let id = registry.allocate_type_id();
    registry
        .register_type(TypeDescriptor {
            id,
            name,
            is_integer: false,
            parse_numeric_literal: None,
            parse_string_literal: None,
            parse_regex_literal: None,
            parse_boolean_literal: None,
            parse_null_literal: None,
            format: format_value,
        })
        .unwrap();
}

/// Supplies the inert formatter required by the placeholder descriptors.
fn format_value(_: &Value) -> Result<String, CoreError> {
    Ok(String::new())
}

/// A comparison executor that always reports equality.
fn executor_returns_true(_: &Value, _: &Value) -> Result<bool, CoreError> {
    Ok(true)
}

/// A comparison executor that always reports inequality.
fn executor_returns_false(_: &Value, _: &Value) -> Result<bool, CoreError> {
    Ok(false)
}

/// Verifies `declare_pair` wires each operator position to its executor.
#[test]
fn declares_every_operator_with_its_executor() {
    let mut registry = Registry::new();
    register_test_type(&mut registry, "left");
    register_test_type(&mut registry, "right");
    let left = registry.type_by_name("left").unwrap();
    let right = registry.type_by_name("right").unwrap();

    let executors: [ComparisonExecutor; 6] = [
        executor_returns_true,
        executor_returns_false,
        executor_returns_true,
        executor_returns_false,
        executor_returns_true,
        executor_returns_false,
    ];
    declare_pair(&mut registry, "left", "right", executors).unwrap();

    for (operator, expected) in OPERATORS
        .into_iter()
        .zip([true, false, true, false, true, false])
    {
        let comparison_id = registry.resolve_comparison(operator, left, right).unwrap();
        let descriptor = registry.comparison(comparison_id).unwrap();
        assert_eq!(
            (descriptor.execute)(&Value::new(left, 0_i64), &Value::new(right, 1_i64)).unwrap(),
            expected
        );
    }
}

/// Verifies `declare_pair` declares only the requested operand order.
#[test]
fn declares_only_the_requested_operand_order() {
    let mut registry = Registry::new();
    register_test_type(&mut registry, "left");
    register_test_type(&mut registry, "right");
    let left = registry.type_by_name("left").unwrap();
    let right = registry.type_by_name("right").unwrap();

    let executors: [ComparisonExecutor; 6] = [executor_returns_true; 6];
    declare_pair(&mut registry, "left", "right", executors).unwrap();

    assert!(
        registry
            .resolve_comparison(ComparisonOperator::Equal, left, right)
            .is_ok()
    );
    assert!(
        registry
            .resolve_comparison(ComparisonOperator::Equal, right, left)
            .is_err()
    );
}

/// Verifies declaring the same pair twice reports the duplicate signature.
#[test]
fn rejects_a_duplicate_pair_declaration() {
    let mut registry = Registry::new();
    register_test_type(&mut registry, "left");
    register_test_type(&mut registry, "right");

    let executors: [ComparisonExecutor; 6] = [executor_returns_true; 6];
    declare_pair(&mut registry, "left", "right", executors).unwrap();

    assert!(matches!(
        declare_pair(&mut registry, "left", "right", executors),
        Err(CoreError::DuplicateComparison { .. })
    ));
}

/// Verifies total-ordering evaluation for every operator and ordering.
#[test]
fn evaluates_every_operator_from_a_total_order() {
    for (ordering, expected) in [
        (Ordering::Less, [false, true, true, true, false, false]),
        (Ordering::Equal, [true, false, false, true, false, true]),
        (Ordering::Greater, [false, true, false, false, true, true]),
    ] {
        for (operator, expected_value) in OPERATORS.into_iter().zip(expected) {
            assert_eq!(evaluate_total_order(ordering, operator), expected_value);
        }
    }
}

/// Verifies an unordered partial comparison reports only inequality.
#[test]
fn evaluates_every_operator_from_an_unordered_partial_order() {
    for (operator, expected_value) in OPERATORS
        .into_iter()
        .zip([false, true, false, false, false, false])
    {
        assert_eq!(evaluate_partial_order(None, operator), expected_value);
    }
}

/// Verifies partial-ordering evaluation matches total-ordering evaluation.
#[test]
fn evaluates_partial_orders_consistently_with_total_orders() {
    for ordering in [Ordering::Less, Ordering::Equal, Ordering::Greater] {
        for operator in OPERATORS {
            assert_eq!(
                evaluate_partial_order(Some(ordering), operator),
                evaluate_total_order(ordering, operator)
            );
        }
    }
}
