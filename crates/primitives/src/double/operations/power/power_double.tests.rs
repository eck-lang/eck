use super::*;

/// Verifies exponentiation of two double values.
#[test]
fn raises_double_values_to_a_power() {
    let base = Value::new(crate::double::test_type_id(), 1.5_f64);
    let exponent = Value::new(crate::double::test_type_id(), 2.0_f64);

    let result = power_double(&base, &exponent).unwrap();

    assert_eq!(*result.downcast_ref::<f64>().unwrap(), 2.25);
}
