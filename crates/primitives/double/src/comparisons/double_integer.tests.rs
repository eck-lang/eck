use super::*;

/// Verifies ordinary double-integer ordering in both operand directions.
#[test]
fn compares_double_and_integer_in_both_operand_orders() {
    let double = Value::new(crate::test_type_id(), 2.5_f64);
    let integer = Value::new(crate::test_type_id(), 2_i64);

    assert!(greater(&double, &integer).unwrap());
    assert!(less(&integer, &double).unwrap());
}

/// Verifies exact ordering beyond the integer precision boundary of a double.
#[test]
fn preserves_integer_precision_beyond_two_to_the_power_of_fifty_three() {
    let rounded_double = Value::new(crate::test_type_id(), 9_007_199_254_740_992.0_f64);
    let exact_integer = Value::new(crate::test_type_id(), 9_007_199_254_740_993_i64);

    assert!(less(&rounded_double, &exact_integer).unwrap());
    assert!(greater(&exact_integer, &rounded_double).unwrap());
    assert!(!equal(&rounded_double, &exact_integer).unwrap());
}

/// Verifies exact comparisons at both signed 64-bit integer boundaries.
#[test]
fn compares_signed_integer_boundaries_without_rounding() {
    let positive_boundary_double = Value::new(crate::test_type_id(), 2_f64.powi(63));
    let maximum_integer = Value::new(crate::test_type_id(), i64::MAX);
    let negative_boundary_double = Value::new(crate::test_type_id(), -2_f64.powi(63));
    let minimum_integer = Value::new(crate::test_type_id(), i64::MIN);

    assert!(less(&maximum_integer, &positive_boundary_double).unwrap());
    assert!(equal(&minimum_integer, &negative_boundary_double).unwrap());
}

/// Verifies fractional, subnormal, infinite, and NaN double behavior.
#[test]
fn preserves_special_and_fractional_double_semantics() {
    let zero = Value::new(crate::test_type_id(), 0_i64);
    let negative_one = Value::new(crate::test_type_id(), -1_i64);
    let smallest_positive_double = Value::new(crate::test_type_id(), f64::from_bits(1));
    let negative_fraction = Value::new(crate::test_type_id(), -0.5_f64);
    let positive_infinity = Value::new(crate::test_type_id(), f64::INFINITY);
    let nan = Value::new(crate::test_type_id(), f64::NAN);

    assert!(less(&zero, &smallest_positive_double).unwrap());
    assert!(less(&negative_one, &negative_fraction).unwrap());
    assert!(less(&zero, &positive_infinity).unwrap());
    assert!(!equal(&zero, &nan).unwrap());
    assert!(not_equal(&zero, &nan).unwrap());
    assert!(!less(&zero, &nan).unwrap());
    assert!(!greater_or_equal(&nan, &zero).unwrap());
}

/// Verifies the exact helper against values around integral double boundaries.
#[test]
fn compares_binary_decompositions_exactly() {
    assert_eq!(
        compare_integer_with_double(1, 1.0_f64),
        Some(Ordering::Equal)
    );
    assert_eq!(
        compare_integer_with_double(1, 1.0_f64.next_down()),
        Some(Ordering::Greater)
    );
    assert_eq!(
        compare_integer_with_double(1, 1.0_f64.next_up()),
        Some(Ordering::Less)
    );
    assert_eq!(
        compare_integer_with_double(0, -0.0_f64),
        Some(Ordering::Equal)
    );
}
