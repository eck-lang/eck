use language_core::{CoreError, Value};

use crate::operations::float_double_operands;

/// Calculates a float/double remainder after promoting the float to double precision.
pub(crate) fn remainder_float_double(lhs: &Value, rhs: &Value) -> Result<Value, CoreError> {
    let (lhs, rhs, double_id) = float_double_operands(lhs, rhs)?;
    if rhs == 0.0 {
        return Err(CoreError::DivisionByZero);
    }
    Ok(Value::new(double_id, lhs % rhs))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn calculates_float_double_remainder_in_both_orders_and_rejects_zero() {
        let float = Value::new(crate::test_type_id(), 10.5_f32);
        let double = Value::new(crate::test_type_id(), 2.0_f64);
        let zero = Value::new(crate::test_type_id(), 0.0_f64);

        let float_left = remainder_float_double(&float, &double).unwrap();
        let double_left = remainder_float_double(&double, &float).unwrap();

        assert_eq!(*float_left.downcast_ref::<f64>().unwrap(), 0.5);
        assert_eq!(*double_left.downcast_ref::<f64>().unwrap(), 2.0);
        assert!(matches!(
            remainder_float_double(&float, &zero),
            Err(CoreError::DivisionByZero)
        ));
    }
}
