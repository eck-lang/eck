use super::*;

/// Verifies exponentiation of two float values.
#[test]
fn raises_float_values_to_a_power() {
    let base = Value::new(crate::test_type_id(), 1.5_f32);
    let exponent = Value::new(crate::test_type_id(), 2.0_f32);

    let result = power_float(&base, &exponent).unwrap();

    assert_eq!(*result.downcast_ref::<f32>().unwrap(), 2.25);
}
