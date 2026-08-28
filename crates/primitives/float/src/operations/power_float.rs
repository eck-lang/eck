use language_core::{CoreError, Value};

use crate::value::get;

/// Raises a floating-point base to a floating-point exponent.
pub(crate) fn power_float(lhs: &Value, rhs: &Value) -> Result<Value, CoreError> {
    Ok(Value::new(lhs.type_id(), get(lhs)?.powf(get(rhs)?)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn raises_float_values_to_a_power() {
        let base = Value::new(crate::test_type_id(), 1.5_f32);
        let exponent = Value::new(crate::test_type_id(), 2.0_f32);

        let result = power_float(&base, &exponent).unwrap();

        assert_eq!(*result.downcast_ref::<f32>().unwrap(), 2.25);
    }
}
