use language_core::{CoreError, Value};

use crate::operations::float_double_operands;

/// Raises a float/double pair to a power after promoting the float to double precision.
pub(crate) fn power_float_double(lhs: &Value, rhs: &Value) -> Result<Value, CoreError> {
    let (lhs, rhs, double_id) = float_double_operands(lhs, rhs)?;
    Ok(Value::new(double_id, lhs.powf(rhs)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn raises_float_and_double_values_to_a_power_in_both_orders() {
        let float = Value::new(crate::test_type_id(), 2.0_f32);
        let double = Value::new(crate::test_type_id(), 3.0_f64);

        let float_left = power_float_double(&float, &double).unwrap();
        let double_left = power_float_double(&double, &float).unwrap();

        assert_eq!(*float_left.downcast_ref::<f64>().unwrap(), 8.0);
        assert_eq!(*double_left.downcast_ref::<f64>().unwrap(), 9.0);
    }
}
