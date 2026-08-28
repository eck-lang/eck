use language_core::{CoreError, Value};

use crate::value::get;

/// Subtracts the right floating-point value from the left value.
pub(crate) fn subtraction_float(lhs: &Value, rhs: &Value) -> Result<Value, CoreError> {
    Ok(Value::new(lhs.type_id(), get(lhs)? - get(rhs)?))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn subtracts_float_values() {
        let lhs = Value::new(crate::test_type_id(), 5.5_f32);
        let rhs = Value::new(crate::test_type_id(), 2.25_f32);

        let result = subtraction_float(&lhs, &rhs).unwrap();

        assert_eq!(*result.downcast_ref::<f32>().unwrap(), 3.25);
    }
}
