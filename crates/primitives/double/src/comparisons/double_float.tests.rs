use super::*;

/// Verifies equality between exactly promoted float and double values.
#[test]
fn compares_double_and_float_in_both_operand_orders() {
    let double = Value::new(crate::test_type_id(), 2.5_f64);
    let float = Value::new(crate::test_type_id(), 2.5_f32);

    assert!(equal(&double, &float).unwrap());
    assert!(equal(&float, &double).unwrap());
}

/// Verifies that double precision is retained after exact float promotion.
#[test]
fn preserves_double_precision_when_comparing_with_float() {
    let double = Value::new(crate::test_type_id(), f64::from(1.0_f32).next_up());
    let float = Value::new(crate::test_type_id(), 1.0_f32);

    assert!(greater(&double, &float).unwrap());
    assert!(less(&float, &double).unwrap());
    assert!(!equal(&double, &float).unwrap());
}

/// Verifies that float NaN and infinity retain IEEE-754 comparison behavior.
#[test]
fn preserves_float_special_value_semantics() {
    let double = Value::new(crate::test_type_id(), 1.0_f64);
    let nan = Value::new(crate::test_type_id(), f32::NAN);
    let positive_infinity = Value::new(crate::test_type_id(), f32::INFINITY);

    assert!(!equal(&double, &nan).unwrap());
    assert!(not_equal(&double, &nan).unwrap());
    assert!(!less(&double, &nan).unwrap());
    assert!(!greater(&nan, &double).unwrap());
    assert!(less(&double, &positive_infinity).unwrap());
}
