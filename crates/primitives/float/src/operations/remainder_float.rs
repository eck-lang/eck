use language_core::{CoreError, Value};

use crate::value::get;

/// Calculates the remainder of the left floating-point value divided by the right one.
pub(crate) fn remainder_float(lhs: &Value, rhs: &Value) -> Result<Value, CoreError> {
    let rhs = get(rhs)?;
    if rhs == 0.0 {
        return Err(CoreError::DivisionByZero);
    }
    Ok(Value::new(lhs.type_id(), get(lhs)? % rhs))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn calculates_float_remainder_and_rejects_zero() {
        let lhs = Value::new(crate::test_type_id(), 10.5_f32);
        let rhs = Value::new(crate::test_type_id(), 4.0_f32);
        let zero = Value::new(crate::test_type_id(), 0.0_f32);

        let result = remainder_float(&lhs, &rhs).unwrap();

        assert_eq!(*result.downcast_ref::<f32>().unwrap(), 2.5);
        assert!(matches!(
            remainder_float(&lhs, &zero),
            Err(CoreError::DivisionByZero)
        ));
    }
}
