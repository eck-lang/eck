use super::*;

/// Verifies ordinary float-integer ordering in both operand directions.
#[test]
fn compares_float_and_integer_in_both_operand_orders() {
    let float = Value::new(crate::test_type_id(), 2.5_f32);
    let integer = Value::new(crate::test_type_id(), 2_i64);

    assert!(greater(&float, &integer).unwrap());
    assert!(less(&integer, &float).unwrap());
}

/// Verifies exact ordering beyond the integer precision boundary of a float.
#[test]
fn preserves_integer_precision_beyond_two_to_the_power_of_twenty_four() {
    let rounded_float = Value::new(crate::test_type_id(), 16_777_216.0_f32);
    let exact_integer = Value::new(crate::test_type_id(), 16_777_217_i64);

    assert!(less(&rounded_float, &exact_integer).unwrap());
    assert!(greater(&exact_integer, &rounded_float).unwrap());
    assert!(!equal(&rounded_float, &exact_integer).unwrap());
}

/// Verifies exact comparisons at both signed 64-bit integer boundaries.
#[test]
fn compares_signed_integer_boundaries_without_rounding() {
    let positive_boundary_float = Value::new(crate::test_type_id(), 2_f32.powi(63));
    let maximum_integer = Value::new(crate::test_type_id(), i64::MAX);
    let negative_boundary_float = Value::new(crate::test_type_id(), -2_f32.powi(63));
    let minimum_integer = Value::new(crate::test_type_id(), i64::MIN);

    assert!(less(&maximum_integer, &positive_boundary_float).unwrap());
    assert!(equal(&minimum_integer, &negative_boundary_float).unwrap());
}

/// Verifies fractional, subnormal, infinite, and NaN float behavior.
#[test]
fn preserves_special_and_fractional_float_semantics() {
    let zero = Value::new(crate::test_type_id(), 0_i64);
    let negative_one = Value::new(crate::test_type_id(), -1_i64);
    let smallest_positive_float = Value::new(crate::test_type_id(), f32::from_bits(1));
    let negative_fraction = Value::new(crate::test_type_id(), -0.5_f32);
    let positive_infinity = Value::new(crate::test_type_id(), f32::INFINITY);
    let nan = Value::new(crate::test_type_id(), f32::NAN);

    assert!(less(&zero, &smallest_positive_float).unwrap());
    assert!(less(&negative_one, &negative_fraction).unwrap());
    assert!(less(&zero, &positive_infinity).unwrap());
    assert!(!equal(&zero, &nan).unwrap());
    assert!(not_equal(&zero, &nan).unwrap());
    assert!(!less(&zero, &nan).unwrap());
    assert!(!greater_or_equal(&nan, &zero).unwrap());
}

/// Verifies the exact helper around adjacent integral float boundaries.
#[test]
fn compares_binary_decompositions_exactly() {
    assert_eq!(
        compare_integer_with_float(1, 1.0_f32),
        Some(Ordering::Equal)
    );
    assert_eq!(
        compare_integer_with_float(1, 1.0_f32.next_down()),
        Some(Ordering::Greater)
    );
    assert_eq!(
        compare_integer_with_float(1, 1.0_f32.next_up()),
        Some(Ordering::Less)
    );
    assert_eq!(
        compare_integer_with_float(0, -0.0_f32),
        Some(Ordering::Equal)
    );
}
