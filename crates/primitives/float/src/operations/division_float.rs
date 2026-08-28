use language_core::{CoreError, Value};

use crate::value::get;

/// Divides the left floating-point value by the right one.
pub(crate) fn division_float(lhs: &Value, rhs: &Value) -> Result<Value, CoreError> {
    let rhs = get(rhs)?;
    if rhs == 0.0 {
        return Err(CoreError::DivisionByZero);
    }
    Ok(Value::new(lhs.type_id(), get(lhs)? / rhs))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn divides_float_values_and_rejects_zero() {
        let lhs = Value::new(crate::test_type_id(), 9.0_f32);
        let rhs = Value::new(crate::test_type_id(), 4.0_f32);
        let zero = Value::new(crate::test_type_id(), 0.0_f32);

        let result = division_float(&lhs, &rhs).unwrap();

        assert_eq!(*result.downcast_ref::<f32>().unwrap(), 2.25);
        assert!(matches!(
            division_float(&lhs, &zero),
            Err(CoreError::DivisionByZero)
        ));
    }
}
