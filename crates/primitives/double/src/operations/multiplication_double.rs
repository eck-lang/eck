use language_core::{CoreError, Value};

use crate::value::get;

/// Multiplies two double-precision floating-point values.
pub(crate) fn multiplication_double(lhs: &Value, rhs: &Value) -> Result<Value, CoreError> {
    Ok(Value::new(lhs.type_id(), get(lhs)? * get(rhs)?))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn multiplies_double_values() {
        let lhs = Value::new(crate::test_type_id(), 1.5_f64);
        let rhs = Value::new(crate::test_type_id(), 4.0_f64);

        let result = multiplication_double(&lhs, &rhs).unwrap();

        assert_eq!(*result.downcast_ref::<f64>().unwrap(), 6.0);
    }
}
