use language_core::{CoreError, Value};

use crate::value::get;

/// Adds two double-precision floating-point values.
pub(crate) fn addition_double(lhs: &Value, rhs: &Value) -> Result<Value, CoreError> {
    Ok(Value::new(lhs.type_id(), get(lhs)? + get(rhs)?))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn adds_double_values() {
        let lhs = Value::new(crate::test_type_id(), 1.5_f64);
        let rhs = Value::new(crate::test_type_id(), 2.25_f64);

        let result = addition_double(&lhs, &rhs).unwrap();

        assert_eq!(*result.downcast_ref::<f64>().unwrap(), 3.75);
    }
}
