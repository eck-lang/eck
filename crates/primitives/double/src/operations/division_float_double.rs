use language_core::{CoreError, Value};

use crate::operations::float_double_operands;

/// Divides a float and a double after promoting the float to double precision.
pub(crate) fn division_float_double(lhs: &Value, rhs: &Value) -> Result<Value, CoreError> {
    let (lhs, rhs, double_id) = float_double_operands(lhs, rhs)?;
    if rhs == 0.0 {
        return Err(CoreError::DivisionByZero);
    }
    Ok(Value::new(double_id, lhs / rhs))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn divides_float_and_double_in_both_orders_and_rejects_zero() {
        let float = Value::new(crate::test_type_id(), 5.0_f32);
        let double = Value::new(crate::test_type_id(), 2.0_f64);
        let zero = Value::new(crate::test_type_id(), 0.0_f64);

        let float_left = division_float_double(&float, &double).unwrap();
        let double_left = division_float_double(&double, &float).unwrap();

        assert_eq!(*float_left.downcast_ref::<f64>().unwrap(), 2.5);
        assert_eq!(*double_left.downcast_ref::<f64>().unwrap(), 0.4);
        assert!(matches!(
            division_float_double(&float, &zero),
            Err(CoreError::DivisionByZero)
        ));
    }
}
