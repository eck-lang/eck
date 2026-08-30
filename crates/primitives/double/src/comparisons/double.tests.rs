use super::*;

/// Verifies the native ordering of finite double values.
#[test]
fn compares_finite_double_values() {
    let smaller = Value::new(crate::test_type_id(), -1.25_f64);
    let larger = Value::new(crate::test_type_id(), 2.5_f64);

    assert!(less(&smaller, &larger).unwrap());
    assert!(less_or_equal(&smaller, &larger).unwrap());
    assert!(greater(&larger, &smaller).unwrap());
    assert!(greater_or_equal(&larger, &smaller).unwrap());
    assert!(not_equal(&smaller, &larger).unwrap());
}

/// Verifies IEEE-754 equality for signed zero and unordered NaN semantics.
#[test]
fn preserves_double_special_value_semantics() {
    let positive_zero = Value::new(crate::test_type_id(), 0.0_f64);
    let negative_zero = Value::new(crate::test_type_id(), -0.0_f64);
    let nan = Value::new(crate::test_type_id(), f64::NAN);

    assert!(equal(&positive_zero, &negative_zero).unwrap());
    assert!(less_or_equal(&positive_zero, &negative_zero).unwrap());
    assert!(!equal(&positive_zero, &nan).unwrap());
    assert!(not_equal(&positive_zero, &nan).unwrap());
    assert!(!less(&positive_zero, &nan).unwrap());
    assert!(!greater_or_equal(&positive_zero, &nan).unwrap());
}
