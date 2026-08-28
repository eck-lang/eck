use language_core::{CoreError, Value};

use crate::value::get;

/// Multiplies two floating-point values.
pub(crate) fn multiplication_float(lhs: &Value, rhs: &Value) -> Result<Value, CoreError> {
    Ok(Value::new(lhs.type_id(), get(lhs)? * get(rhs)?))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn multiplies_float_values() {
        let lhs = Value::new(crate::test_type_id(), 1.5_f32);
        let rhs = Value::new(crate::test_type_id(), 4.0_f32);

        let result = multiplication_float(&lhs, &rhs).unwrap();

        assert_eq!(*result.downcast_ref::<f32>().unwrap(), 6.0);
    }
}
