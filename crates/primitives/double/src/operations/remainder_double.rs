use language_core::{CoreError, Value};

use crate::value::get;

/// Calculates the remainder of the left double-precision value divided by the right one.
pub(crate) fn remainder_double(lhs: &Value, rhs: &Value) -> Result<Value, CoreError> {
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
    fn calculates_double_remainder_and_rejects_zero() {
        let lhs = Value::new(crate::test_type_id(), 10.5_f64);
        let rhs = Value::new(crate::test_type_id(), 4.0_f64);
        let zero = Value::new(crate::test_type_id(), 0.0_f64);

        let result = remainder_double(&lhs, &rhs).unwrap();

        assert_eq!(*result.downcast_ref::<f64>().unwrap(), 2.5);
        assert!(matches!(
            remainder_double(&lhs, &zero),
            Err(CoreError::DivisionByZero)
        ));
    }
}
