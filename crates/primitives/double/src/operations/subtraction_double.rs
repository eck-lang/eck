use language_core::{CoreError, Value};

use crate::value::get;

/// Subtracts the right double-precision value from the left value.
pub(crate) fn subtraction_double(lhs: &Value, rhs: &Value) -> Result<Value, CoreError> {
    Ok(Value::new(lhs.type_id(), get(lhs)? - get(rhs)?))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn subtracts_double_values() {
        let lhs = Value::new(crate::test_type_id(), 5.5_f64);
        let rhs = Value::new(crate::test_type_id(), 2.25_f64);

        let result = subtraction_double(&lhs, &rhs).unwrap();

        assert_eq!(*result.downcast_ref::<f64>().unwrap(), 3.25);
    }
}
