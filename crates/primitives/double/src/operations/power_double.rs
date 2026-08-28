use language_core::{CoreError, Value};

use crate::value::get;

/// Raises a double-precision floating-point base to a double-precision exponent.
pub(crate) fn power_double(lhs: &Value, rhs: &Value) -> Result<Value, CoreError> {
    Ok(Value::new(lhs.type_id(), get(lhs)?.powf(get(rhs)?)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn raises_double_values_to_a_power() {
        let base = Value::new(crate::test_type_id(), 1.5_f64);
        let exponent = Value::new(crate::test_type_id(), 2.0_f64);

        let result = power_double(&base, &exponent).unwrap();

        assert_eq!(*result.downcast_ref::<f64>().unwrap(), 2.25);
    }
}
